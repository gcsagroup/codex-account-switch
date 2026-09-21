"""验证发布时拒绝错版本、错平台及损坏包；不依赖 GitHub 或用户账号。"""
import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
import zipfile

spec = importlib.util.spec_from_file_location("ci_release", Path(__file__).resolve().parents[1] / "ci-release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.expected = dict(commit="a" * 40, version="1.3.1", build="2101", tag="v1.3.1-ci.123.1")

    def package(self, platform):
        info = dict(self.expected, platform=platform, signing="unsigned")
        path = self.directory / f"Codex-Account-Switch-{info['tag']}-{platform}.zip"
        with zipfile.ZipFile(path, "w") as archive:
            for name in ("codex-account-switch.exe", "LICENSE", "THIRD_PARTY_NOTICES.md"):
                archive.writestr(name, "fixture")
            archive.writestr("BUILD_INFO.json", json.dumps(info))
        release.write_manifest(path, info)
        return path

    def complete_windows(self):
        self.package("windows-x64")
        self.package("windows-x86")
        (self.directory / "macos-signing-pending.txt").write_text("等待公司签名配置")

    def test_metadata_distinguishes_commits_and_full_reruns(self):
        env = dict(GITHUB_RUN_ID="123", GITHUB_RUN_NUMBER="11", GITHUB_RUN_ATTEMPT="1")
        tag, build = release.metadata(env)
        self.assertEqual(build, 2101)
        self.assertNotEqual(tag, release.metadata(dict(env, GITHUB_RUN_ID="124"))[0])
        retry, retry_build = release.metadata(dict(env, GITHUB_RUN_ATTEMPT="2"))
        self.assertNotEqual(tag, retry)
        self.assertGreater(retry_build, build)

    def test_windows_pair_with_signing_pending_is_valid(self):
        self.complete_windows()
        self.assertEqual(len(release.validate_packages(self.directory, self.expected)), 2)

    def test_missing_windows_architecture_is_rejected(self):
        self.package("windows-x64")
        with self.assertRaisesRegex(ValueError, "同时提供"):
            release.validate_packages(self.directory, self.expected)

    def test_old_commit_and_build_are_rejected(self):
        self.complete_windows()
        for field in ("commit", "build", "version", "tag"):
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "不一致"):
                release.validate_packages(self.directory, dict(self.expected, **{field: "wrong"}))

    def test_checksum_corruption_is_rejected(self):
        self.complete_windows()
        path = next(self.directory.glob("*.zip"))
        path.write_bytes(path.read_bytes() + b"corrupt")
        with self.assertRaisesRegex(ValueError, "SHA256"):
            release.validate_packages(self.directory, self.expected)

    def test_unlisted_zip_is_rejected(self):
        self.complete_windows()
        (self.directory / "unexpected.zip").write_bytes(b"unexpected")
        with self.assertRaisesRegex(ValueError, "清单"):
            release.validate_packages(self.directory, self.expected)

    def test_missing_macos_is_not_silently_ignored(self):
        self.package("windows-x64")
        self.package("windows-x86")
        with self.assertRaisesRegex(ValueError, "macOS"):
            release.validate_packages(self.directory, self.expected)

    def test_pe_architecture_must_match(self):
        path = self.directory / "fixture.exe"
        data = bytearray(128)
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 0x3C, 64)
        data[64:68] = b"PE\0\0"
        struct.pack_into("<H", data, 68, 0x8664)
        path.write_bytes(data)
        release.check_pe(path, 0x8664)
        with self.assertRaisesRegex(ValueError, "架构"):
            release.check_pe(path, 0x14C)

    def icon_pe(self, magic=0x20B):
        # 构造最小资源树，同时覆盖 32/64 位 PE 的不同可选头位置。
        data = bytearray(4096)
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 0x3C, 64)
        data[64:68] = b"PE\0\0"
        struct.pack_into("<HH", data, 68, 0x8664 if magic == 0x20B else 0x14C, 1)
        struct.pack_into("<H", data, 84, 240)
        struct.pack_into("<H", data, 88, magic)
        struct.pack_into("<II", data, 88 + (112 if magic == 0x20B else 96) + 16, 0x1000, 2048)
        struct.pack_into("<IIII", data, 88 + 240 + 8, 2048, 0x1000, 2048, 512)
        def directory(offset, entries):
            struct.pack_into("<HH", data, 512 + offset + 12, 0, len(entries))
            for index, item in enumerate(entries):
                struct.pack_into("<II", data, 512 + offset + 16 + index * 8, *item)
        directory(0, [(3, 0x80000040), (14, 0x80000080)])
        directory(64, [(n + 1, 0x80000100 + n * 32) for n in range(4)])
        directory(128, [(1, 0x80000180)])
        group = bytearray(struct.pack("<HHH", 0, 1, 4))
        for n, size in enumerate((16, 32, 48, 256)):
            directory(256 + n * 32, [(1033, 512 + n * 16)])
            struct.pack_into("<IIII", data, 1024 + n * 16, 0x1300 + n * 4, 4, 0, 0)
            data[1280 + n * 4:1284 + n * 4] = b"ICON"
            group.extend(struct.pack("<BBBBHHIH", size % 256, size % 256, 0, 0, 1, 32, 4, n + 1))
        directory(384, [(1033, 576)])
        struct.pack_into("<IIII", data, 1088, 0x1400, len(group), 0, 0)
        data[1536:1536 + len(group)] = group
        return data

    def test_windows_icon_resources_for_both_architectures(self):
        for magic in (0x10B, 0x20B):
            path = self.directory / "fixture.exe"
            path.write_bytes(self.icon_pe(magic))
            self.assertEqual(release.check_windows_icon(path), [(16, 16), (32, 32), (48, 48), (256, 256)])

    def test_missing_icon_or_dangling_group_reference_is_rejected(self):
        path = self.directory / "fixture.exe"
        for offset, fmt, value in ((216, "<I", 0), (1554, "<H", 999)):
            data = self.icon_pe()
            struct.pack_into(fmt, data, offset, value)
            path.write_bytes(data)
            with self.assertRaisesRegex(ValueError, "图标"):
                release.check_windows_icon(path)

    def test_assemble_command_produces_download_checksums_and_notes(self):
        self.expected["version"] = release.version()
        self.expected["tag"] = f"v{release.version()}-ci.123.1"
        self.complete_windows()
        env = dict(os.environ, GITHUB_SHA=self.expected["commit"],
                   CI_BUILD_NUMBER=self.expected["build"], RELEASE_TAG=self.expected["tag"])
        subprocess.run([sys.executable, str(Path(release.__file__)), "assemble",
                        "--directory", str(self.directory)], env=env, check=True,
                       capture_output=True, text=True)
        build = json.loads((self.directory / "BUILD_INFO.json").read_text())
        self.assertEqual(build["commit"], self.expected["commit"])
        self.assertEqual(len(build["packages"]), 2)
        self.assertEqual(len((self.directory / "SHA256SUMS.txt").read_text().splitlines()), 2)
        self.assertIn("等待公司签名配置", (self.directory / "RELEASE_NOTES.md").read_text())

    def smoke_process(self, message, exit_code):
        def spawn(_command, **kwargs):
            folder = Path(kwargs["env"]["CODEX_SWITCH_HOME"])
            folder.mkdir(parents=True)
            (folder / "app.log").write_text(message, encoding="utf-8")
            process = Mock()
            process.poll.return_value = exit_code
            process.returncode = exit_code
            return process
        return spawn

    def test_startup_failure_preserves_application_error(self):
        with patch.object(release.subprocess, "Popen", side_effect=self.smoke_process("图形初始化失败", 1)):
            with self.assertRaisesRegex(RuntimeError, "图形初始化失败"):
                release.startup_smoke(self.directory / "fixture.exe")

    def test_alive_process_without_ui_is_rejected(self):
        with patch.object(release.subprocess, "Popen", side_effect=self.smoke_process("starting", None)):
            with patch.object(release.time, "sleep"), self.assertRaisesRegex(RuntimeError, "事件循环"):
                release.startup_smoke(self.directory / "fixture.exe")

    def test_startup_requires_ui_log_and_live_process(self):
        with patch.object(release.subprocess, "Popen", side_effect=self.smoke_process("ui shown; entering event loop", None)):
            with patch.object(release.time, "sleep"):
                release.startup_smoke(self.directory / "fixture.exe")


if __name__ == "__main__":
    unittest.main()
