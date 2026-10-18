"""Explicit root/bubblewrap file-and-ACL qualification; no real devices/services."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile

SOURCE = Path(__file__).resolve().parents[1]


def invoke(root, command):
    return subprocess.run([
        'bwrap', '--unshare-pid', '--unshare-net', '--unshare-uts', '--unshare-ipc',
        '--ro-bind', '/usr', '/usr', '--symlink', '/usr/lib', '/lib',
        '--symlink', '/usr/lib64', '/lib64', '--symlink', '/usr/bin', '/bin',
        '--proc', '/proc', '--dev', '/dev', '--bind', str(root/'uinput'), '/dev/uinput',
        '--tmpfs', '/tmp', '--bind', str(root/'usr-local'), '/usr/local',
        '--bind', str(root/'etc'), '/etc', '--bind', str(root/'var'), '/var',
        '--ro-bind', str(root/'artifacts'), '/artifacts',
        '--ro-bind', str(root/'test-bin'), '/test-bin', '--ro-bind', str(SOURCE), '/src',
        '--setenv', 'PATH', '/test-bin:/usr/bin:/bin', '/bin/sh', '-c', command,
    ], capture_output=True, text=True)


def setup(root):
    for name in ['usr-local/bin', 'usr-local/share/wigglecontrol/models',
                 'usr-local/lib/wigglecontrol', 'etc/wigglecontrol', 'etc/udev/rules.d',
                 'etc/daimon-integrators', 'etc/modules-load.d', 'etc/ssh', 'var', 'test-bin',
                 'artifacts/models', 'artifacts/runtime/lib']:
        (root/name).mkdir(parents=True, exist_ok=True)
    (root/'etc/passwd').write_text('root:x:0:0:root:/root:/bin/sh\ntester:x:1000:1000:tester:/home/tester:/bin/sh\n')
    (root/'etc/group').write_text('root:x:0:\ntester:x:1000:\n')
    (root/'etc/nsswitch.conf').write_text('passwd: files\ngroup: files\n')
    (root/'etc/daimon-integrators/devices.json').write_text('{"camera":"/dev/fake-camera"}')
    (root/'etc/ssh/sshd_config').write_text('ADMIN_SENTINEL\n')
    (root/'uinput').write_text('No real device\n'); (root/'uinput').chmod(0o600)
    (root/'test-bin/udevadm').write_text('#!/bin/sh\nexit 0\n')
    (root/'test-bin/modprobe').write_text('#!/bin/sh\n[ "$#" = 1 ] && [ "$1" = uinput ] || exit 88\nprintf "%s\\n" "$1" >> /var/module-calls\n')
    (root/'test-bin/cp').write_text('#!/bin/sh\n[ ! -e /var/fail-copy ] || exit 77\nexec /usr/bin/cp "$@"\n')
    for p in (root/'test-bin').iterdir(): p.chmod(0o755)
    (root/'artifacts/binary').write_text('TEST_BINARY_NOT_EXECUTED\n')
    models=[]
    for name in ['palm_detection_mediapipe', 'handpose_estimation_mediapipe']:
        filename=name+'_2023feb.onnx'; data=('TEST_MODEL_'+name).encode()
        (root/'artifacts/models'/filename).write_bytes(data)
        (root/'artifacts/models'/f'{name}-LICENSE').write_text('TEST_LICENSE\n')
        models.append({'file':filename,'sha256':hashlib.sha256(data).hexdigest()})
    (root/'artifacts/models/manifest.json').write_text(json.dumps({'models':models}))
    data=b'TEST_LIBRARY_NOT_LOADED'
    (root/'artifacts/runtime/lib/libonnxruntime.so.1.22.0').write_bytes(data)
    (root/'artifacts/runtime/LICENSE').write_text('TEST_LICENSE\n')
    arch={'x86_64':'x64','aarch64':'aarch64'}[platform.machine()]
    (root/'artifacts/models/runtime-manifest.json').write_text(json.dumps({'runtimes':[{'arch':arch,'version':'1.22.0','library_sha256':hashlib.sha256(data).hexdigest()}]}))


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--scratch',required=True)
    args=parser.parse_args(); assert os.getuid()==0, 'Explicit sudo required'
    scratch=Path(args.scratch); scratch.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='install-',dir=scratch) as temp:
        root=Path(temp); setup(root)
        paths=['usr-local/bin/wigglecontrol','usr-local/share/wigglecontrol/models/old-model',
               'usr-local/lib/wigglecontrol/old-library','etc/wigglecontrol/config.json',
               'etc/udev/rules.d/80-wigglecontrol-uinput.rules',
               'etc/udev/rules.d/80-wigglecontrol-keyboard.rules',
               'etc/modules-load.d/wigglecontrol.conf']
        originals={}
        for i,name in enumerate(paths):
            p=root/name; p.write_text(f'ORIGINAL-{i}\n'); p.chmod(0o640)
            originals[name]=p.read_bytes()
        baseline=invoke(root,'getfacl -p /dev/uinput > /var/original.acl; setfacl -m u:tester:rw /dev/uinput')
        assert baseline.returncode==0,baseline.stderr
        install='/src/tools/install.sh /artifacts/binary /artifacts/models /artifacts/runtime tester /var/original.acl'
        (root/'var/fail-copy').touch(); failed=invoke(root,install)
        assert failed.returncode!=0 and not (root/'var/lib/wigglecontrol/rollback/manifest').exists()
        assert all((root/p).read_bytes()==v for p,v in originals.items())
        (root/'var/fail-copy').unlink()
        for _ in range(2):
            result=invoke(root,install); assert result.returncode==0,result.stderr
            assert (root/'etc/modules-load.d/wigglecontrol.conf').read_text().splitlines()[-1]=='uinput'
            assert (root/'etc/udev/rules.d/80-wigglecontrol-keyboard.rules').read_bytes()==(SOURCE/'packaging/80-wigglecontrol-keyboard.rules').read_bytes()
        rolled=invoke(root,'/src/tools/rollback.sh; getfacl -p /dev/uinput > /var/restored.acl')
        assert rolled.returncode==0,rolled.stderr
        assert (root/'var/original.acl').read_bytes()==(root/'var/restored.acl').read_bytes()
        assert all((root/p).read_bytes()==v and (root/p).stat().st_mode&0o777==0o640 for p,v in originals.items())
        assert (root/'etc/ssh/sshd_config').read_text()=='ADMIN_SENTINEL\n'
        print('Interrupted snapshot, retry, content/mode rollback and explicit original uinput ACL restoration passed')

    # Older rollback manifests lack boot and keyboard-classification entries.
    # Upgrades preserve existing contents/mode or remove each newly added path.
    for existing in (False, True):
        with tempfile.TemporaryDirectory(prefix='upgrade-',dir=scratch) as temp:
            root=Path(temp); setup(root)
            boot=root/'etc/modules-load.d/wigglecontrol.conf'
            keyboard=root/'etc/udev/rules.d/80-wigglecontrol-keyboard.rules'
            if existing:
                keyboard.write_text('PRE_EXISTING_KEYBOARD_RULE\n'); keyboard.chmod(0o640)
                boot.write_text('PRE_EXISTING_BOOT_POLICY\n'); boot.chmod(0o640)
            (root/'etc/udev/rules.d/80-wigglecontrol-uinput.rules').write_text('KERNEL=="uinput", SUBSYSTEM=="misc", RUN+="/usr/bin/setfacl -m u:tester:rw /dev/uinput"\n')
            backup=root/'var/lib/wigglecontrol/rollback'; backup.mkdir(parents=True)
            original='remove /usr/local/bin/wigglecontrol\n'
            (backup/'manifest').write_text(original)
            acl=invoke(root,'getfacl -p /dev/uinput > /var/lib/wigglecontrol/rollback/uinput.acl')
            assert acl.returncode==0,acl.stderr
            prepare='/src/tools/prepare_input.sh tester'
            (root/'var/fail-copy').touch(); failed=invoke(root,prepare)
            assert failed.returncode!=0 and (backup/'manifest').read_text()==original
            assert boot.read_text()=='PRE_EXISTING_BOOT_POLICY\n' if existing else not boot.exists()
            (root/'var/fail-copy').unlink()
            for _ in range(2):
                result=invoke(root,prepare); assert result.returncode==0,result.stderr
                lines=(backup/'manifest').read_text().splitlines()
                assert len(lines)==3 and lines[0]==original.strip()
                assert keyboard.read_bytes()==(SOURCE/'packaging/80-wigglecontrol-keyboard.rules').read_bytes()
                assert boot.read_text().splitlines()[-1]=='uinput'
            rolled=invoke(root,'/src/tools/rollback.sh')
            assert rolled.returncode==0,rolled.stderr
            if existing:
                assert boot.read_text()=='PRE_EXISTING_BOOT_POLICY\n' and boot.stat().st_mode&0o777==0o640
                assert keyboard.read_text()=='PRE_EXISTING_KEYBOARD_RULE\n' and keyboard.stat().st_mode&0o777==0o640
            else:
                assert not boot.exists()
                assert not keyboard.exists()
            assert (root/'etc/ssh/sshd_config').read_text()=='ADMIN_SENTINEL\n'
    print('Legacy manifest upgrade preserves/removes boot and keyboard classification, interruption, retry and rollback passed')


if __name__=='__main__': main()
