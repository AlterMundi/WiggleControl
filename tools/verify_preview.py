#!/usr/bin/env python3
"""Verify an extracted preview's complete inventory without running its binary."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import stat


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(root):
    root = Path(root)
    manifest_path = root / 'preview.json'
    if manifest_path.is_symlink() or not manifest_path.is_file() or manifest_path.stat().st_size > 256 * 1024:
        raise ValueError('Expected a bounded regular preview manifest')
    manifest = json.loads(manifest_path.read_text())
    if manifest['schema'] != 'wigglecontrol.preview-package/v1':
        raise ValueError('Unsupported preview manifest')
    files = manifest['files']
    if not isinstance(files, dict) or not 1 <= len(files) <= 512:
        raise ValueError('Invalid preview inventory')
    actual = set()
    for path in root.rglob('*'):
        if path.is_symlink():
            raise ValueError('Preview must not contain symlinks')
        if path.is_file() and path != manifest_path:
            actual.add(path.relative_to(root).as_posix())
        elif not path.is_dir() and path != manifest_path:
            raise ValueError('Preview must contain ordinary files/directories')
    if actual != set(files):
        raise ValueError('Preview file inventory differs')
    for name, entry in files.items():
        relative = PurePosixPath(name)
        if relative.is_absolute() or '..' in relative.parts or relative.as_posix() != name:
            raise ValueError('Invalid relative preview path')
        path = root / name
        if path.stat().st_size > 128 * 1024 * 1024 or digest(path) != entry['sha256']:
            raise ValueError('Preview file digest differs: ' + name)
        if stat.S_IMODE(path.stat().st_mode) != entry['mode']:
            raise ValueError('Preview file permissions differ: ' + name)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', nargs='?', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    manifest = verify(args.directory)
    print(json.dumps({'verified': True, 'source_commit': manifest['source_commit'],
                      'architecture': manifest['architecture'], 'files': len(manifest['files']),
                      'receiving_qualified': manifest['receiving_qualified'], 'binary_executed': False}))


if __name__ == '__main__':
    main()
