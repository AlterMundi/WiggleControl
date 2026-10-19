#!/usr/bin/env python3
"""Assemble a source-bound Linux candidate; no binary/model/sensor is executed.

Run in the matching source checkout after a native build and asset preparation.
Cargo metadata resolves licenses from its existing offline cache. Output is one
new tar.gz; publish its checksum through the selected delivery channel separately.
"""
import argparse
import gzip
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

from verify_preview import digest, verify

ROOT = Path(__file__).resolve().parents[1]
ARCHES = {'aarch64': ('arm64', 183, 'aarch64-unknown-linux-gnu'),
          'x86_64': ('x86_64', 62, 'x86_64-unknown-linux-gnu')}
TOOLS = ('fetch_provider_assets.py', 'prepare_preview_launcher.py',
         'manage_preview_launcher.py', 'mouse_launcher.py', 'prepare_input.sh',
         'rollback.sh', 'verify_preview.py')
SOURCES = ('Cargo.toml', 'Cargo.lock', 'LICENSE', 'NOTICE', 'README.md',
           'models/handpose_estimation_mediapipe-LICENSE', *('tools/' + p for p in TOOLS))


def git(root, *args):
    return subprocess.run(['git', '-C', str(root), *args], capture_output=True,
                          check=True, timeout=30).stdout


def metadata(root, architecture):
    return json.loads(subprocess.run(['cargo', 'metadata', '--offline', '--locked',
        '--features', 'native-vision', '--filter-platform', ARCHES[architecture][2],
        '--format-version', '1'], cwd=root, capture_output=True, check=True, timeout=120).stdout)


