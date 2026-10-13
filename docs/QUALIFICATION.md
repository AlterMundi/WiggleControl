# Development and receiving evidence

## 2026-10-13 — Gesture replay and Linux registration

Published gesture engine at b55d645, Linux adapter at f35eaa9.
VPS Rust 1.95.0 and receiving Pi 5 ARM64 Rust 1.85.1 each passed 14 tests:
pinch/drag, jitter/confirmation, identity replacement, neutral reacquisition,
stale/stop/loss/ineligible/malformed/out-of-order input release, CLI size and EOF
handling, Linux event codes and fractional movement. Local clippy passed;
GitHub Actions run 38387111298 passed fmt, tests and clippy on Rust 1.85.1.

The earlier run 38386242490 failed only an older Clippy `format_collect` lint in
a fixture-building test; its actual job output was inspected and that expression
was corrected. Do not attribute this to camera or ARM incompatibility.

Receiving temporary ACL allowed only the existing operator to create uinput
devices; original retained locally for reversal. A five-second no-event check
observed udev ID_INPUT_MOUSE=1 and automatic removal after expiry. A separate
finite check observed libinput's `Capabilities: pointer` and adaptive/flat/custom
acceleration availability. No camera or microphone was used. No application
movement/click was emitted. The existing panel remained running.

Remaining for milestone 2: actual native Wayland/XWayland application delivery,
finite watchdog/button-release/SIGKILL device removal tests, calibrated workspace
and scroll mapping. Device registration is not user acceptance.

## Native perception candidates

The separately enabled model-probe feature evaluates tract 0.21.15 with pinned
OpenCV Zoo palm/hand ONNX artifacts listed in models/manifest.json. These are
Apache-2.0 candidates, not MIT assets or a selected production backend. Licenses
are retained alongside the manifest; downloaded model binaries are ignored.
The probe uses a synthetic zero crop, opens no sensor, and separates model-only
load/timing from the eventual full pipeline. Compatibility/performance results
will be recorded after execution, rather than inferred from successful download.

Default camera-free runtime does not link the optional inference engine.
The dependency resolver honors the declared Rust 1.85 floor; an initial native
probe resolve selected kstring 2.0.5 (Rust 1.96) and was corrected to a compatible
locked dependency set without changing either host's compiler.


Offline candidate probe on VPS Rust 1.95.0: hand landmark model produced finite
outputs with shapes [1,63], [1,1], [1,1], [1,63] from the synthetic zero crop.
Debug-profile model-only timing was about 2591 ms for one run and is not a Pi
performance estimate. The palm model failed at Resize__235: expected output
[1,256,12,12], actual [1,256,6,6], both with and without graph optimization. Thus
this pinned tract candidate does not yet qualify the full two-model pipeline.
Preserve this concrete incompatibility and evaluate a different supported
backend/version before choosing production inference. No camera used.


ONNX Runtime 1.22.0 with ort 2.0.0-rc.10 runs both pinned models and produces
finite expected output shapes on VPS and receiving ARM64 Rust 1.85.1. On the
Pi's explicit silent profile (1.5 GHz cap, fan off, ongoing one-core encoder,
SoC about 83–85 C and firmware thermal limiting), 20 synthetic model-only runs
measured palm p50/p95 80.5/87.3 ms and hand 49.8/54.6 ms, using one inference
thread. This is not an accuracy, full-pipeline or thermally neutral benchmark.
Public runtime artifacts and their observed hashes are in runtime-manifest.json;
upstream assets did not supply a digest. Rust wrapper plus native CPU library is
selected for the next perception experiment after the exact tract palm failure.

Native perception geometry/anchor/fist-eligibility tests pass with replay and
Linux mapping tests (19 total). Generated 2016-anchor ordering was compared to
the pinned upstream literal table (maximum coordinate error 3.34e-8). The
current rotated ROI sampling uses bilinear interpolation and direct square crops,
not a claim of byte-identical OpenCV INTER_AREA preprocessing; source-linked
real-input comparison and hand accuracy remain pending. The producer uses
mirrored palm-center cursor and square fingertip metric; these are initial
calibration choices, not final human acceptance.


