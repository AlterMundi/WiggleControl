# Receiving installation and reversal

Current qualification is Linux ARM64 Pi 5, labwc/Wayland and a single 1080p output.
Source, device delivery, finite lifecycle and human hand accuracy are separate;
see QUALIFICATION.md for what is proved. No boot camera/inference session exists.

## GPU-preview asset preparation

The [full candidate package](PREVIEW-PACKAGE.md) assembles the native binary,
manual tools, pinned provider assets and licenses for ARM64/x86-64. Candidate
assembly/CI integrity checks remain distinct from clean-host or physical receiving
qualification and tagged delivery. Its START-HERE leads to the connected setup
guide; the source component paths below remain available independently.

The selected GPU preview now has a reproducible, non-root asset preparation path:
[PROVIDER-ASSETS.md](PROVIDER-ASSETS.md). It verifies the pinned SDK/archive/WASM
and task bundle, retains license/metadata and supports offline verified reuse.
This is separate from the older native installation below; standalone configuration,
launch, permissions/update/removal and receiving acceptance remain issue12 work.

The provider now accepts [independent session/controller configuration](PROVIDER-CONFIG.md)
without unused native hand models or ONNX Runtime paths. Validate it with
`wigglecontrol config-check-provider SESSION SETTINGS`, without assets or acquisition.
Native inference and opted-in ONNX visual macros retain their runtime requirements.
This is a configuration component, not a complete standalone setup/installer.

[Independent launcher preparation and user installation](PREVIEW-LAUNCHER.md)
emit manual-only user units and open/stop wrappers using that configuration and
the existing launch helper. The separate non-root manager supports scoped
install/update/rollback/removal, preserving bundles/configuration/profiles and
recordings; changing versions leaves input off. Actual VPS user-unit lifecycle
and interrupted-update recovery are qualified with zero capture/input. These
components do not yet assemble a release. The independent input preparation
below supplies kernel access separately; camera permissions and clean-host/
physical receiving qualification remain open. Do not use the
older native installer below as if it installed the independent GPU preview.

## Independent preview input preparation

With Python3, ACL utilities, kmod, udev and `flock` installed, the receiving
administrator can prepare input access without models, a binary, calibration or
any integrator/Matrix configuration:

```sh
sudo tools/prepare_input.sh --standalone RECEIVING_OPERATOR
```

Keep `packaging/80-wigglecontrol-keyboard.rules` alongside `tools` in the selected
source tree. This mode loads only the `uinput` kernel interface, registers its
boot module and grants the explicitly named existing account read/write access
to `/dev/uinput`. It also classifies the fixed-key Escape device for libinput.
It does not start a userspace input device, inference, camera, controller or
service. Normal desktop camera access must be available separately; this helper
does not add a user to broad input/video groups or run inference as root.

Independent mode owns only these new paths:

- `/etc/modules-load.d/wigglecontrol-preview.conf`
- `/etc/udev/rules.d/81-wigglecontrol-preview-uinput.rules`
- `/etc/udev/rules.d/81-wigglecontrol-preview-keyboard.rules`

Existing files/symlinks at these paths are refused on first preparation. The
older native installation's files, configuration and rollback are preserved.
The first device ACL, numeric normalized ACL, expected grant, operator/name UID
and exact selected inventory remain in the root-private
`/var/lib/wigglecontrol/preview-input-rollback`. Retries preserve that baseline;
one retained operator is supported per host. Rule updates/operator changes need
explicit reconciliation of that retained installation, not silent adoption.

After stopping/removing the selected controller with the user manager and
confirming its input/camera are closed, remove the independent grant:

```sh
sudo tools/rollback.sh --standalone
```

This validates all three selected files and the current ACL before mutation,
removes only those files and restores the original ACL entries. Device ownership
and other administration changes are not restored from historical metadata.
An independently changed file/ACL is preserved and reported for reconciliation;
removal is not complete in that case. The rollback retains its private baseline,
supports a repeated removal/reinstall with the same baseline and leaves the
kernel module loaded because another client may use it. It neither deletes
user configurations/recordings nor stops unrelated sessions.

Isolated root/bubblewrap qualification uses a regular fake device and stubbed
module/udev calls. Fresh preparation without integrator configuration, interrupted
snapshot and partial-apply recovery, retry, original read-only ACL restoration,
removal/reinstall, foreign-file/ACL refusal and preservation of an existing native
installation pass. Existing native installation/rollback regressions still pass.
This proves file/ACL behavior, not actual udev execution, a reboot, real device
classification or receiving normal-user access. Those remain #4/#12 checks on
a coordinated host; no permission changes were made on the Pi by this increment.

