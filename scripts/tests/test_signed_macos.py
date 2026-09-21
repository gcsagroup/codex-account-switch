"""公司签名包必须与同次 Windows 发布和完整源码指纹对应。"""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('signed', Path(__file__).resolve().parents[1] / 'verify-signed-macos.py')
signed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signed)


class SignedMetadataTests(unittest.TestCase):
    def setUp(self):
        self.published = dict(commit='a' * 40, tag='v1.4.0-ci.123.1', build='2301', version='1.4.0')
        self.sources = {'Cargo.toml': 'digest', 'src/main.rs': 'source'}
        self.info = dict(self.published, platform='macos-arm64', signing='company-signed', source_hashes=self.sources,
                         file='Codex-Account-Switch-v1.4.0-ci.123.1-macos-arm64.zip')

    def test_same_release_and_sources_are_accepted(self):
        self.assertEqual(signed.check_metadata(self.info, self.published, self.sources), self.info['file'])

    def test_wrong_commit_version_build_or_tag_is_rejected(self):
        for key in self.published:
            with self.subTest(key=key), self.assertRaises(ValueError):
                signed.check_metadata(dict(self.info, **{key: 'wrong'}), self.published, self.sources)

    def test_unsigned_wrong_platform_and_path_are_rejected(self):
        for change in ({'signing': 'unsigned'}, {'platform': 'windows-x64'}, {'file': '../bad.zip'}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                signed.check_metadata(dict(self.info, **change), self.published, self.sources)

    def test_missing_or_changed_source_is_rejected(self):
        for sources in ({}, {'Cargo.toml': 'changed', 'src/main.rs': 'source'}):
            with self.subTest(sources=sources), self.assertRaises(ValueError):
                signed.check_metadata(self.info, self.published, sources)
