#!/usr/bin/env python3
"""自动发布的版本、Windows 包及跨平台产物核验；只使用 Python 标准库。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import tempfile
import time
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
PLATFORMS = {"windows-x64", "windows-x86", "macos-arm64"}


def version():
    return tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]


def metadata(env):
    # run_id 区分不同运行，attempt 区分完整重跑；包名与标签不复用。
    run = int(env["GITHUB_RUN_ID"])
    attempt = int(env["GITHUB_RUN_ATTEMPT"])
    number = int(env["GITHUB_RUN_NUMBER"])
    if min(run, attempt, number) < 1:
        raise ValueError("运行编号必须为正整数")
    return f"v{version()}-ci.{run}.{attempt}", 1000 + number * 100 + attempt


def build_info(platform, signing):
    sha = os.environ["GITHUB_SHA"]
    tag = os.environ["RELEASE_TAG"]
    build = os.environ["CI_BUILD_NUMBER"]
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ValueError("源码 SHA 不合法")
    if not re.fullmatch(r"v[0-9.]+-ci\.[0-9]+\.[0-9]+", tag):
        raise ValueError("发布标签不合法")
    if not build.isdigit() or int(build) < 1:
        raise ValueError("构建号不合法")
    return dict(platform=platform, signing=signing, version=version(),
                build=build, commit=sha, tag=tag)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_manifest(package, info):
    info = dict(info, file=package.name, sha256=digest(package))
    package.with_suffix(".json").write_text(json.dumps(info, indent=2) + "\n")


def check_pe(binary, machine):
    data = binary.read_bytes()
    if data[:2] != b"MZ" or len(data) < 64:
        raise ValueError("不是 Windows PE 文件")
    offset = struct.unpack_from("<I", data, 0x3C)[0]
    if data[offset:offset + 4] != b"PE\0\0":
        raise ValueError("PE 头缺失")
    if struct.unpack_from("<H", data, offset + 4)[0] != machine:
        raise ValueError("Windows 架构与包名不匹配")


def startup_smoke(binary):
    # 云端测试使用空目录；不导入真实账号、不执行切换。
    with tempfile.TemporaryDirectory(prefix="codex-switch-smoke-") as temp:
        env = dict(os.environ, CODEX_SWITCH_HOME=str(Path(temp) / "switch"),
                   CODEX_HOME=str(Path(temp) / "codex"))
        with (Path(temp) / "startup.log").open("wb") as log:
            process = subprocess.Popen([str(binary.resolve())], env=env, stdout=log, stderr=log)
            try:
                for _ in range(10):
                    if process.poll() is not None:
                        log.flush()
                        details = []
                        for path in (Path(temp) / "startup.log", Path(temp) / "switch/app.log"):
                            if path.exists():
                                details.append(path.read_text(encoding="utf-8", errors="replace")[-12000:])
                        raise RuntimeError(f"启动检查失败：进程提前退出 {process.returncode}\n" + "\n".join(details))
                    time.sleep(1)
                app_log = Path(temp) / "switch/app.log"
                if not app_log.exists() or "ui shown; entering event loop" not in app_log.read_text(encoding="utf-8", errors="replace"):
                    raise RuntimeError("进程存活，但没有界面进入事件循环的证据")
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=15)


def windows_package(target):
    arch, machine = ("x64", 0x8664) if target.startswith("x86_64-") else ("x86", 0x14C)
    binary = ROOT / "target" / target / "release/codex-account-switch.exe"
    check_pe(binary, machine)
    startup_smoke(binary)
    info = build_info(f"windows-{arch}", "unsigned")
    output = ROOT / "dist/ci"
    output.mkdir(parents=True, exist_ok=True)
    package = output / f"Codex-Account-Switch-{info['tag']}-windows-{arch}.zip"
    with zipfile.ZipFile(package, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.write(binary, "codex-account-switch.exe")
        for name in ("LICENSE", "THIRD_PARTY_NOTICES.md"):
            archive.write(ROOT / name, name)
        archive.writestr("BUILD_INFO.json", json.dumps(info, indent=2))
    write_manifest(package, info)
    print(f"Windows {arch} PE、隔离启动和打包检查通过：{package.name}")


def validate_packages(directory, expected):
    manifests = sorted(directory.glob("Codex-Account-Switch-*.json"))
    packages = []
    seen = set()
    for manifest in manifests:
        item = json.loads(manifest.read_text())
        platform = item["platform"]
        if platform not in PLATFORMS or platform in seen:
            raise ValueError("平台缺失、未知或重复")
        seen.add(platform)
        for key in ("commit", "tag", "build", "version"):
            if item[key] != expected[key]:
                raise ValueError(f"{platform} 的 {key} 与本次提交不一致")
        name = item["file"]
        required = f"Codex-Account-Switch-{expected['tag']}-{platform}.zip"
        if name != required or digest(directory / name) != item["sha256"]:
            raise ValueError("包名或 SHA256 不匹配")
        if item["signing"] not in ("unsigned", "company-signed"):
            raise ValueError("签名状态不合法")
        with zipfile.ZipFile(directory / name) as archive:
            if archive.testzip() is not None:
                raise ValueError("ZIP 完整性检查失败")
            names = set(archive.namelist())
            if platform.startswith("windows"):
                if not {"codex-account-switch.exe", "LICENSE", "THIRD_PARTY_NOTICES.md", "BUILD_INFO.json"} <= names:
                    raise ValueError("Windows ZIP 缺少程序、许可或构建信息")
                inside = json.loads(archive.read("BUILD_INFO.json"))
                if any(inside[k] != item[k] for k in expected):
                    raise ValueError("ZIP 内构建信息不一致")
            else:
                prefix = "Codex Account Switch.app/Contents/"
                if not {prefix + "Info.plist", prefix + "MacOS/codex-account-switch-bin",
                        prefix + "Resources/LICENSE", prefix + "Resources/THIRD_PARTY_NOTICES.md"} <= names:
                    raise ValueError("macOS ZIP 结构不完整")
        packages.append(item)
    if not {"windows-x64", "windows-x86"} <= seen:
        raise ValueError("必须同时提供 Windows x64 和 x86")
    if "macos-arm64" not in seen and not (directory / "macos-signing-pending.txt").exists():
        raise ValueError("macOS 产物或签名待配置说明缺失")
    if {p.name for p in directory.glob("*.zip")} != {p["file"] for p in packages}:
        raise ValueError("存在未纳入清单的 ZIP")
    return packages


def assemble(directory):
    info = build_info("", "")
    expected = {k: info[k] for k in ("commit", "tag", "build", "version")}
    packages = validate_packages(directory, expected)
    (directory / "SHA256SUMS.txt").write_text("".join(
        f"{p['sha256']}  {p['file']}\n" for p in packages))
    (directory / "BUILD_INFO.json").write_text(json.dumps(
        dict(expected, packages=packages), indent=2) + "\n")
    signed = next((p["signing"] for p in packages if p["platform"] == "macos-arm64"), "pending")
    mac = {"pending": "macOS 自动包等待公司签名配置；历史公司签名版仍可下载。",
           "unsigned": "macOS 自动包没有公司签名，未公证；与历史公司签名版本分开保留。",
           "company-signed": "macOS 自动包使用 Global Cybersecurity Alliance Limited 公司签名，未公证。"}[signed]
    notes = f"""# {info['tag']} · 自动构建

