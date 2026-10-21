# Independent browser-provider configuration

The GPU landmark provider accepts session/controller JSON without native hand
model or ONNX Runtime paths. The configuration checker uses the same admission
as live/replay operation, without assets, model initialization, camera access,
an HTTP listener or system input:

```sh
wigglecontrol config-check-provider session.json settings.json
```

This checks configuration consistency, not camera/browser/device readiness.
Native inference commands still require explicit absolute `models` and `runtime`
paths. A provider profile opting into ONNX visual macros still requires its
explicit absolute `runtime`; ordinary right click/detail/Escape/scroll do not.
Existing provider profiles containing native paths remain compatible.

## Configuration example

For a fresh preview, generate a coherent pair instead of manually copying the
illustrative JSON below:

```sh
wigglecontrol prepare-preview-config \
  /dev/video0 1920 1080 \
  /absolute/private/recordings /absolute/private/new-config
```

Choose the actual camera, single-output dimensions, private recording directory
and a new configuration directory whose parent exists. Paths must be absolute;
camera/recording locations need not exist for this camera-free preparation.
The command opens neither, initializes no inference and installs/starts nothing.
It writes private0700/0600 `session.json`, `settings.json` and `setup.json` using
actual provider admission and the same atomic new-directory publication as
calibration import. Existing destinations/symlinks are preserved, including one
appearing during publication. Invalid configuration leaves no partial directory.

The compiled [preview templates](../packaging/preview-session.json) use the
current experimental 2D wrist/middle-MCP pinch ratio0.38/0.4, one-second bounded
same-hand drag grace,300ms freshness and the selected independently timed cursor
filter/prediction/click stabilization. [Controller settings](../packaging/preview-settings.json)
enable left/right click, detail, Escape and two-axis scroll/inertia. These portable
numeric settings come from the selected beta; no personal workspace, host binding
or footage is included. They are a starting configuration, not universal physical
acceptance. Native inference paths remain absent.

The generated pair has the full default workspace and `calibrated:false`; it is
for [guided calibration](GPU-CALIBRATION.md), not immediate accepted mouse use.
After Save, import that profile with `prepare-provider-config` below. Recording is
explicitly configured because the current experimental camera path requires it;
only later manual Start opens capture. Video recording is off unless explicitly selected for a finite dataset sample. No microphone or upload is used.
See the [source setup walkthrough](PREVIEW-QUICKSTART.md) for the connected flow.

### Native Full HD capture

The session may explicitly select `"camera_resolution": [1920, 1080]` before
preparing its hash-bound launcher. Omitted resolution preserves640x480 for
existing installations. Supported dimensions are16..1920 by16..1080; the selected
camera must accept that exact MJPEG format. Unsupported negotiation is an error,
never a silent resize or an upscaled low-resolution image. Capture requests30FPS; measure actual driver timestamps rather than assuming
that rate is achieved. The native inference path uses `fps` for its10..20Hz
processing cadence. The browser provider consumes the latest available frame
with one inference in flight; its actual cadence depends on capture and processing.

These capture dimensions are distinct from `pointer_output`, which describes the
selected display. Browser frame leases, landmark geometry, preview metadata and
recording manifests retain the selected native capture dimensions. Recorded GPU
replay validates the manifest and each JPEG's matching dimensions. The synthetic
startup reference stays explicitly640x480 and never supplies calibration samples.

After changing camera or field of view, use a new calibration/configuration and
retain the previously accepted profile. Importing a saved calibration preserves
the selected camera resolution. A receiving browser/GPU choice is independent:
use the existing launcher browser executable hook and verify the actual model
renderer rather than inferring NVIDIA use from installed hardware.

### Lower-level 3D configuration example

The following files select the optional3D pinch path and minimal cursor defaults.
Use the generator above for the selected preview's 2D tuning and cursor policy.
These are complete illustrative files for checking the interface, not a calibrated
comfort profile or an instruction to start capture. Replace the camera and private
recording paths with the selected local resources. No daimon-integrators file,
Matrix connection or CompAII service is read.

`session.json`:

```json
{
  "camera": "/dev/video0",
  "fps": 10,
  "gain": 2000,
  "gesture_min_confidence": 0.5,
  "freshness_ms": 300,
  "provider_absolute_pointer": true,
  "pinch_3d": true,
  "pinch_close_m": 0.045,
  "pinch_open_m": 0.065,
  "recording_directory": "/absolute/private/wigglecontrol-recordings"
}
```

`settings.json`:

```json
{
  "gestures": {
    "min_confidence": 0.5,
    "stale_us": 300000,
    "close_m": 0.045,
    "open_m": 0.065
  },
  "right_click": true,
  "detail": {},
  "escape": {},
  "scroll": {"acceleration_max": 2.5, "inertia": {}}
}
```

Omitted workspace means the full normalized image, not a human-accepted
calibration. Preserve/import the accepted workspace in both `touch_workspace`
(session) and `workspace` (controller); cursor-output and gesture parameters must
also agree in both files. Do not replace an existing profile with these examples.
Recording is disabled by default. Dataset recording requires an explicit opt-in and finite 1..30-second trial; see [recording policy](RECORDING.md). The
checker reports whether recording is configured, without exposing its directory.
Pinch distances are model estimates, not calibrated physical measurements.