The first real 20-second camera-only session captured 293 frames, processed 184
(latest-frame drops 108), detected no accepted hands and injected nothing. Driver
timestamps were monotonic start-of-exposure; timestamp-to-intent p50/p95 about
119/150 ms while no hand was tracked. Camera closed afterward; microphone stayed
inactive. Absence of accepted hands does not establish that no hand was visible.

A cropped gesture-grid reference was rejected by both this pipeline and original
OpenCV, so it was not a positive test. The first frame of the pinned upstream
palm demonstration animation gave OpenCV confidence 0.9904 and Rust/ORT 0.9878.
Corresponding 21-point deviation: median 0.45 pixels, maximum 2.50 pixels on a
600x450 image. This is one source-backed positive preprocessing comparison, not
physical accuracy acceptance. Palm threshold now matches upstream's 0.5; hand
acceptance stays 0.8. Native vision plus replay/input tests now total 20.


The relative device delivered trusted pointer, click, drag and wheel events to
Chromium native Wayland at 1080p. A producer stall released a held button in
the application. Forced writer death was separately tested with a keepalive
preventing watchdog expiry: the application observed release and the virtual
device was absent afterward. These are real uinput/libinput/application effects,
not JavaScript/CDP-dispatched input. CDP was used only for state/geometry reads
and preparing this task-owned qualification window.

Installed manually enabled receiving artifacts with model/native-library hash
verification and rollback snapshot; a named-operator uinput rule persists the
required access without root perception. No startup camera/model service exists.
Integrators `input 20` entered visible preparation, then observed input activity
and camera-open with microphone inactive; its existing Stop ended the owned
process group and actual camera usage. Application/gesture human accuracy, VLC
/XWayland acceptance and direct multitouch remain pending.

## Direct-touch writer checkpoint — 2026-10-13

Twenty-two native-vision/replay/Linux tests pass on the development host. New
tests verify stable slots despite producer-array reordering, individual contact
loss, neutral release and rejection of duplicate/nonfinite contacts. The shared
bounded intent reader is used by mouse and touch writers; their watchdog clocks
remain independent of a blocked reader.

Receiving ARM64 Rust 1.85.1 built the camera-free touch writer. Real type-B
contacts reached the task-owned Chromium native Wayland page through uinput,
libinput and labwc. The page received trusted touchstart/touchmove events with
two simultaneous stable contacts despite alternating producer order. Its explicit
touch interpretation measured scale 1.709 and rotation 5.88 degrees. These values
describe the scratch touch-aware client, not universal system gesture support.

A producer stall then delivered touchend and left zero active contacts in the
application. Explicit Stop exited successfully and removed only the virtual
touch device. No camera/microphone was opened. Coordinates matched the single
1080p output; multi-output mapping remains unqualified. Two-hand recognition,
crossing/occlusion identity and camera-to-touch engagement are still pending.

## Two-hand gesture semantics checkpoint — 2026-10-13

Twenty-nine native-feature tests now pass. Five additional state tests qualify
independent confirmed pinches, stable contact ownership after producer reordering,
loss of one hand preserving the other, ambiguity release/open-before-repress with
new contact IDs, malformed/future/duplicate/regressed input, stale/Stop neutrality
and explicit absolute workspace mapping. Two CLI tests verify the fictional
two-hand replay, final EOF release and malformed-input failure with neutral output.
This is camera-free semantic evidence. It does not qualify native recognition of
two hands or whether the producer correctly notices a real crossing/occlusion.

## Native two-hand pipeline checkpoint — 2026-10-13

