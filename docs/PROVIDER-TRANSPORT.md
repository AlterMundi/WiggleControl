# Worker transport qualification

The experimental `provider-fixture` command connects a finite JPEG source, the
existing loopback feedback server, the pinned browser GPU worker and actual Rust
tracking/meaning. It opens no camera or virtual input device. It is not a live
accepted receiving runtime. The explicit camera source below shares its transport;
physical camera-to-input acceptance remains separate.

```sh
wigglecontrol provider-fixture SEQUENCE.json SESSION.json SETTINGS.json ASSET_ROOT PORT [MAX_HANDS]
```

Build with `native-vision`; native ONNX inference is not initialized by this
command when visual macros are disabled. Existing session/controller settings
must match, select a single estimate and an explicit gesture confidence/SDK
presence floor (the retained comparison profiles use0.5). Optional measured
presence is described below. Optional visual macros
are described below. Retained
3D/2D pinch interpretation, workspace, manual modes, open rearm and Stop remain
the existing controller's responsibility. No gesture threshold/freshness is
silently changed by transport.

`MAX_HANDS` explicitly selects1 or2, default2, for the fixed model graph of this
experiment. It is not automatic mode switching or participant identity. A
one-hand comparison should select the existing one-contact mode in settings.
The host rejects returned hand counts above the selected graph capacity.

The sequence uses the existing version1 image manifest, with relative regular
640×480 JPEG files, at most2MiB each, and source offsets covering at most90seconds.
Prepare compression before the run and retain original/JPEG hashes plus independent
geometry. Compression changes pixels, so actual worker inference must run afresh.
After model-ready acknowledgment the finite producer publishes at those offsets
using the host's actual monotonic timestamp after reading each encoded frame.
This measures **encoded-frame publication→meaning**, not camera exposure→OS event.

Open `/provider/view` in the explicitly owned receiving browser. It reuses the
native feedback/mode/Stop page and labels the source synthetic. Model/SDK assets,
the classic worker and client share this loopback origin. Preparation is bounded
30seconds; model inference has the existing30second worker-request bound; the
source has a finite completion bound and ends with a release. Existing native
camera commands and their human-ready/recording requirements remain separate.

`ASSET_ROOT/provider-assets.json` contains `package_version:"1.1.1"` and a `files`
map of relative names to SHA256. Include the qualified `hand_landmarker.task`,
`package/vision_bundle.mjs` and selected files directly under `package/wasm/`.
Preflight verifies each bounded regular file with the existing Linux `sha256sum`
tool. Explicit receiving-local public-asset symlinks are supported; the server
serves only the verified allowlist. Model/SDK binaries stay outside Git with their
licenses and original manifest. The model digest also gets checked in the browser.

Rust owns acquisition timestamps, dimensions, a per-process monotonic run ID and
a monotonically increasing lease number. `/provider/frame` leases the latest
published JPEG; only one is in flight. The client returns `{run_id,sequence,hands}`
to `/provider/result`, with raw pixel/world
points and a separate handedness score. It cannot submit a replacement source or
evaluation timestamp. Result acceptance consumes the exact lease once and waits
for the actual meaning receipt. Old leases expire; the source continues replacing
its latest slot, and skipped/expired/processed counts remain separate.
Provider readiness, cleanup Stop and model-close acknowledgment also carry the
run ID. An old browser instance cannot complete a similarly numbered frame or
cancel/close a later run on the reused loopback port. This is data association,
not a replacement for the existing host/origin contract.

The main loop ticks tracking/meaning independently of worker execution. Mode,
hand selection and resume use existing control-time fences, so a pre-control
image cannot reactivate input. The optional HTTP extension requires the existing
Host/same-origin checks; headers stay bounded4KiB and provider bodies16KiB.
An absolute read deadline prevents partial bodies from indefinitely occupying
the listener. Existing preview routes remain available without this extension.

JSONL output separates source publication, accepted frames, intent transitions,
expired leases, manual controls and final release, with host-monotonic timestamps.
Presence stays explicitly a configured SDK threshold floor, never a measured
per-hand score or the handedness probability. The preview shows
encoded-frame→meaning duration separately from the aging last image; it does not
label that number as model-only inference. A worker-close acknowledgment is
recorded separately from finite server/process termination.

