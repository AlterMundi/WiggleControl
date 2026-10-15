#!/bin/sh
# Explicit off-host comparison build; never deploys or starts a compositor.
set -eu
[ "$#" = 1 ] || { echo 'Usage: build_candidate.sh NEW_OUTPUT_DIRECTORY' >&2; exit 1; }
base=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
[ ! -e "$1" ] || { echo 'Output must be new; retain failed build evidence' >&2; exit 1; }
mkdir -p "$1"
output=$(CDPATH= cd -- "$1" && pwd)
export GIT_CEILING_DIRECTORIES="$output"
export CC=gcc-14
expected=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["gcc_version"])' "$base/source.json")
[ "$(gcc-14 -dumpfullversion)" = "$expected" ] || { echo 'Compiler version differs from source manifest' >&2; exit 1; }
commit=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["commit"])' "$base/source.json")
curl -fsSL "https://github.com/labwc/labwc/archive/$commit.tar.gz" -o "$output/source.tar.gz"
python3 - "$base/source.json" "$output/source.tar.gz" <<'PY'
import hashlib,json,sys
from pathlib import Path
assert hashlib.sha256(Path(sys.argv[2]).read_bytes()).hexdigest()==json.loads(Path(sys.argv[1]).read_text())['archive_sha256']
PY
mkdir "$output/source"
tar -xf "$output/source.tar.gz" -C "$output/source" --strip-components=1
meson setup "$output/build" "$output/source" --prefix=/usr --sysconfdir=/etc --buildtype=debugoptimized -Dtest=enabled -Dman-pages=disabled -Dxwayland=enabled
meson compile -C "$output/build" -j2
meson test -C "$output/build" --print-errorlogs
install -m755 "$output/build/labwc" "$output/labwc-baseline"
patch -p1 -d "$output/source" < "$base/touch-pointer-focus.patch"
meson compile -C "$output/build" -j2
meson test -C "$output/build" --print-errorlogs
install -m755 "$output/build/labwc" "$output/labwc-focus-candidate"
cp "$base/source.json" "$base/touch-pointer-focus.patch" "$base/build_candidate.sh" "$base/LICENSE" "$output/"
python3 - "$output" <<'PY'
import hashlib,json,platform,subprocess,sys
from pathlib import Path
root=Path(sys.argv[1])
files=['labwc-baseline','labwc-focus-candidate','source.tar.gz','touch-pointer-focus.patch','build_candidate.sh','source.json','LICENSE']
m={'schema':'wigglecontrol.compositor-candidate/v1','source':json.loads((root/'source.json').read_text()),
   'architecture':platform.machine(),'compiler':subprocess.check_output(['gcc-14','--version'],text=True).splitlines()[0],
   'packages':subprocess.check_output(['dpkg-query','-W'],text=True).splitlines(),
   'sha256':{name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in files},
   'build_options':['--prefix=/usr','--sysconfdir=/etc','--buildtype=debugoptimized','-Dtest=enabled','-Dman-pages=disabled','-Dxwayland=enabled'],
   'receiving_qualified':False,'upstream_merged':False}
(root/'manifest.json').write_text(json.dumps(m,indent=2)+'\n')
PY
tar -C "$output" -czf "$output/labwc-focus-candidate.tar.gz" labwc-baseline labwc-focus-candidate source.tar.gz source.json touch-pointer-focus.patch build_candidate.sh LICENSE manifest.json
