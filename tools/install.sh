#!/bin/sh
# Root receiving install: explicit built artifacts, named operator, no startup capture.
set -eu
[ "$(id -u)" = 0 ] && [ "$#" = 4 ] || { echo 'Usage as root: install.sh BINARY MODELS_DIR RUNTIME_DIR OPERATOR' >&2; exit 1; }
binary=$1
models=$2
runtime=$3
operator=$4
case "$operator" in *[!a-zA-Z0-9_-]*|'') exit 1;; esac
getent passwd "$operator" >/dev/null
[ -f "$binary" ] && [ -f "$models/manifest.json" ] && [ -f "$runtime/lib/libonnxruntime.so.1.22.0" ]
# Verify exact candidate model bytes before publishing immutable receiving files.
python3 - "$models" "$runtime" <<'PY'
import hashlib,json,sys,platform
from pathlib import Path
root=Path(sys.argv[1])
for r in json.loads((root/'manifest.json').read_text())['models']:
 p=root/r['file']
 assert p.name==r['file'] and hashlib.sha256(p.read_bytes()).hexdigest()==r['sha256'],'model hash mismatch'
arch={'aarch64':'aarch64','x86_64':'x64'}[platform.machine()]
r=next(r for r in json.loads((root/'runtime-manifest.json').read_text())['runtimes'] if r['arch']==arch)
lib=Path(sys.argv[2])/'lib'/f"libonnxruntime.so.{r['version']}"
assert hashlib.sha256(lib.read_bytes()).hexdigest()==r['library_sha256'],'native runtime hash mismatch'
PY
backup=/var/lib/wigglecontrol/rollback
install -d -m 700 /var/lib/wigglecontrol
if [ ! -e "$backup/manifest" ]; then
 [ ! -e "$backup" ] || { echo 'Inspect incomplete prior rollback before installation' >&2; exit 1; }
 stage=$(mktemp -d /var/lib/wigglecontrol/rollback.new.XXXXXX)
 trap 'rm -rf "$stage"' EXIT HUP INT TERM
 : > "$stage/manifest"
 for path in /usr/local/bin/wigglecontrol /usr/local/share/wigglecontrol /usr/local/lib/wigglecontrol /etc/wigglecontrol/config.json /etc/udev/rules.d/80-wigglecontrol-uinput.rules; do
  if [ -e "$path" ]; then
   mkdir -p "$stage/files$(dirname "$path")"
   cp -a "$path" "$stage/files$(dirname "$path")/"
   printf 'restore %s\n' "$path" >> "$stage/manifest"
  else printf 'remove %s\n' "$path" >> "$stage/manifest"; fi
 done
 getfacl -p /dev/uinput > "$stage/uinput.acl"
 mv "$stage" "$backup"
 trap - EXIT HUP INT TERM
fi
install -d /usr/local/share/wigglecontrol/models /usr/local/lib/wigglecontrol /etc/wigglecontrol /etc/udev/rules.d
install -m 755 "$binary" /usr/local/bin/wigglecontrol.new
mv /usr/local/bin/wigglecontrol.new /usr/local/bin/wigglecontrol
for f in palm_detection_mediapipe_2023feb.onnx handpose_estimation_mediapipe_2023feb.onnx manifest.json palm_detection_mediapipe-LICENSE handpose_estimation_mediapipe-LICENSE; do install -m 644 "$models/$f" /usr/local/share/wigglecontrol/models/; done
install -m 755 "$runtime/lib/libonnxruntime.so.1.22.0" /usr/local/lib/wigglecontrol/
install -m 644 "$runtime/LICENSE" /usr/local/lib/wigglecontrol/ONNX-RUNTIME-LICENSE
if [ -f "$runtime/ThirdPartyNotices.txt" ]; then install -m 644 "$runtime/ThirdPartyNotices.txt" /usr/local/lib/wigglecontrol/; fi
python3 - <<'PY'
import json
from pathlib import Path
camera=json.loads(Path('/etc/daimon-integrators/devices.json').read_text())['camera']
p=Path('/etc/wigglecontrol/config.json')
if not p.exists():p.write_text(json.dumps({'camera':camera,'models':'/usr/local/share/wigglecontrol/models','runtime':'/usr/local/lib/wigglecontrol/libonnxruntime.so.1.22.0','fps':10,'gain':2000.0})+'\n')
PY
chmod 644 /etc/wigglecontrol/config.json
printf 'KERNEL=="uinput", SUBSYSTEM=="misc", RUN+="/usr/bin/setfacl -m u:%s:rw /dev/uinput"\n' "$operator" > /etc/udev/rules.d/80-wigglecontrol-uinput.rules
setfacl -m "u:$operator:rw" /dev/uinput
udevadm control --reload-rules
echo 'Installed manual-only WiggleControl; no camera/input session started'