源代码提交：`{info['commit']}`；应用版本：`{info['version']}`；构建号：`{info['build']}`。

每次推送到 main，测试成功后自动发布此提交的独立测试版。无需手工创建标签。

- Windows x64 / x86：ZIP 内含可执行文件、许可、第三方声明和构建信息；未做代码签名。已通过对应架构测试、Release 构建、PE 校验及空数据目录启动检查；账号切换和托盘交互仍需真机验收。
- {mac}
- 附 SHA256SUMS.txt 和 BUILD_INFO.json；缺少平台产物、版本不一致或校验失败时不公开发布。
- 最新功能与修改见本次源码内的 CHANGELOG.zh-CN.md、CHANGELOG.zh-TW.md 和 CHANGELOG.md。

## 繁體中文

每次推送 main，通過測試後自動發布獨立測試版。提供 Windows x64 / x86 ZIP、校驗和及來源提交。Windows 未簽名，啟動檢查不代表帳戶與系統匣完整驗收。macOS 簽名狀態見上方；未公證。

## English

Each main push creates an independent testing prerelease after checks pass. Windows x64 and x86 ZIPs include the executable, licenses and build metadata. Windows packages are unsigned; isolated startup checks do not replace account-switching and tray acceptance. macOS signing status: `{signed}`; not notarized. See CHANGELOG.md at the source commit for changes.
"""
    (directory / "RELEASE_NOTES.md").write_text(notes)
    print(f"已核验 {len(packages)} 个平台包，来源 {info['commit']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["metadata", "build-number", "windows", "macos-manifest", "assemble"])
    parser.add_argument("--target", choices=["x86_64-pc-windows-msvc", "i686-pc-windows-msvc"])
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--signing", choices=["unsigned", "company-signed"])
    args = parser.parse_args()
    if args.command == "metadata":
        tag, build = metadata(os.environ)
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"tag={tag}\nbuild={build}\n")
    elif args.command == "build-number":
        build = os.environ["CI_BUILD_NUMBER"]
        if not build.isdigit() or int(build) < 1:
            raise ValueError("构建号必须为正整数")
        (ROOT / "BUILD_NUMBER").write_text(build + "\n")
    elif args.command == "windows":
        windows_package(args.target)
    elif args.command == "macos-manifest":
        write_manifest(args.package, build_info("macos-arm64", args.signing))
    else:
        assemble(args.directory)


if __name__ == "__main__":
    main()
