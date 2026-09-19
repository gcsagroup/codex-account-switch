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


if __name__ == "__main__":
    unittest.main()