## Import a selected calibration without replacing it

For an already adopted profile, prepare a new matching session/controller pair:

```sh
wigglecontrol prepare-provider-config \
  /absolute/private/session-template.json \
  /absolute/private/settings-template.json \
  /absolute/private/accepted-profile.json \
  /absolute/private/new-mouse-config

wigglecontrol config-check-provider \
  /absolute/private/new-mouse-config/session.json \
  /absolute/private/new-mouse-config/settings.json
```

The templates must already pass provider admission. Their camera/recording paths,
gesture thresholds, cursor filtering and optional actions remain selected by you.
The command copies the profile's workspace into both files and its dimensions
into `pointer_output`. An explicitly configured different output is rejected;
do not reuse a profile for another display without review. Relative camera or
recording paths, mismatched settings and invalid profiles are rejected by the
existing validators. Native model/runtime paths remain optional.

The new directory must be absolute and its parent must exist. It is published
only after the actual serialized pair validates. Existing directories/files and
symlinks are preserved, including a destination appearing during preparation.
The directory is private0700 and its four files are0600: `session.json`,
`settings.json`, a copied `calibration.json` and a path-free `setup.json` report.
Source templates, the selected profile and its previous/draft/verification files
are not changed. No assets, model, camera, browser, listener or input device is
opened. This copies an explicitly selected valid profile; it does not certify
that its four corners were verified, perform a new calibration or install it.

Use the resulting `session.json` and `settings.json` as the explicit inputs to
[launcher preparation](PREVIEW-LAUNCHER.md). Keep the old configuration/bundle
for intentional rollback. The [guided GPU calibration](GPU-CALIBRATION.md) now
reuses the existing engine; receiving/human qualification remains open. The
existing `calibrate-guided` command uses native inference.

The selected GPU path still needs [pinned assets](PROVIDER-ASSETS.md), its explicit
synthetic warmup image and compatible Chromium. `config-check-provider` does not
download, prepare or acquire them. The existing `provider-camera-manual-preflight`
then checks the supplied assets/configuration without acquisition; actual manual
Start remains a separate boundary. No startup capture or automatic restart is
introduced by this configuration change.

## Qualification and remaining delivery

The actual CLI checker and camera-free provider replay accept omitted/empty
native paths, while native config checking and relative-path inputs reject them.
Opted-in visual macros reject missing runtime before model loading. Shared
gesture/workspace/cursor consistency checks remain enforced. These source checks
do not qualify a new installation or physical gestures.

The documented example passes the real CLI checker. Full native-feature suite:
238 passed, zero failures, one existing opt-in benchmark ignored; all-target
Clippy passes. No receiving configuration or service was changed.

Replay with native model/runtime paths removed preserves every non-timing output
value on3095 retained observations (248+393+1271+1183). Only actual per-call
`classifier_ms` wall time is excluded. No inference, camera, OS input or new human
intent labels are involved; this is compatibility evidence, not clean-install
or physical acceptance.

Fresh configuration generation, GPU guided calibration, normal-user launcher and
independent permissions helpers now exist; actual clean-host/post-boot and physical
qualification plus release assembly are still required by
[LINUX-PREVIEW.md](LINUX-PREVIEW.md) and issue12. The older
`install.sh` remains the native/integrator installer; it is not the complete GPU
preview installer.

Calibration import adds CLI qualification of the actual generated pair, private
permissions, preservation of all unrelated settings and original files, rejection
of mismatched outputs/settings, symlink profiles and unfinished drafts. A separate
publication regression preserves even an empty destination appearing during
preparation. The full native-feature suite now passes242 tests with zero failures
and one existing opt-in benchmark ignored; all-target Clippy passes. These checks
remain camera-free and do not qualify physical calibration or a receiving install.

An actual VPS preparation chain also passes: illustrative profile import, real
configuration check, then independent until-Stop launcher preparation against
the pinned assets. Source-file hashes remain unchanged; preflight confirms manual
Start with no acquisition or system input. The profile and blank warmup image
are fictional/preflight-only. No user units were linked/started, camera opened,
model initialized or input emitted; this does not qualify a live GPU calibration.

Fresh setup extends that real CLI chain: generated preview configuration, actual
provider check, calibration launcher preflight, explicitly fictional profile
import, another provider check and until-Stop mouse launcher preflight all pass.
Both prepared bundles also pass manager hash/binding validation. No unit was
installed, browser/model initialized, real profile saved, recording created or
camera/input opened. The warmup was an explicit blank synthetic preflight image;
it does not qualify graph warmup or physical tracking. An initial disk preparation
exceeded its30-second fixture deadline while the host showed high I/O pressure;
that terminal attempt remains retained. The next bounded120-second attempt
completed; the exact initial blocked syscall was not observed.

The full native-feature suite now passes248 tests, zero failures and one existing
ignored benchmark; all-target Clippy passes. New CLI regressions cover explicit
resource/dimension selection, private publication, absent personal mapping,
preservation of tuning during profile import, invalid resource/dimension rejection,
existing destination/symlink preservation and no partial output on normal errors.
The shared no-replace publication regression remains applicable. This does not
replace clean-host, post-boot, real guided-calibration or integrated human acceptance.
