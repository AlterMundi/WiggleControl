#!/usr/bin/env python3
"""Source-bound packaging with fictional ELF/assets/licenses; never execution."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import fetch_provider_assets as fetch
import package_preview as package
from test_fetch_provider_assets import fixture as asset_fixture
from verify_preview import verify


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'; self.source.mkdir()
        for name in package.SOURCES:
            path = self.source / name; path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(package.ROOT / name, path)
        shutil.copytree(package.ROOT / 'packaging', self.source / 'packaging')
        (self.source / 'docs').mkdir()
        (self.source / 'docs/PREVIEW-QUICKSTART.md').write_text('Fictional source guide\n')
        fixture = self.root / 'asset-fixture'; fixture.mkdir()
        cache, lock = asset_fixture(fixture)
        shutil.copyfile(lock, self.source / 'packaging/provider-assets.lock.json')
        self.assets = self.root / 'assets'
        fetch.prepare(self.assets, cache, lock, True)
        subprocess.run(['git', 'init', '-q', str(self.source)], check=True)
        subprocess.run(['git', '-C', str(self.source), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.source), '-c', 'user.name=Fixture',
            '-c', 'user.email=fixture@example.invalid', '-c', 'commit.gpgsign=false',
            'commit', '-qm', 'Fictional packaging source'], check=True)
        self.commit = package.git(self.source, 'rev-parse', 'HEAD').decode().strip()
        self.candidate = self.root / 'candidate'; self.candidate.mkdir()
        binary = b'\x7fELF\x02\x01' + b'\0' * 12 + (62).to_bytes(2, 'little') + b'FICTIONAL_NEVER_EXECUTED'
        (self.candidate / 'wigglecontrol').write_bytes(binary)
        self.manifest = {'schema': 'wigglecontrol.candidate/v1', 'source_commit': self.commit,
            'architecture': 'x86_64', 'features': ['native-vision'], 'receiving_qualified': False,
            'sha256': hashlib.sha256(binary).hexdigest(),
            'rustc': 'rustc 1.85.1 (4eb161250 2025-03-15)'}
        self.write_manifest()
        crate = self.root / 'crate'; crate.mkdir()
        (crate / 'LICENSE-MIT').write_text('FICTIONAL_LICENSE_NOT_A_REAL_DEPENDENCY\n')
        self.metadata = {'resolve': {'root': 'root', 'nodes': [
            {'id': 'root', 'dependencies': ['dependency']}, {'id': 'dependency', 'dependencies': []}]},
            'packages': [{'id': 'root', 'name': 'wigglecontrol', 'version': '0.1.0'},
                {'id': 'dependency', 'name': 'fixture', 'version': '1.0', 'license': 'MIT',
                 'source': 'registry+https://invalid.example/index', 'manifest_path': str(crate/'Cargo.toml')}]}

    def tearDown(self):
        self.temp.cleanup()

    def write_manifest(self):
        (self.candidate / 'manifest.json').write_text(json.dumps(self.manifest))

    def assemble(self, filename='preview.tar.gz'):
        with patch.object(package, 'metadata', return_value=self.metadata):
            return package.assemble(self.candidate, self.assets, self.root / filename, self.source)

    def test_complete_reproducible_inventory_ignores_private_extras_and_rejects_extracted_changes(self):
        for parent in (self.source, self.assets, self.candidate):
            (parent/'private-recording').write_text('NEVER_DISTRIBUTE')
        result = self.assemble()
        with tarfile.open(self.root/'preview.tar.gz') as archive:
            archive.extractall(self.root/'extracted', filter='data')
        root = next((self.root/'extracted').iterdir())
        manifest = verify(root)
        self.assertFalse(result['binary_executed'])
        self.assertFalse(manifest['receiving_qualified'])
        self.assertEqual(manifest['source_commit'], self.commit)
        self.assertTrue((root/'tools/prepare_input.sh').is_file())
        self.assertTrue((root/'provider-assets/APACHE-2.0-LICENSE').is_file())
        self.assertTrue((root/'licenses/rust/fixture-1.0/LICENSE-MIT').is_file())
        self.assertTrue((root/'licenses/rust-standard-library/COPYRIGHT').is_file())
        self.assertFalse(any(p.name == 'private-recording' for p in root.rglob('*')))
        previous = os.umask(0o077)
        try: repeated = self.assemble('repeated.tar.gz')
        finally: os.umask(previous)
        self.assertEqual(result['sha256'], repeated['sha256'])
        changed = root/'bin/wigglecontrol'; original = changed.read_bytes()
        changed.write_bytes(b'CHANGED')
        with self.assertRaisesRegex(ValueError, 'digest differs'): verify(root)
        changed.write_bytes(original); changed.chmod(0o600)
        with self.assertRaisesRegex(ValueError, 'permissions differ'): verify(root)
        changed.chmod(0o755)
        (root/'unlisted').write_text('UNKNOWN')
        with self.assertRaisesRegex(ValueError, 'inventory differs'): verify(root)

    def test_wrong_source_architecture_digest_and_existing_destination_never_publish(self):
        for key, changed in [('source_commit', '0'*40), ('architecture', 'aarch64'),
                             ('sha256', '0'*64), ('rustc', 'rustc 0.0.0')]:
            original = self.manifest[key]; self.manifest[key] = changed; self.write_manifest()
            with self.assertRaises(ValueError): self.assemble()
            self.assertFalse((self.root/'preview.tar.gz').exists())
            self.manifest[key] = original
        self.write_manifest()
        output = self.root/'preview.tar.gz'; output.write_text('PRESERVE')
        with self.assertRaisesRegex(ValueError, 'Preserve existing'): self.assemble()
        self.assertEqual(output.read_text(), 'PRESERVE')

    def test_corrupt_assets_or_missing_dependency_notices_refuse_publication(self):
        license_path = self.root/'crate/LICENSE-MIT'; original = license_path.read_bytes()
        license_path.unlink()
        with self.assertRaises(KeyError): self.assemble()
        self.assertFalse((self.root/'preview.tar.gz').exists())
        license_path.write_bytes(original)
        (self.assets/'hand_landmarker.task').write_text('CORRUPT')
        with self.assertRaises(subprocess.CalledProcessError): self.assemble()
        self.assertFalse((self.root/'preview.tar.gz').exists())

    def test_missing_crate_text_requires_hash_and_exact_published_vcs_revision(self):
        (self.root/'crate/LICENSE-MIT').unlink()
        vcs = self.root/'crate/.cargo_vcs_info.json'
        vcs.write_text(json.dumps({'git': {'sha1': 'a'*40}}))
        supplement = self.source/'packaging/fixture-LICENSE'
        supplement.write_text('FICTIONAL_UPSTREAM_LICENSE\n')
        index = self.source/'packaging/dependency-licenses.json'
        index.write_text(json.dumps({'crates': {'fixture@1.0': {
            'vcs_revision': 'a'*40, 'file': 'fixture-LICENSE',
            'sha256': package.digest(supplement), 'url': 'https://invalid.example/source'}}}))
        output = self.root/'notices'; output.mkdir()
        self.assertEqual(package.licenses(self.source, output, self.metadata), 1)
        self.assertEqual((output/'fixture-1.0/fixture-LICENSE').read_bytes(), supplement.read_bytes())
        vcs.write_text(json.dumps({'git': {'sha1': 'b'*40}}))
        with self.assertRaisesRegex(ValueError, 'published crate revision'):
            package.licenses(self.source, self.root/'wrong-notices', self.metadata)


if __name__ == '__main__':
    unittest.main()