Source tests cover one-in-flight/latest-slot accounting, immutable host-time
association, wrong/repeated/expired results, actual receipt acknowledgment,
body/header/origin bounds, partial-body expiry and existing preview behavior.
Actual same-host ARM64 timing, skipped-frame quality, delayed-result release and
1080p fixture execution are recorded in [the receiving report](research/realtime-worker-2026-10.md).
Broader negatives/crossings, recognized macro sequences, participants and
physical/application acceptance remain open.

Initial ARM64 execution atd1d4265 reaches its finite worker-preparation timeout
without starting inference. Inspection of the served page finds the existing
HTML omits its optional `</body>` tag, so the client insertion never occurred.
The fixture now inserts before the actual `</html>` tag, with a served-view/client
route regression. This startup failure is not a measured GPU/transport result.

## Optional visual macros

The raw adapter accepts optional `image_depths:[Z;21]` in source-image pixels:
the worker converts normalized image Z using image width. World XYZ remains
metric and separate. Old XY/world-only packets still support existing tracking
and input meaning; absent image depth produces no visual pose evidence. Invalid,
nonfinite or unbounded depth rejects the observation and retires its track.
The native detector retains exactly its previous numeric depth values.

Provider commands now reuse the native `visual_macros` settings and the same
classifier, ownership, positive-neutral, pinch-priority, hold and cooldown rules.
`gesture_min_confidence` explicitly selects the worker's actual
`minHandPresenceConfidence`; it must match controller settings and meet the
visual policy's confidence threshold (at least0.8), unless explicitly selecting
`provider_presence_scores:true`. In that mode, validated same-frame measurements
can meet the visual gate while the tracking floor stays0.5. Without that mode,
a0.5 profile with0.8 visual policy fails before acquiring anything. Raising the
SDK floor may reduce tracking
coverage; no default or everyday profile is silently raised.

Configuration preflight does not load ORT or the classifier. Actual enabled
execution loads the selected small classifier through the existing ORT library,
without another hand detector; disabled execution remains model/runtime-free.
Raw pose/category/score, usable evidence and `classifier_ms` are reported in
frame journals and feedback. Classification time is included before the actual
controller evaluation timestamp, so inference cannot extend source freshness.
Only existing local actions are implemented; an external media/launcher intent
is not evidence of application delivery. The conditional four-tensor parity
check is documented in [the macro integration report](research/provider-visual-macros-2026-10.md);
image-sequence, receiving and human acceptance remain distinct.

## Optional measured presence

`provider_presence_scores:true` selects the hash-pinned experimental observer in
[provider_presence.mjs](../tools/synthetic/provider_presence.mjs). Disabled profiles
do not install it. The worker checks the exact SDK bundle, discovers the three
matching internal score/presence/pre-deduplication landmark streams and exports
them through the SDK's original graph setup. It reuses inference and SDK parsing;
it neither patches the SDK file nor runs another detector. This internal bridge
is qualified only for the pinned bundle and fixed1/2-hand graph lifetime.

Scores are filtered by actual presence flags, then associated to final SDK hands
by exact same-frame serialized landmark correspondence through deduplication.
Ambiguous/missing/malformed/stale quality becomes unavailable; it never supplies
another hand's score. Optional raw `presence_confidence` is accepted only with
this profile and must be finite and within `[configured_floor,1]`. Absent quality
keeps the explicit floor, which at0.5 cannot satisfy the unchanged0.8 macro gate.
Pinch/input confidence and classification confidence retain their existing rules.

Preflight still performs no graph/model acquisition; it does not qualify this
internal bridge. An unsupported bundle/graph fails actual preparation before
worker-ready/source capture. Frame journals retain measured-score versus floor
provenance. See [the evidence and remaining gates](research/provider-presence-scores-2026-10.md).

## Explicit finite camera source

The same bridge now supports the existing V4L camera producer, private raw recorder,
controller modes and independent native controller writer:

```sh
wigglecontrol provider-camera-preflight SESSION SETTINGS ASSET_ROOT SYNTHETIC_WARMUP_JPEG SECONDS PORT MAX_HANDS [--inject]
wigglecontrol provider-camera SESSION SETTINGS ASSET_ROOT SYNTHETIC_WARMUP_JPEG SECONDS PORT MAX_HANDS [--inject]
```

