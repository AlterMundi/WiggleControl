# Receiving installation and reversal

Current qualification is Linux ARM64 Pi 5, labwc/Wayland and a single 1080p output.
Source, device delivery, finite lifecycle and human hand accuracy are separate;
see QUALIFICATION.md for what is proved. No boot camera/inference session exists.

## Build and artifacts

Use the ordinary checkout with Rust 1.85 or later. Native vision requires libclang
for the V4L build dependency; the CPU runtime is a pinned separately packaged
native library, not a GPU dependency. Receiving tools include Python 3.12+ (safe
archive extraction), ACL utilities and udev. Reuse existing distribution packages.

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

## Manual use

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

Installation adds a named-operator uinput ACL plus a dedicated udev rule. It does
not make the device world writable or run inference as root. The initial installed
file tree, config/rule and uinput ACL are retained in receiving rollback storage.
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

## Rollback

End this component's owned session and verify it is terminal first. Then run:

```sh
sudo tools/rollback.sh
```

Rollback validates retained file snapshots before mutation, restores previous
artifacts/config/rule or removes fresh ones, restores the selected original ACL
and reloads udev rules. It neither stops arbitrary processes nor changes SSH,
sudo policy, the desktop or the independent panel. Reinstallation uses the same
qualified build/artifacts; the original rollback snapshot remains retained.

The isolated fixture checks interrupted snapshot/retry, original contents/modes
and explicit ACL restoration on a fake device with no network or host service
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
