# WiggleControl Linux preview candidate

This archive is a preparation candidate, not a human-qualified published release.
It contains the native binary, manual setup tools, pinned GPU assets, licenses,
an original synthetic startup image and a complete file/checksum manifest.
Chromium and Linux desktop/device dependencies are supplied by the receiving
system. No Matrix, CompAII or daimon-integrators installation is required.

Check the archive's external SHA256 against its selected download source, extract
it into a new stable version directory and run from that directory:

```sh
python3 tools/verify_preview.py
```

This verifies integrity; it does not authenticate an unknown download source or
execute the binary. `preview.json` and `binary-manifest.json` identify the exact
source, architecture, compiler/build system and unqualified receiving status.
The dependency declarations are included for provenance; building the runtime
from source uses the corresponding Git checkout linked in the manifest.

Continue with [the connected setup guide](docs/PREVIEW-QUICKSTART.md). From this
extracted root, its packaged artifact choices are:

```sh
preview_binary="$PWD/bin/wigglecontrol"
preview_assets="$PWD/provider-assets"
preview_warmup="$PWD/packaging/warmup.jpg"
```

Choose your own camera/output and a separate private configuration/recording
directory. Camera-only calibration requires explicit Start and all four corners
before Save. Mouse use requires the administrator's separate input preparation
and another manual Start. No capture or controller is enabled at boot.

Video recording is off by default, including calibration and ordinary mouse
use. A recording directory alone is not opt-in. Only explicitly requested finite
dataset samples record: maximum 30 seconds/256 MiB per take, 2 GiB total store and
2 GiB free-space reserve. See [recording policy](docs/RECORDING.md). No microphone
or upload is used. Stop closes capture; retained samples require intentional cleanup.
The guide explains scoped Stop, update/rollback and removal.
Keep physical input available. Confirm real input/camera closure before reversing
permissions; failed removal is reported rather than replacing independent edits.

The startup JPEG is a constant color, not a hand photograph or reconstructed rig.
It exercises initial detector inference without private images. Full hand/finger
graph warmup, physical accuracy and performance are not established by it.
Compatibility/GPU, clean-host/post-boot and integrated human qualification remain
required before publishing the public preview as accepted.