Native perception now uses score/NMS palm selection and two independent temporal
ROIs. Thirty source tests pass, including suppression and conservative ambiguous
identity geometry; those thresholds still require physical qualification.
A deterministic paired/mirrored version of the same permitted upstream positive
frame produced two accepted hands on both VPS and Pi 5. IDs 1/2 remained stable
through three repeated model frames; confidence ranged 0.9866–0.9916. This tests
native model/ROI plumbing on synthetic source-backed input, not real crossing,
occlusion, seated comfort or gesture accuracy.

The first finite supervised touch attempt captured 123 frames, processed 85,
accepted no hands and ended with BrokenPipe. Its actual log showed a final
inference/cleanup extending past the independent writer's finite deadline. The
session now avoids sending final inferred intent after expiry and accepts neutral
successful writer teardown at that deadline; early failure remains an error.
Receiving retest, rather than the code change alone, qualifies this correction.

Actual receiving five-second retests passed for both touch and mouse: panel phase
completed without error, camera opened then closed, matching virtual device was
created then removed, and microphone stayed inactive. Touch captured 70 frames,
processed 40, dropped 28, accepted zero hands; timestamp-to-intent p50/p95
106.95/138.17 ms. Mouse captured 69, processed 40, dropped 27, accepted zero hands;
p50/p95 111.86/143.16 ms. Neither run emitted a press. These no-accepted-hand
measurements do not estimate two-hand tracked latency or physical accuracy.
The receiving finite expiry correction is verified; camera-driven selection,
two-hand touch manipulation and real crossing/occlusion acceptance remain pending.

## Installation and original-permission reversal — 2026-10-13

The installation-time uinput snapshot included a temporary operator test grant.
An explicit original-ACL input now selects the retained pre-test baseline, while
preserving the installation-time ACL separately. It validates the single-device
baseline instead of guessing which earlier grants to remove.

The disposable network/mount namespace fixture passed injected snapshot failure,
retry, original content/mode restoration and selected ACL restoration on a fake
regular-file device. It loaded no model, started no service and touched no host
input device. Fictional artifacts exercise hash/package validation, not real ONNX.

Actual Pi receiving installation selected the retained original baseline. Rollback
removed installed WiggleControl artifacts and its udev rule; independent SSH
readback matched the original uinput ACL exactly. The independent panel remained
active. Fresh administrator access worked and SSH/sudo/access-file digests were
unchanged. Reinstallation restored the manually enabled package/rule, without
opening camera or microphone. Baseline custody and original snapshots remain
private. This qualifies real reversal/reinstallation, not human gesture acceptance.

## Whole-session resource measurements — 2026-10-13

Session summaries now include CPU accounting over model load, camera/gesture work,
the reaped device writer and cleanup, plus separate process/child lifetime RSS
peaks. CPU uses one-core normalization; peaks are neither summed nor described
as simultaneous process-tree memory. Thirty-one source tests pass, including
counter regression/unknown values and multicore CPU normalization.

Actual eight-second supervised trials on Pi 5, silent profile (1.5 GHz, 0 RPM):

| Mode | Fresh processed FPS | CPU, one-core % | Runtime peak KiB | Child peak KiB | Timestamp-to-intent p50/p95 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mouse | 8.30 | 63.91 | 45552 | 36848 | 107.06 / 141.22 |
| Touch | 8.15 | 64.29 | 46032 | 37328 | 110.87 / 142.03 |

Each captured 114 frames, processed 69/68, dropped 44 and counted zero stale
processed frames. Both accepted zero hands, so these measure repeated palm search,
not tracked-hand latency, jitter, accuracy or two-hand manipulation. Neither
emitted a press. Runtime peak is about 44.5/45.0 MiB; child peaks may include the
fork/exec transient, not steady-state writer usage. CPU accounting wall intervals
were 8.38/8.41 seconds; model load is included there, distinct from camera duration.

Temperatures progressed 71.6 → 73.8 → 75.45 C after compilation; firmware throttling
was not separately sampled in this trial. Panel status confirmed silent/0 RPM and
camera/microphone inactive afterward; the virtual devices were absent. These
dated no-accepted-hand trials do not qualify sustained thermal equilibrium or
the proposed browser-baseline comparison.