## Build and artifacts

Use the ordinary checkout with Rust 1.85 or later. Native vision requires libclang
for the V4L build dependency; the CPU runtime is a pinned separately packaged
native library, not a GPU dependency. Receiving tools include Python 3.12+ (safe
archive extraction), ACL utilities, kmod and udev. Reuse existing distribution packages.

```sh
python3 tools/fetch_models.py
python3 tools/fetch_runtime.py
cargo build --release --locked --features native-vision --bin wigglecontrol
```

The fetch tools verify exact manifest hashes and preserve cached artifacts under
ignored models/runtime directories. Model and native-runtime upstream licenses
travel with installation; original Rust code licensing does not replace them.
Do not upgrade another component's compiler or runtime to install this component.

On the existing integrator, an administrator runs:

```sh
sudo tools/install.sh target/release/wigglecontrol models \
  runtime/onnxruntime-linux-aarch64-1.22.0 RECEIVING_OPERATOR
```

Choose the actual receiving operator and architecture, not a copied host/account.
The camera binding is reused from the integrator's existing devices configuration.
Existing WiggleControl configuration is preserved. New configuration has FPS 10,
gain 2000, and absolute local model/library/device paths. Calibrate against real
hand comfort before treating gain/workspace defaults as final. No credentials,
camera serials, host endpoints or captured images belong in Git.

Source versions after the initial receiving deployment support an optional
`touch_workspace` entry in that existing JSON configuration:

```json
"touch_workspace": {
  "min": {"x": 0.2, "y": 0.2},
  "max": {"x": 0.8, "y": 0.8}
}
```

Merge this entry into the configuration object; it is not a complete config file.
Coordinates refer to the mirrored, normalized camera cursor space. The chosen
rectangle maps onto the entire touch output; motion beyond its edges clamps to
the corresponding screen edge. Bounds must be finite, within 0–1 and at least
0.05 apart on each axis. Invalid calibration fails before models, camera or device
creation. Omission preserves the full-camera default. This setting affects direct
touch only; relative mouse sensitivity still uses `gain`. The example is fictional,
not a measured comfort profile. Older installed binaries reject this new field:
deploy the updated binary before adding it, and remove the field before reverting
to an older binary. The receiving config is preserved by installation/rollback.

## Escape keyboard source extension —2026-10-18

The controller writer source now creates a separate fixed-key Escape device.
Its classification rule travels in `packaging/80-wigglecontrol-keyboard.rules`;
keep this directory alongside `tools` when using the existing preparation scripts.
Installation and `prepare_input.sh` enroll/restore this path in the initial
rollback, including older manifests; the rule grants no additional access and
creates no device. Prepare before starting the candidate writer. Exact receiving
keyboard classification/application delivery and the actual gesture remain
unqualified; see [ESCAPE.md](ESCAPE.md). This extension does not publish or install
the independently packaged GPU preview. No selected host configuration changes
from a source update alone.

## Manual use

The2026-10-15 source adds optional experimental `adaptive_smoothing`:

```json
"adaptive_smoothing": {"min_cutoff_hz": 0.5, "beta": 20.0, "derivative_cutoff_hz": 1.0}
```

This is a configuration entry, not a complete file. Omission retains the existing
30ms exponential filter. Cutoffs must be0.01..120Hz and beta0..1000 in normalized
camera units. Optional `speed_floor` defaults to0 and must be0..1 normalized
units/second; it prevents low estimated speed from raising cutoff, without a
spatial dead zone. Validation occurs before resource loading. It affects filtered
cursor/scroll displacement in the shared gesture engine, not pinch thresholds.
The example is a simulation candidate, not a physically accepted profile. Older
receiving binaries reject the field; preserve the selected live config until
the matching artifact and human behavior are qualified.

Before starting a capture, validate a configuration edit without opening any
sensor/device or loading a model/runtime:

```sh
wigglecontrol config-check /etc/wigglecontrol/config.json
wigglecontrol --help
```

`config-check` validates the schema, bounded settings and absolute path syntax;
it does not check whether those resources exist or are accessible. Its JSON
report omits resource paths. Help lists commands available in the actual build
and distinguishes camera-only checks from input injection. These commands are
source additions after the initial receiving deployment, not yet installed there.

```sh
daimon-integrators input 30 mouse
daimon-integrators input 30 touch
daimon-integrators status
daimon-integrators stop
```