Preflight validates the matched policy, pinned assets and synthetic JPEG without
opening a listener, camera, recorder or input device. It does not establish camera
availability, writer permissions or physical accuracy. Duration must be1..90seconds;
recording_directory is optional unless a finite dataset sample is explicitly enabled; see [recording policy](RECORDING.md). MAX_HANDS is explicit1/2 for the entire session.
The live command requires a fresh human ready acknowledgment after receiving
preparation; --inject additionally enables the existing system writer for that
coordinated desktop trial. Omitting it retains local meaning/feedback only.
There is no startup service or implicit camera activation in either fixture or
preflight. Original native workflows remain supported.

The owned `/provider/view` warms the graph only through `/provider/warmup`, an
explicit selected synthetic JPEG with its own response marker. It never warms on
the current preview/camera image. Only after worker-ready does the producer start
640×480 MJPEG/30 capture with the existing driver-monotonic timestamp validation. Only an explicitly
requested finite dataset sample records raw frames before the latest mailbox
can drop them; ordinary operation and calibration do not record video; publication/kernel-frame counts and drops remain distinct.
The worker consumes the latest available frame when ready, rather than applying
the legacy synchronous config.fps throttle. No driver timestamp is replaced by
browser, receipt or decode time. Independent camera and writer deadlines bound
the finite trial even while model execution is delayed.

The shared native writer receives new source stamps only for new accepted model
results; tick-only releases and manual controls use transition packets without
replaying an old source. The existing source-order/mode checks and separate
watchdog remain authoritative. A semantic HTTP receipt is not proof of OS or
application delivery. Stop/deadline/error releases input, closes the writer and
camera, gives the worker a bounded close-acknowledgment opportunity and retains
final neutral/error feedback. The page distinguishes a synthetic warmup image
from actual camera/recording state and retains the terminal frame without
pretending inference continues.

Private trial artifacts reuse recording/v1: raw MJPEG, kernel sequence/time index,
observations/control journal, original session/controller settings and a manifest
with backend/model/threshold attribution. Human annotations remain separate.
Visual macros/image-depth remain unavailable on this provider; the shared
configuration rejects a visual classifier rather than manufacturing image Z.
A receiving missing-camera test exposed a terminal-display race: immediate worker
close could end the listener before the page read the final error. Delaying only
the close acknowledgment reproduced correct retained feedback. The page now
acknowledges the terminal run ID after two animation frames; the listener waits
for both that view receipt and worker-close acknowledgment, bounded by2seconds.
This wait occurs after camera/input teardown. Missing/stale view receipts remain
explicit in the final journal; they never authorize another capture or imply
human perception.

Source tests and a preflight are preparation evidence only. A real camera trial,
input delivery, application behavior and comfort must be measured before selecting
this backend as the receiving default.

## Retained camera recording replay

```sh
wigglecontrol provider-recording DIRECTORY SESSION SETTINGS ASSET_ROOT SYNTHETIC_WARMUP START_MS DURATION_MS PORT MAX_HANDS
```

This camera/device-free path reads an explicitly selected recording/v1 directory
at640×480. DURATION_MS is1..90000; START_MS is relative to its first original
frame. The existing raw recorder and annotation player now share one bounded
indexed reader, including rotated MJPEG segments, exact byte ranges and original
timestamps/kernel sequences. Replay reads original compressed bytes, without
re-encoding or modifying video, indices or labels. No previous input actions
are replayed into the system.

Only the selected window is published, preserving its recorded inter-frame
spacing and gaps. Original index and `recorded_frame` remain explicit in the
publication/accepted-frame journal; the latter retains original source_us,
kernel sequence, segment, offset and length. The new `source_us` is the receiving
host's monotonic time after reading the encoded frame. Old and new monotonic
clocks must not be compared as physical latency; they can belong to different
boots. Measured delay is **replay publication→meaning**, excluding the original
exposure/acquisition and OS/application output.

Model preparation still uses the separately selected public synthetic warmup.
The page labels actual input **recorded camera playback**, shows its time in
the original recording, reports sensors/input off and retains final feedback
receipts. The original camera configuration's requested frame rate does not
replace measured index cadence. This provider has no injection option.

Existing annotations remain separate from predictions; an absent label set
is not evidence of intended clicks or pose correctness. The synthetic geometry
scorer explicitly refuses recorded-source journals. Report observed detections,
tracking, timing and held transitions separately from any independent accuracy
assessment. Private frames, recording paths and annotations stay outside public
Git and shared research; select only suitable aggregate findings.
