#!/usr/bin/env python3
"""Prepare pinned MobRecon topology with the provider's unchanged spiral code.

This one-time step uses Python3.9 because the pinned OpenMesh Linux wheel has
that ABI. Model inference reuses the separate Python3.10 CPU environment.
"""
import argparse
import json
from pathlib import Path
import pickle
import subprocess
import sys

from wilor_probe import sha256

SOURCE = '3c87e958d4855f890e3884ec94bcfc0f99422c3d'


def verify_source(repo):
    revision = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != SOURCE or subprocess.run(['git', '-C', str(repo), 'diff', '--quiet', 'HEAD']).returncode:
        raise ValueError('Expected unchanged pinned HandMesh source')
    return revision


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    verify_source(args.repo)
    if args.output.exists():
        raise ValueError('Refusing to overwrite prepared topology')
    import numpy as np
    import openmesh as om
    sys.path.insert(0, str(args.repo.resolve()))
    from utils.generate_spiral_seq import extract_spirals

    template = args.repo/'template/transform.pkl'
    with template.open('rb') as stream:
        data = pickle.load(stream, encoding='latin1')
    arrays = {'j_regressor': np.load(args.repo/'template/j_reg.npy', allow_pickle=False)}
    for index in range(4):
        mesh = om.TriMesh(np.array(data['vertices'][index]), np.array(data['face'][index]))
        arrays['spiral_'+str(index)] = np.asarray(extract_spirals(mesh, 9, dilation=1), dtype=np.int64)
        transform = data['up_transform'][index].tocoo()
        arrays['row_'+str(index)] = transform.row.astype(np.int64)
        arrays['col_'+str(index)] = transform.col.astype(np.int64)
        arrays['value_'+str(index)] = transform.data.astype(np.float32)
    with args.output.open('xb') as stream:
        np.savez(stream, **arrays)
    print(json.dumps(dict(source_commit=SOURCE, topology_sha256=sha256(args.output),
        template_sha256=sha256(template), python=sys.version,
        shapes={key:list(value.shape) for key,value in arrays.items()})), flush=True)


if __name__ == '__main__':
    main()