The supported duration is 1–120 seconds. Omitted mode defaults to mouse. Visible
preparation precedes acquisition; request acknowledgment is not device use or Stop
completion. Verify observed camera state and terminal managed status. The panel's
Stop ends only its owned process group, preserving physical inputs and other apps.
Keep the camera free of another active user during a trial; don't kill their app.

Direct CLI alternatives: `camera-check SECONDS CONFIG` and `camera-check-touch`
open the camera without input; `run` and `run-touch` inject mouse/direct touch.
Replay opens neither sensor nor device. Normal sessions retain aggregate summaries,
not frames; supervised logs are owner-only in the login runtime directory.
No VPS, model wakeup, Matrix attention or network upload is required at runtime.

Session JSON summaries also report fresh processed FPS and Linux resource
accounting over model load, camera work, gesture handling, device writer and
cleanup. CPU is normalized to one core (100% = one core); it can exceed 100%,
unlike the panel's all-core percentage. Runtime and reaped-child RSS peaks are
separate lifetime peaks in KiB, not simultaneous total memory; don't add them
and call the result an observed process-tree peak. Child peaks can include a
fork/exec transient and are not steady writer memory. Latency is driver timestamp
to intent construction, not optical exposure-to-application latency. No-accepted-
hand trials do not predict tracked-hand cost, jitter or gesture accuracy.

## Permissions and original baseline

