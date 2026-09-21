#!/usr/bin/env python3
"""核验经 SSH 提交的公司签名原包，再与同提交的 Windows 发布附件合并。"""
import hashlib
import json
from pathlib import Path, PurePosixPath
import plistlib
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_hashes():
    # BUILD_NUMBER 在 CI 和本地签名构建时按同一发布元数据写入，另由 plist 核验。
    names = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT).decode().split('\0')
    return {name: digest(ROOT / name) for name in names if name and name != 'BUILD_NUMBER'}


def check_metadata(info, published, sources):
    for key in ('commit', 'tag', 'build', 'version'):
        if info[key] != published[key]:
            raise ValueError('macOS 与已发布 Windows 来源不一致：' + key)
    if info['platform'] != 'macos-arm64' or info['signing'] != 'company-signed':
        raise ValueError('必须提供公司签名的 macOS arm64 包')
    if info['source_hashes'] != sources:
        raise ValueError('构建输入与来源提交不一致')
    expected = f"Codex-Account-Switch-{info['tag']}-macos-arm64.zip"
    if info['file'] != expected:
        raise ValueError('macOS 包名不一致')
    return expected


def verify(payload, assets):
    info = json.loads((payload / 'manifest.json').read_text())
    published = json.loads((assets / 'BUILD_INFO.json').read_text())
    filename = check_metadata(info, published, source_hashes())
    if subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip() != info['commit']:
        raise ValueError('检出的源码不是声明的提交')
    package = payload / filename
    if digest(package) != info['sha256']:
        raise ValueError('macOS ZIP 校验失败')
    with zipfile.ZipFile(package) as archive:
        if archive.testzip() is not None:
            raise ValueError('ZIP 已损坏')
        for entry in archive.infolist():
            path = PurePosixPath(entry.filename)
            if path.is_absolute() or '..' in path.parts or '\\' in entry.filename:
                raise ValueError('ZIP 路径不安全')
            if (entry.external_attr >> 16) & 0o170000 == 0o120000:
                raise ValueError('本应用包不应包含符号链接')
    with tempfile.TemporaryDirectory(prefix='signed-macos-') as temp:
        subprocess.run(['ditto', '-x', '-k', str(package), temp], check=True)
        app = Path(temp) / 'Codex Account Switch.app'
        plist = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
        if (plist['CFBundleShortVersionString'], plist['CFBundleVersion'], plist['CFBundleIdentifier']) != (
                info['version'], info['build'], 'local.gcsa.codex-account-switch'):
            raise ValueError('App 内的版本、构建号或标识不一致')
        binary = app / 'Contents/MacOS/codex-account-switch-bin'
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
        for item in (app, binary):
            signature = subprocess.check_output(['codesign', '-dv', '--verbose=2', str(item)], stderr=subprocess.STDOUT).decode()
            if ('Authority=Apple Distribution: Global Cybersecurity Alliance Limited (R3JK7R29AC)' not in signature
                    or 'TeamIdentifier=R3JK7R29AC' not in signature):
                raise ValueError('公司签名不匹配')
        if subprocess.check_output(['lipo', '-archs', str(binary)]).decode().strip() != 'arm64':
            raise ValueError('macOS 架构不匹配')
    # 先验证已发布包，防止覆盖已有、不同内容的同名公司签名包。
    for item in published['packages']:
        if Path(item['file']).name != item['file'] or digest(assets / item['file']) != item['sha256']:
            raise ValueError('已有发布附件校验失败')
        if item['platform'] == 'macos-arm64' and item['sha256'] != info['sha256']:
            raise ValueError('同版本已存在不同的 macOS 包，拒绝覆盖')
        (assets / Path(item['file']).with_suffix('.json')).write_text(json.dumps(item, indent=2) + '\n')
    shutil.copy2(package, assets / filename)
    (assets / Path(filename).with_suffix('.json')).write_text(json.dumps(info, indent=2) + '\n')
    print('来源、构建输入、ZIP、App 版本及公司签名均通过')


if __name__ == '__main__':
    verify(Path(sys.argv[1]), Path(sys.argv[2]))
