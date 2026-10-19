#!/usr/bin/env python3
"""Prepare exact GPU-provider assets; no npm scripts, models, sensors or devices run."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import tarfile
import tempfile
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / 'packaging/provider-assets.lock.json'
RUNTIME = {'package/vision_bundle.mjs', *('package/wasm/'+n for n in (
    'vision_wasm_internal.js', 'vision_wasm_internal.wasm',
    'vision_wasm_module_internal.js', 'vision_wasm_module_internal.wasm',
    'vision_wasm_nosimd_internal.js', 'vision_wasm_nosimd_internal.wasm'))}
METADATA = {'package/package.json', 'package/README.md'}


def checked(data, entry, algorithm='sha256'):
    if len(data) != entry['bytes'] or hashlib.new(algorithm, data).hexdigest() != entry[algorithm]:
        raise ValueError('Asset bytes differ from the pinned manifest')
    return data


def load_lock(path):
    lock = json.loads(path.read_text())
    package, model = lock['package'], lock['model']
    if (lock['schema'] != 'wigglecontrol.provider-asset-lock/v1'
            or package['name'] != '@mediapipe/tasks-vision' or package['version'] != '1.1.1'
            or set(package['files']) != RUNTIME | METADATA
            or model['file'] != 'hand_landmarker.task'
            or lock['license_text']['file'] != 'models/handpose_estimation_mediapipe-LICENSE'):
        raise ValueError('Unsupported provider package/model/license manifest')
    for entry, algorithm in [(package, 'sha512'), (model, 'sha256'),
                             *((f, 'sha256') for f in package['files'].values())]:
        if (not isinstance(entry['bytes'], int) or not 0 < entry['bytes'] <= 64*1024*1024
                or len(entry[algorithm]) != hashlib.new(algorithm).digest_size*2
                or any(c not in '0123456789abcdef' for c in entry[algorithm])):
            raise ValueError('Invalid bounded asset size/digest')
    for entry in (package, model):
        if not entry['url'].startswith('https://'):
            raise ValueError('Expected an HTTPS artifact source')
    return lock


def atomic_bytes(path, data):
    fd, name = tempfile.mkstemp(prefix=path.name+'.', suffix='.partial', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data); stream.flush(); os.fsync(stream.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def cached(cache, name, entry, offline, algorithm='sha256'):
    path = cache / name
    if path.is_file():
        try:
            if path.stat().st_size != entry['bytes']:
                raise ValueError('Cached size differs')
            return checked(path.read_bytes(), entry, algorithm)
        except ValueError:
            if offline:
                raise ValueError('Offline cached artifact is corrupt: '+name) from None
    if offline:
        raise ValueError('Offline cached artifact is missing: '+name)
    with urlopen(entry['url'], timeout=30) as response:
        data = checked(response.read(entry['bytes']+1), entry, algorithm)
    atomic_bytes(path, data)
    return data


def prepare(output, cache, lock_path=LOCK, offline=False):
    lock = load_lock(lock_path)
    package, model = lock['package'], lock['model']
    license_bytes = (ROOT / lock['license_text']['file']).read_bytes()
    if hashlib.sha256(license_bytes).hexdigest() != lock['license_text']['sha256']:
        raise ValueError('Bundled Apache license differs from the pinned text')
    expected = {name: entry['sha256'] for name, entry in package['files'].items()}
    expected.update({model['file']: model['sha256'], 'APACHE-2.0-LICENSE': lock['license_text']['sha256']})
    provider = {'package_version': package['version'],
                'files': {n: expected[n] for n in sorted(RUNTIME | {model['file']})}}
    receipt = {'schema': 'wigglecontrol.provider-asset-bundle/v1',
               'lock_sha256': hashlib.sha256(lock_path.read_bytes()).hexdigest(),
               'files': expected, 'model_license_source': model['model_card'],
               'sdk_license_source': package['metadata_url']}
    generated = {'provider-assets.json': provider, 'asset-bundle.json': receipt}
    if output.exists():
        try:
            valid = all((output/name).is_file() and hashlib.sha256((output/name).read_bytes()).hexdigest() == sha
                        for name, sha in expected.items())
            valid &= all(json.loads((output/name).read_text()) == value for name, value in generated.items())
        except (OSError, ValueError):
            valid = False
        if not valid:
            raise ValueError('Existing asset directory differs; choose a new destination')
        return receipt
    cache.mkdir(parents=True, exist_ok=True)
    archive = cached(cache, 'tasks-vision-1.1.1.tgz', package, offline, 'sha512')
    model_bytes = cached(cache, model['file'], model, offline)
    # Read selected regular members only; never execute npm or extract archive paths.
    selected = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as source:
        names = [m.name for m in source.getmembers()]
        for name, entry in package['files'].items():
            if names.count(name) != 1 or PurePosixPath(name).is_absolute() or '..' in PurePosixPath(name).parts:
                raise ValueError('Ambiguous or unsafe selected archive member')
            member = source.getmember(name)
            if not member.isfile() or member.size != entry['bytes']:
                raise ValueError('Expected selected bounded regular archive member')
            with source.extractfile(member) as stream:
                selected[name] = checked(stream.read(entry['bytes']+1), entry)
    metadata = json.loads(selected['package/package.json'])
    if any(metadata.get(k) != package[k] for k in ('name', 'version', 'license')):
        raise ValueError('SDK metadata differs from the pinned package')
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.wigglecontrol-assets-', dir=output.parent) as stage:
        staging = Path(stage)/'bundle'; staging.mkdir()
        for name, data in {**selected, model['file']: model_bytes, 'APACHE-2.0-LICENSE': license_bytes}.items():
            target = staging/name; target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data); target.chmod(0o644)
        for name, value in generated.items():
            (staging/name).write_text(json.dumps(value, indent=2)+'\n')
        # Destination was never modified during download, validation or extraction.
        staging.rename(output)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--cache', required=True, type=Path)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    receipt = prepare(args.output.resolve(), args.cache.resolve(), offline=args.offline)
    print(json.dumps({'prepared': True, 'files_verified': len(receipt['files']),
                      'lock_sha256': receipt['lock_sha256'], 'camera_opened': False,
                      'system_input': False, 'npm_executed': False}))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, tarfile.TarError) as error:
        raise SystemExit(str(error)) from error