Installation adds a named-operator uinput ACL plus a dedicated udev rule. It also
registers the kernel module and retains a dedicated
`/etc/modules-load.d/wigglecontrol.conf` containing `uinput`, using the normal
[systemd module-loading mechanism](https://github.com/systemd/systemd/blob/v257/man/modules-load.d.xml).
This registers the kernel interface before udev grants access after startup;
loading the module creates no userspace virtual input or sensor session. It does
not make the device world writable or run inference as root. The initial installed
file tree, config/rule, module-loading configuration and uinput ACL are retained in receiving rollback storage.
The exact baseline matters: if a temporary test grant preceded installation,
the installation-time ACL already includes it.

Supply an explicitly retained original ACL to include that earlier test grant in
reversal, without guessing which preexisting grants belonged to this work:

```sh
sudo tools/install.sh BINARY MODELS_DIR RUNTIME_DIR RECEIVING_OPERATOR ORIGINAL_UINPUT_ACL
```

The optional ACL must describe only `/dev/uinput`; the tool validates it and
preserves both the installation ACL and the selected original baseline. Without
this argument, rollback restores the installation-time ACL, including preexisting
grants. Custody paths and the selected receiving baseline stay in the private map.

For an existing installation that already has its matching named-operator rule
and retained original rollback, prepare this startup dependency without replacing
binaries, models or camera configuration:

```sh
sudo tools/prepare_input.sh RECEIVING_OPERATOR
```

The helper rejects a missing/mismatched installed rule or missing original rollback.
It extends an older manifest before replacing the new module-loading path, retaining
an existing file/symlink or recording its absence. Retry never replaces the first
baseline. It reloads rules, loads only `uinput`, waits at most10seconds for udev,
then restores only the named operator's access. Full installation calls this same
helper. This is not an input/session start and needs no human gesture capture.
Source/fixture qualification does not establish that a particular host rebooted
successfully; record controlled registration and actual reboot evidence separately.

## Rollback

End this component's owned session and verify it is terminal first. Then run:

```sh
sudo tools/rollback.sh
```

Rollback validates retained file snapshots before mutation, restores previous
artifacts/config/rule/module-loading file or removes fresh ones, restores the selected original ACL
and reloads udev rules. It leaves the kernel module loaded; unloading it might affect
other input clients. It neither stops arbitrary processes nor changes SSH,
sudo policy, the desktop or the independent panel. Reinstallation uses the same
qualified build/artifacts; the original rollback snapshot remains retained.

The isolated fixture checks interrupted snapshot/retry, original contents/modes
explicit ACL restoration, and old-manifest boot-file upgrade/retry/rollback on a fake device with no network or host service
effects. Actual receiving rollback/reinstallation has its separate evidence in
QUALIFICATION.md; a mocked fixture alone does not prove real permission removal.

## Local live hand feedback

`wigglecontrol camera-preview SECONDS CONFIG PORT` runs the same single-hand
native model and gesture engine without an input writer. SECONDS is a 1–90
second measured test after the first painted frame; preparation is separately
bounded to thirty seconds. The
HTTP listener binds only IPv4 loopback. On the receiving desktop, open
`http://127.0.0.1:PORT/` while the explicitly started session is running. Opening
the page never starts acquisition. Confirm human readiness separately for each
physical trial before invoking the command.

The mirrored image, 21 landmarks, thumb/index pinch segment and raw palm cursor
anchor share one frame. Upper-right data reports model confidence/eligibility,
pinch state and normalized ratio, press intents, frame age and separate decode/
inference time. These are estimates; no calibrated screen rectangle is implied.
Stale observations remain subject to the existing gesture freshness rule. The
view distinguishes producer-rejected frames from waiting on an older displayed
frame. Preview adds image transport/rendering overhead; it is diagnostic feedback,
not a production gesture throughput benchmark.

The page's **STOP PREVIEW** controls this session through a same-origin request;
expiry/SIGINT/SIGTERM also end it. The integrators panel observes camera use but
its managed-input Stop does not own a directly invoked preview. Confirm observed
camera closure. The server keeps only the latest in-memory snapshot plus an
in-flight response, sends no-store responses and clears memory on shutdown. No
video/frame file, microphone, network upload or startup service is created.

Initial receiving adoption uses the ordinary development binary, preserving the
installed production binary and operator permissions. Source publication and
physical alignment/pinch acceptance remain separate checks.

The preview header shows a red TESTING lamp and large countdown derived from
the native session's remaining milliseconds; duplicate frames do not reset its
clock. Ending the test extinguishes the lamp and retains the press counter.
Disconnect is distinguished from a known deadline. PINCH PRESSED is highlighted
in green in the upper-right state box and beside the thumb/index segment on the
image. Preparation/build/checks must finish before asking the human to be ready.

Optional `pinch_close_ratio` and `pinch_open_ratio` configure distance between
thumb/index landmarks divided by palm width. Omission preserves 0.25/0.4.
Values must satisfy 0 < close < open <= 1; the configuration is rejected before
model/device/camera acquisition otherwise. For a deliberately wider experiment,
0.35/0.5 allows closing at 35% of palm width and requires separation to at least
50% before releasing/rearming. Confirmation, freshness, eligibility and
open-before-press remain unchanged. The preview displays the selected thresholds.
Use an explicitly selected diagnostic config first and qualify intended pinches
and relaxed-hand false activations before adopting a new profile. Older binaries
reject these fields; this does not change the installed production defaults.

Preview timing revision: the page acknowledges its first painted camera frame
with a same-origin readiness request. This starts the measured TESTING window,
not camera acquisition. PREPARING has a thirty-second timeout; preview test
durations are 1–90 seconds, with a hard 120-second acquisition ceiling. Displayed
press counts exclude preparation. JSON summaries include preview_test_started,
preview_test_press_intents and preview_test_elapsed_seconds; the original
aggregate counts/resources still cover preparation plus testing.

The upper-right explanation now identifies open-before-press, close threshold,
confirmation, stale input, tracking loss, rejected posture, low confidence or
scroll-mode blocking. Confirmation time is shown separately; matching landmark
positions alone do not assert that the gesture engine accepted the frame.
A candidate 0.35/0.45 profile reduces release/rearm separation while retaining
hysteresis. It remains a diagnostic candidate until human acceptance.

When controlling the receiving browser remotely, await HTTP 200 from the preview
root on that receiving host before navigating to its loopback URL. A model-load
or connection delay must not leave the browser on an error page while the camera
continues preparation. On a failed bounded readiness probe, stop only the owned
preview, verify camera closure, restore the local page and obtain fresh human
readiness for another physical attempt. No probe or page navigation starts capture.

## Native desktop feedback, without video

`wigglecontrol run-feedback SECONDS CONFIG PORT` joins the existing relative
virtual mouse writer and bounded local metadata server. Separately run the
receiving daimon-integrators `hand-overlay PORT UI_SECONDS` on its Wayland desktop.
The native overlay requests `/state` only; no video is displayed or recorded,
and there is no second perception stack. Both commands are manual, finite and
require a new human ready acknowledgment before each physical gesture trial.

The overlay is a transparent GTK3/layer-shell surface above ordinary applications,
with no keyboard focus and an empty input region. Its separate native STOP INPUT
button targets `/stop` of this direct session. The existing independent panel
continues to report observed camera activity; its managed capture Stop does not
own this direct invocation. No background/startup capture is installed.

The hand is mirrored, with camera-pixel bone lengths around a palm anchor mapped
into the output workspace. This is **observed hand position**, not the relative
mouse's global cursor position. Cursor-coincident mapping and guided calibration
remain pending; no unsupported Wayland global pointer position is guessed.

Input remains neutral during preparation. Once the native overlay paints its
first metadata view, `/ready` starts the visible measured test/countdown. The
writer independently expires within the combined preparation/test bound and
releases on EOF or its freshness watchdog. Raw-frame freshness and cached-state
age determine whether the skeleton can remain visible.

Use the static `tools/qualification.html`, VLC or another existing application
below it. The temporary JavaScript hand renderer was removed; the page observes
trusted application input only. Camera-preview remains a separate diagnosis
view when checking alignment against the actual image.

## Calibrated absolute mouse and native controls

`wigglecontrol run-calibrated SECONDS CONFIG PORT PROFILE` selects the absolute
mouse writer and finite native feedback/control projection. Its receiving-local
config must include `pointer_output: [width, height]` matching the qualified
single output. Missing/invalid output or an incompatible existing profile fails
before inference/camera initialization. Existing modes/configs remain separate.
The native overlay refuses the measured-window acknowledgment if its actual
allocated dimensions differ. Each physical trial still requires human readiness.

CALIBRATE begins left/right/top/bottom collection. Hold each comfortable extreme
and choose NEXT; the runtime requires stable fresh samples. Buttons/wheel/motion
are neutral during collection. Corner review emits absolute hover only, requiring
a continuous half-second at each ordered corner. ACCEPT is valid only after all
four corners; CANCEL retains the old profile. ROLLBACK restores the previous
compatible profile. Each control releases/rearms the gesture engine and writer,
including rejected actions; a closed hand never becomes an adoption click.

Profiles are owner-local bounded JSON, atomically published with 0600 permissions
and a `.previous` rollback. Queue acknowledgment is not adoption: the native
calibration state reports stage/message, and accepted profile bytes prove saving.
No camera frames are retained. A corrupt/incompatible active profile is reported
without overwriting it. Source corner tests and storage tests are not physical
calibration acceptance.

In active use and corner review, the overlay anchors the hand to the same
normalized `screen_target` actually emitted to the system writer. During neutral
collection it shows the mirrored observation workspace. Physical mouse movement
after hand loss can override the last emitted target; the overlay does not
pretend to query a global Wayland cursor. Visual hand feedback may remain up to
300 ms with delayed coloring, while input/green press eligibility keeps the
strict 150 ms freshness rule. Live flicker/comfort still require human acceptance.
Prayer entry and physical acceptance remain pending.

### Prayer entry, opt-in after manual acceptance

`prayer_enabled` is a receiving-local config boolean, false by default. It is
valid operationally only for `run-calibrated`; other session modes reject it
before loading inference/camera resources. Keep it false until the human accepts
the manual workflow. Opt-in uses the existing native multi-hand tracker and
feeds pixel-space landmark/confidence/handedness/IDs to the candidate detector.

A fresh sustained candidate can request calibration only with an active accepted
profile, after the overlay acknowledgment and inside the finite measured window.
The runtime first releases/rearms its input engine, then enters neutral bound
collection. It never adopts a profile from a prayer gesture. Existing sampling
and corner review cannot be displaced by another gesture request. The detector
continues observing authorized frames during those stages, preserving its latch:
occlusion does not count as separation. No gesture activates an idle camera.

Feedback reports prayer enabled/available and idle/holding/latched states.
The selected pointer hand retains its tracker ID when present; replacement
still passes through the existing neutral/open-first gesture rules. Ambiguous
pairs release ordinary input and cannot request calibration. Prototype pose
thresholds/model coverage, paired inference latency and physical activation
remain unqualified; 2-D adjacency is not proof of physical palm contact.

Session `freshness_ms` optionally selects the gesture/frame and device-producer
watchdog budget (50..1000 ms; omitted preserves150 ms). Selected diagnostic
receiving profile uses300 ms. Metadata carries it to web/native feedback. Device
stream commands accept `[WATCHDOG_MS]`; defaults remain150 ms for mouse and250 ms
for standalone touch. Calibration/prayer sampling freshness remains separate.

For an explicitly requested manual continuous session, use
`run-calibrated-until-stop CONFIG PORT PROFILE` and the receiving
`hand-overlay-until-stop PORT`. These have no exit countdown; Stop/EOF/error
releases input. Recordings rotate indexed512 MiB segments. Existing finite
commands keep their duration bounds; no auto-start/restart is installed.
