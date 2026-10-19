# Independent guided GPU calibration

The browser GPU worker feeds the existing Rust diagonal sampling, four-corner
review and profile store. Calibration is camera-only: it cannot create a virtual
mouse or keyboard. Its separate controls show instructions and target/observed
points without camera images or a skeleton. Receiving/human qualification is
still pending; preserve the accepted Pi profile when selecting a trial candidate.

## Preparation

Generate a fresh pair with [`prepare-preview-config`](PROVIDER-CONFIG.md#configuration-example)
or use a valid existing standalone provider configuration, pinned assets
and explicit synthetic warmup. Set session `pointer_output` to the selected single
display's dimensions. Select an absolute profile path with an existing private
parent. Existing profiles must validate, belong to the current user and match
those dimensions; preparation reads them without replacement.

```sh
wigglecontrol provider-calibrate-preflight \
  /absolute/private/session.json /absolute/private/settings.json \
  /absolute/provider-assets /absolute/synthetic-warmup.jpg \
  /absolute/private/calibration.json 18877

python3 tools/prepare_preview_launcher.py \
  --binary /absolute/wigglecontrol --browser /usr/bin/chromium \
  --session /absolute/private/session.json \
  --settings /absolute/private/settings.json \
  --assets /absolute/provider-assets --warmup /absolute/synthetic-warmup.jpg \
  --calibration-profile /absolute/private/calibration.json \
  --output /absolute/private/calibration-launcher \
  --port 18877 --name calibration
```

Calibration and `--seconds` are mutually exclusive. Follow the existing
[explicit user-unit operation](PREVIEW-LAUNCHER.md#user-installation-update-and-rollback)
using this bundle. Its source is `provider-calibrate` with the same arguments as
preflight; detector/controls use `/provider/worker` and `/provider/calibration`.
Preparation starts no units, browser, model, listener, sensor or input device.
No Matrix/integrators configuration is required. Before Start, coordinate camera
ownership and stop the selected mouse session if it is using that camera;
opening idle calibration does not interrupt another application.

## Human flow and lifecycle

Start is explicit. The worker prepares on the synthetic image, then the source
opens camera/private recording. Warmup points never calibrate. Each stage clock
waits for a source frame and acknowledgment that its instruction painted.

Each diagonal stage gives15 seconds to move and5 seconds to hold, reusing the
existing stable-sample/freshness rules and workspace margin. All four corners
then require fresh continuous dwell in order, each within20 seconds. Two diagonal
inputs alone cannot finish. Missing/out-of-frame/ambiguous/unclear observations
do not supply valid samples, and tracking loss/staleness interrupts corner dwell.

Completion or failure closes capture before the next choice. Save requires all
four corners, saved verification and confirmed camera closure; it adopts through
the existing profile store with a previous-profile backup. Retry discards this
attempt's draft/verification and returns to idle; a new Start is required. Cancel
closes capture and preserves the adopted profile. No action automatically starts
another acquisition.

Capture is bounded to150 seconds per attempt. With camera off, results can wait
up to ten minutes for Save/Retry/Cancel; expiry never adopts them. Closing controls
does not stop capture: reopen through `open.sh`, choose Cancel or use the scoped
`stop.sh`. Save/Cancel end the source; controls retain its final state.

Private recording is currently required for this experimental camera path; each
attempt creates its own recording. No microphone is used. After explicit Save,
use [`prepare-provider-config`](PROVIDER-CONFIG.md#import-a-selected-calibration-without-replacing-it)
for a new matched mouse configuration. Calibration never rewrites/starts an
existing mouse bundle. Native `calibrate-guided` remains a separate supported path.

## Qualification

Camera-free tests reuse actual sampling/review/store code: two diagonals continue
into four corners; Save waits for closure; previous profiles survive; no-hand
failure, manual Retry, capture limit, Cancel and lost-frame dwell are covered.
Run-bound HTTP actions, painted-step controls, retained terminal state and the
manual camera-only launcher have matching regressions.

The full native-feature suite passes246 tests, zero failures, with one existing
ignored benchmark; all-target Clippy, the control JS and seven launcher tests
pass. Actual VPS user-unit preparation/open reaches idle calibration with no
model preparation, camera/input, profile or recording creation. Cancel through
the real browser ends source/worker with exit0, terminal-display acknowledgment
and retained controls. Selected units/displays were stopped and installed test
links removed; private evidence remains retained.

The actual controls fit1920×1080 and1600×950 without page scrolling;375px reflows
vertically without horizontal overflow. SVG target/point rendering was inspected
with an explicitly labelled presentation fixture after the source ended. It is
not physical tracking evidence. The browser check used existing CDP after the
Playwright attachment failed; controls used software rendering in the second
fixture. GPU/model performance was not measured. An earlier bounded fixture
expired in idle with zero capture/input; its timeout is retained separately.

These checks do not establish physical calibration accuracy, live GPU performance
or human comfort. Receiving trials, complete install/update/removal and offered
ThinkPad qualification remain in [LINUX-PREVIEW.md](LINUX-PREVIEW.md), issue12/#13.

## Cata idle graphics and compact layout —2026-10-19

On actual Debian13/GNOME Wayland X230, the original selected headless Chromium
flags produce WebGL2 through ANGLE/Mesa Intel HD Graphics4000, OpenGL4.2; this
is hardware-context availability, not model inference speed or hand accuracy.
The packaged7aed4ef source entered manual idle under its installed normal-user
unit, without starting the product worker/control browser. A separate bounded
owned diagnostic browser inspected the page without touching human tabs.

At1366×700 available content area, the original page was826px high. A compact
height-specific CSS rule now reduces spacing/type/SVG height on desktop viewports
at most850px tall, retaining the original1080p presentation and narrow-screen
reflow. Actual Cata prototype checks fit1366×700 and1366×768 without whole-page
scrolling;1920×1080 remains fitted. This prototype was injected only into the owned
diagnostic page before the source change; matching binary deployment is separate.

Actual Cancel before Start reached retained camera-off/mouse-off terminal state,
source exit0, zero capture/input and no profile/recording. Diagnostic process was
stopped after the check. This qualifies idle cancellation and a viewport fix;
physical retry/cancel after acquisition, all four corners and human Save remain
unqualified. No camera Start, model warmup or virtual input was requested.

Source309234744276a91c0e1aa5526bbc7f097eff46f9 passed all four CI39207545887
jobs and was deployed as the complete x86-64 archive. External archive SHA256
480b98813bc0837062e7a39f8ff5b023422e54416cabeb7da1cae4a299f6411d and all249
files/modes passed receiving verification. Updating the selected user launcher
preserved both configuration hashes and retained the previous package. The actual
product window, rather than an injected prototype, fits its1366×712 content area
without scrolling; Start/Cancel and disabled Save/Retry are fully visible.
Source, worker and controls opened in manual idle with camera/input off. The
human manually started calibration, but it terminated with EINVAL before any
captured frame or timer, without input/profile. Separate non-streaming checks of
the same requested format/rate and buffers pass; the exact failing operation is
not yet established. Source camera errors now name their failing operation; that
diagnostic increment has not been deployed. Temporary owned
inspection forwarding and the diagnostic unit were removed; future control
launches use the normal flags. No physical acceptance follows from this layout
or idle lifecycle proof.

At Nicolás's fresh request, a separate bounded camera-only probe (no Chromium,
model or input) reproduced EINVAL in the first stream read, before receiving any
frame. Raw private recording files remain empty. Non-streaming Rust setup and
both QBUF operations pass. This narrows the failing phase but does not yet
establish whether STREAMON or dequeue fails, nor its driver/hardware cause.
No new physical calibration was started after that probe.

### Startup feedback followup

Controls now display preparation until a tracking frame reaches calibration;
opening a video device alone does not start an instruction or its timer. Startup
and interrupted-session errors retain their reason, distinguish closing from
confirmed closure, and preserve the saved-profile guidance. `has_frame` proves a
tracking observation reached calibration, not that no raw camera bytes existed.
Existing JS lifecycle tests now cover these distinctions, alongside painted-step
acknowledgments and closure-gated Save. An isolated Chromium presentation fixture
fits preparing, startup-error and interrupted states at1366×712 without scrolling;
this is source/UI evidence, not a new Cata deployment or acquisition. The earlier
diagnostic source3cffb0c passed all four CI39209268610 jobs.

## Cata first-frame timeout diagnosis —2026-10-19

A freshly acknowledged bounded camera-only trace, without Chromium/inference or
system input, establishes this sequence: format/rate/MMAP setup and both initial
QBUF calls pass, STREAMON succeeds, then after250ms another QBUF for buffer0 fails
with EINVAL. No successful DQBUF occurs between those queue operations. Capture
closes and recording remains empty. This supersedes the earlier uncertainty
about STREAMON versus dequeue; it establishes a duplicate queue after a timeout,
not a failed format or an exposure-control diagnosis.

The pinned v4l0.14.0 capture `next` queues its last arena index whenever streaming
is active, including after a poll timeout before any successful dequeue. Current
upstream source was inspected and retains that pattern. The product now uses a
capture-only adaptation of its MMAP stream/arena in `src/camera_mmap`, reusing the
same pinned library's device/format/ABI/syscall helpers. It requeues only a
successfully dequeued buffer and clears that ownership after successful QBUF.
Timeouts keep waiting without duplicate queue, deliberate first-frame discard
or a longer blocking interval.
The250ms polling interval, per-poll Stop/deadline checks, raw timestamp validation,
private recording and independent writer deadlines remain unchanged. Inactive
stream teardown also avoids an unnecessary STREAMOFF. Complete upstream MIT
attribution/permission is retained in NOTICE and packaging/v4l-mmap-MIT.txt.

A camera-free regression uses a real private pipe: repeated timeouts must poll,
whereas an erroneous video QBUF would fail with ENOTTY. It covers initial wait,
a previously requeued buffer, and preservation of ownership on a failed queue.
This proves the queue-state fix without camera/model/uinput. A fresh real-camera
retry remains necessary before claiming captured frames, calibrated operation or
physical cancellation. No exposure, privacy, lighting or hardware setting was
changed to mask the protocol failure.

Upstream basis: [v4l0.14.0 capture stream](https://github.com/raymanfx/libv4l-rs/blob/867d9b122fde603ff7f1bfcdf19de48845c9e1f7/src/io/mmap/stream.rs)
and its matching arena; the local adaptation retains that revision's provenance.

Local validation: 251 native-feature tests, all-target native Clippy, formatting
and the four archive-assembly regressions pass. No fixed-camera retry is implied.

CI39218542025 built/tested the ARM64 and x86-64 candidates and passed replay;
the separate native job failed Clippy1.85.1 on six redundant lifetime declarations
in the adapted modules. Its actual job output was read before correction.
The declarations now elide only those unused lifetime names; buffer logic and
I/O are unchanged. Local Clippy1.95 had not reported those older-version lints.
A matching CI rerun is required before claiming the complete check set passed.