def licenses(source, target, data):
    nodes = {node['id']: node for node in data['resolve']['nodes']}
    selected, pending = set(), [data['resolve']['root']]
    while pending:
        package = pending.pop()
        if package not in selected:
            selected.add(package)
            pending.extend(nodes[package]['dependencies'])
    supplements = json.loads((source / 'packaging/dependency-licenses.json').read_text())['crates']
    records = []
    for package in sorted(data['packages'], key=lambda p: (p['name'], p['version'])):
        if package['id'] not in selected or package['id'] == data['resolve']['root']:
            continue
        if not package['license'] or not (package['source'] or '').startswith('registry+'):
            raise ValueError('Review non-registry or undeclared dependency license')
        name = package['name'] + '-' + package['version']
        if not re.fullmatch(r'[a-zA-Z0-9_.+-]+', name):
            raise ValueError('Invalid dependency identifier')
        crate = Path(package['manifest_path']).parent
        files = {p.name: p for p in crate.iterdir() if p.is_file()
                 and p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE'))}
        supplement = None
        if not files:
            supplement = supplements[package['name'] + '@' + package['version']]
            revision = json.loads((crate / '.cargo_vcs_info.json').read_text())['git']['sha1']
            path = source / 'packaging' / supplement['file']
            if revision != supplement['vcs_revision'] or digest(path) != supplement['sha256']:
                raise ValueError('Supplement does not match the published crate revision')
            files[path.name] = path
        destination = target / name
        destination.mkdir()
        for filename, path in files.items():
            if path.is_symlink() or path.stat().st_size > 128 * 1024:
                raise ValueError('Expected a bounded ordinary dependency license')
            shutil.copyfile(path, destination / filename)
        records.append({'name': package['name'], 'version': package['version'],
                        'license': package['license'], 'source': package['source'],
                        'repository': package.get('repository'), 'files': sorted(files),
                        'supplement': supplement})
    (target / 'dependencies.json').write_text(json.dumps(records, indent=2) + '\n')
    return len(records)


def assemble(candidate, assets, output, source=ROOT):
    candidate, assets, output, source = map(Path, (candidate, assets, output, source))
    if output.exists() or output.is_symlink():
        raise ValueError('Preserve existing output; select a new archive')
    if output.suffixes[-2:] != ['.tar', '.gz'] or not output.parent.is_dir():
        raise ValueError('Select a new tar.gz in an existing directory')
    manifest_path, binary = candidate / 'manifest.json', candidate / 'wigglecontrol'
    if (manifest_path.is_symlink() or not manifest_path.is_file() or manifest_path.stat().st_size > 8192
            or binary.is_symlink() or not binary.is_file()):
        raise ValueError('Expected regular source-bound candidate files')
    candidate_manifest = json.loads(manifest_path.read_text())
    commit, architecture = candidate_manifest['source_commit'], candidate_manifest['architecture']
    if (candidate_manifest['schema'] != 'wigglecontrol.candidate/v1'
            or not re.fullmatch('[0-9a-f]{40}', commit) or architecture not in ARCHES
            or candidate_manifest['features'] != ['native-vision']
            or candidate_manifest['receiving_qualified'] is not False
            or binary.stat().st_size > 128 * 1024 * 1024
            or digest(binary) != candidate_manifest['sha256']):
        raise ValueError('Candidate/source/digest contract differs')
    with binary.open('rb') as stream:
        elf = stream.read(20)
    if elf[:6] != b'\x7fELF\x02\x01' or int.from_bytes(elf[18:20], 'little') != ARCHES[architecture][1]:
        raise ValueError('Candidate ELF architecture differs')
    if git(source, 'rev-parse', 'HEAD').decode().strip() != commit:
        raise ValueError('Use the candidate source checkout for locked dependency metadata')
    # Metadata reads the working Cargo files; source/runtime payloads come from
    # immutable Git blobs, so local unrelated files can never enter the package.
    for name in ('Cargo.toml', 'Cargo.lock'):
        if (source / name).read_bytes() != git(source, 'show', commit + ':' + name):
            raise ValueError('Working dependency declarations differ from candidate source')
    with tempfile.TemporaryDirectory(prefix='.preview-package-', dir=output.parent) as temporary:
        stage = Path(temporary)
        root = stage / ('wigglecontrol-linux-' + ARCHES[architecture][0] + '-' + commit[:12])
        root.mkdir()
        names = set(SOURCES)
        names.update(git(source, 'ls-tree', '-r', '--name-only', commit, 'docs', 'packaging').decode().splitlines())
        for name in sorted(names):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(git(source, 'show', commit + ':' + name))
        shutil.copyfile(root / 'packaging/START-HERE.md', root / 'START-HERE.md')
        # Reuse the owning validator. Existing verified output needs no network
        # or cache; after copying selected files, validate the actual copy too.
        def validate(directory):
            subprocess.run(['/usr/bin/python3', str(root / 'tools/fetch_provider_assets.py'),
                '--output', str(directory.resolve()), '--cache', str(stage / 'unused-cache'),
                '--offline'], check=True, capture_output=True, timeout=60)
        validate(assets)
        lock = json.loads((root / 'packaging/provider-assets.lock.json').read_text())
        asset_root = root / 'provider-assets'; asset_root.mkdir()
        for name in (*lock['package']['files'], lock['model']['file'], 'APACHE-2.0-LICENSE',
                     'asset-bundle.json', 'provider-assets.json'):
            path = asset_root / name; path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(assets / name, path)
        validate(asset_root)
        (root / 'bin').mkdir()
        shutil.copyfile(binary, root / 'bin/wigglecontrol')
        if digest(root / 'bin/wigglecontrol') != candidate_manifest['sha256']:
            raise ValueError('Candidate changed while copying')
        (root / 'binary-manifest.json').write_text(json.dumps(candidate_manifest, indent=2) + '\n')
        license_root = root / 'licenses/rust'; license_root.mkdir(parents=True)
        count = licenses(root, license_root, metadata(source, architecture))
        # Cargo's graph excludes the Rust standard library linked into the
        # executable. Retain its own version-pinned notices as well.
        standard = json.loads((root / 'packaging/rust-standard-library.json').read_text())
        compiler = 'rustc ' + standard['version'] + ' (' + standard['source_revision'][:9] + ' '
        if not candidate_manifest['rustc'].startswith(compiler):
            raise ValueError('Candidate compiler differs from pinned standard-library notices')
        standard_root = root / 'licenses/rust-standard-library'; standard_root.mkdir()
        if set(standard['files']) != {'LICENSE-MIT', 'LICENSE-APACHE', 'COPYRIGHT'}:
            raise ValueError('Standard-library notice inventory differs')
        for name, entry in standard['files'].items():
            path = root / 'packaging/rust-standard-library' / name
            if digest(path) != entry['sha256']:
                raise ValueError('Standard-library notice digest differs')
            shutil.copyfile(path, standard_root / name)
        warmup = json.loads((root / 'packaging/warmup.json').read_text())
        if warmup['file'] != 'warmup.jpg' or digest(root / 'packaging/warmup.jpg') != warmup['sha256']:
            raise ValueError('Synthetic warmup differs from source metadata')
        files = {}
        for path in sorted(root.rglob('*')):
            if path.is_file():
                name = path.relative_to(root).as_posix()
                mode = 0o755 if name == 'bin/wigglecontrol' or name.endswith('.sh') else 0o644
                path.chmod(mode)
                files[name] = {'sha256': digest(path), 'mode': mode}
            elif path.is_dir():
                path.chmod(0o755)
        manifest = {'schema': 'wigglecontrol.preview-package/v1', 'source_commit': commit,
                    'source_url': 'https://github.com/AlterMundi/WiggleControl/tree/' + commit,
                    'architecture': architecture, 'receiving_qualified': False,
                    'release': False, 'rust_dependencies': count, 'files': files,
                    'rust_standard_library': standard['version'],
                    'provider_lock_sha256': digest(root / 'packaging/provider-assets.lock.json'),
                    'warmup': warmup}
        (root / 'preview.json').write_text(json.dumps(manifest, indent=2) + '\n')
        (root / 'preview.json').chmod(0o644)
        root.chmod(0o755)
        verify(root)
        archive = stage / 'preview.tar.gz'
        with archive.open('wb') as stream, gzip.GzipFile(fileobj=stream, mode='wb', filename='', mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode='w') as tar:
                for path in [root, *sorted(root.rglob('*'))]:
                    entry = tar.gettarinfo(str(path), str(path.relative_to(stage)))
                    entry.uid = entry.gid = entry.mtime = 0
                    entry.uname = entry.gname = ''
                    if path.is_file():
                        with path.open('rb') as content: tar.addfile(entry, content)
                    else:
                        tar.addfile(entry)
        # Same-filesystem exclusive publication preserves even a raced output.
        os.link(archive, output)
        return {'packaged': True, 'source_commit': commit, 'architecture': architecture,
                'sha256': digest(output), 'rust_dependencies': count,
                'receiving_qualified': False, 'binary_executed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('candidate', 'assets', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(assemble(args.candidate, args.assets, args.output)))


if __name__ == '__main__':
    main()
