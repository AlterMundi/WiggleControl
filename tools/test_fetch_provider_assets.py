#!/usr/bin/env python3
"""Offline bundle integrity/publication tests with fictional assets, never inference."""
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile

import fetch_provider_assets as fetch


def fixture(root, bad_member=False):
    files = {name: ('FICTIONAL_'+name).encode() for name in fetch.RUNTIME | fetch.METADATA}
    package = {'name':'@mediapipe/tasks-vision','version':'1.1.1','license':'Apache-2.0'}
    files['package/package.json'] = json.dumps(package).encode()
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw,mode='w:gz') as tf:
        for i,(name,data) in enumerate(sorted(files.items())):
            member=tarfile.TarInfo(name)
            if bad_member and i==0:
                member.type=tarfile.SYMTYPE;member.linkname='/unrelated-file'
                tf.addfile(member)
            else:
                member.size=len(data);tf.addfile(member,io.BytesIO(data))
    archive=raw.getvalue();cache=root/'cache';cache.mkdir()
    (cache/'tasks-vision-1.1.1.tgz').write_bytes(archive)
    model=b'FICTIONAL_MODEL_NOT_LOADED';(cache/'hand_landmarker.task').write_bytes(model)
    license_bytes=(fetch.ROOT/'models/handpose_estimation_mediapipe-LICENSE').read_bytes()
    lock={'schema':'wigglecontrol.provider-asset-lock/v1','package':{
        **package,'url':'https://invalid.example/sdk','metadata_url':'https://invalid.example/metadata',
        'bytes':len(archive),'sha512':hashlib.sha512(archive).hexdigest(),
        'files':{name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}for name,data in files.items()}},
        'model':{'file':'hand_landmarker.task','url':'https://invalid.example/model','model_card':'https://invalid.example/card',
                 'bytes':len(model),'sha256':hashlib.sha256(model).hexdigest()},
        'license_text':{'file':'models/handpose_estimation_mediapipe-LICENSE','sha256':hashlib.sha256(license_bytes).hexdigest()}}
    path=root/'lock.json';path.write_text(json.dumps(lock))
    return cache,path


def main():
    # Any accidental network dependency in offline preparation must fail this test.
    def forbidden(*_args,**_kwargs):raise AssertionError('Offline test attempted a download')
    fetch.urlopen=forbidden
    with tempfile.TemporaryDirectory() as d:
        root=Path(d);cache,lock=fixture(root);output=root/'bundle'
        receipt=fetch.prepare(output,cache,lock,True)
        assert receipt==fetch.prepare(output,root/'missing-cache',lock,True)
        assert not (root/'missing-cache').exists(), 'Verified bundle reuse should need no cache/network'
        provider=json.loads((output/'provider-assets.json').read_text())
        assert set(provider['files'])==fetch.RUNTIME|{'hand_landmarker.task'}
        assert (output/'APACHE-2.0-LICENSE').is_file() and (output/'package/README.md').is_file()
        sentinel=output/'unrelated';sentinel.write_text('RETAIN')
        (output/'package/vision_bundle.mjs').write_text('CORRUPT')
        try:fetch.prepare(output,cache,lock,True)
        except ValueError:pass
        else:raise AssertionError('Corrupt existing destination accepted')
        assert sentinel.read_text()=='RETAIN'
    for failure in ('cache','member','missing'):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);cache,lock=fixture(root,failure=='member');output=root/'bundle'
            if failure=='cache':(cache/'tasks-vision-1.1.1.tgz').write_bytes(b'CORRUPT')
            if failure=='missing':(cache/'hand_landmarker.task').unlink()
            try:fetch.prepare(output,cache,lock,True)
            except ValueError:pass
            else:raise AssertionError('Invalid assets accepted: '+failure)
            assert not output.exists(), 'Failed preparation published a partial bundle'
    print('Provider assets: offline integrity, exact manifest, license/metadata, repeat reuse, corrupt destination/cache and unsafe selected member passed; no inference/sensors/input')


if __name__=='__main__':main()
