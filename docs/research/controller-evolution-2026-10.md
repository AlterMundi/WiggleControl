# Controller evolution: current candidates and first temporal benchmark

Sources inspected 2026-10-15. This is an engineering comparison, not a claim to
have exhaustively benchmarked all current models. Current model/runtime pins and
historical qualification remain in the original [research](state-of-the-art.md)
and [qualification](../QUALIFICATION.md). The active [roadmap](../ROADMAP.md)
owns implementation order and incomplete work.

## Practical candidates

| Candidate | Primary source evidence | Current decision |
| --- | --- | --- |
| Existing ONNX CPU palm/hand baseline | This repository's actual ARM64 qualification, crop invariance and pose-bias records | Keep as measured baseline; do not retune physical thresholds from joint-marker renders |
| Current MediaPipe Tasks Hand Landmarker | [Official API](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker/python) supports multiple hands, timestamped video/live modes and separate presence/tracking thresholds; video tracking avoids repeated palm detection | Useful paired backend comparison; no performance improvement assumed from changing wrapper/model |
| WiLoR | [Official repository](https://github.com/rolpotamias/WiLoR) publishes reconstruction code/checkpoints and a March2026 fast option. Its documented tested stack is PyTorch/CUDA, with external MANO assets and distinct licenses | Offline comparison candidate after asset/access/hardware check; not a demonstrated Pi input backend |
| HaMeR | [Official repository](https://github.com/geopavlakos/hamer) publishes reconstruction/evaluation code; setup requires checkpoints and separately registered MANO assets, with a CUDA-oriented example | Useful independent reconstruction comparison; do not equate a plausible mesh with correct pinch contact or intent |
| Meta hand-tracking toolkit / HOT3D | [Toolkit](https://github.com/facebookresearch/hand_tracking_toolkit) provides dataset/evaluation utilities; [HOT3D](https://github.com/facebookresearch/hot3d) contains egocentric multi-view annotations | Reuse evaluation ideas and eligible independent data; this camera/view domain differs from a fixed consumer webcam |
| EgoForce /4DHands | [EgoForce's official2026 implementation](https://github.com/dfki-av/EgoForce) studies forearm-guided monocular egocentric reconstruction; [4DHands](https://4dhands.github.io/) studies interacting-hand reconstruction from video | Relevant temporal/interaction comparison leads; neither a measured low-latency Pi backend nor an executed candidate here |
| 1€ cursor filter | [Authors' description and implementations](https://gery.casiez.net/1euro/) explicitly trade slow-motion jitter against high-speed lag | Independently implemented as opt-in Rust filter; compare against the existing timestamp-based exponential baseline |

This table supports an incremental local-first decision. It does not claim that
WiLoR/HaMeR were executed here, that CUDA is available on the Pi, or that monocular
estimated metric landmarks are calibrated physical measurements. Newer dataset
or avatar releases alone do not establish a usable low-latency controller.

## Reproducible engine benchmark

`wigglecontrol-simulate NEW_DIRECTORY` exercises the actual gesture engine with
an independent analytic1920x1080 cursor target, scheduled pinch, bounded seeded
observation noise and irregular source times. It needs no camera or uinput.
The first retained revision used sixteen profiles × three rates × three noise
amplitudes × three seeds =432 cases. Seeds11/29 were development candidates;
101 was initially retained separately. Later iterations inspected that seed, so
it is now development data. This is a controlled benchmark, not a broad
unseen-scene validation set. The current tool's expanded revision is described below.

Fixed filters use0/30/60/100ms time constants. Adaptive filters use minimum
cutoffs0.5/1/2Hz, beta2/5/10/20 in normalized camera units, derivative cutoff1Hz.
All432 cases produced exactly one scheduled press and one release. Rate affects
onset delay independently of cursor smoothing: the held-out noisy10Hz case has
101.7ms press delay versus51.7ms at 20Hz, because confirmation needs a later frame.
These are source-time delays, not camera-to-screen latency measurements.

Selected held-out results at 20Hz, normalized uniform noise amplitude0.009:

| Profile | Stationary RMS px | Moving RMS px |
| --- | --- | --- |
| Existing exponential30ms | 9.52 | 12.47 |
| Exponential100ms | 5.48 | 43.27 |
| Adaptive0.5Hz beta20 | 4.32 | 18.95 |
| Adaptive1Hz beta10 | 4.57 | 26.04 |
| Adaptive1Hz beta20 | 5.10 | 17.77 |

The adaptive0.5Hz/beta20 candidate reduces stationary error by about 55% against
the existing30ms baseline in this case, at the cost of increased moving error.
It improves both measures against the fixed100ms alternative. At10Hz it gives
6.78/19.08px, versus11.40/11.34px for the30ms baseline. There is no universal winner
and no deployment selection yet. Temporal image noise, camera timing, pinch-induced
movement and seated preference still need measurement. The live default remains
the existing filter; `adaptive_smoothing` is optional and absent in old configs.

The retained report includes every row and selected full trajectories. The TV view
shows measurements and a separately labelled saved replay, not fabricated work.

## First rendered temporal vision sequence

The MHR renderer now supports a 25-frame right-hand-only rigid motion with lateral
translation and yaw, source timestamps at 20Hz, unchanged open articulation and
invariant113.184mm rig joint-tip separation. Images remain640x480 with the pinned
CPU Blender setup. Geometry source, projection and per-frame markers are retained.

`sequence-check SEQUENCE CONFIG` runs those consecutive images through the actual
multi-hand perception and direct-contact meaning, without opening a camera or
input device. This differs from repeatedly cold-loading isolated images. Source
time is simulated; processing wall time is measured separately. Missing frames,
ordering and path bounds are explicit in the manifest contract.

First result:25/25 frames detected, one track ID, zero ambiguity reports, zero
contact frames. All rig markers remained in frame; actual source frames were
visually inspected on the receiving view. Debug-build processing median was
159.83ms on the VPS during this run. This is not a Pi throughput result and does
not establish real-time20Hz processing. Projected rig-palm versus observed cursor
RMS was23.07 output pixels; joint-marker anatomical correspondence remains a limit.

This is an open-hand tracking scenario. It neither supplies anatomically certified
skin tips nor qualifies closure/contact detection, two-hand crossing or physical
comfort. Those are the next benchmark layers. No live config, calibration,
production model or receiving runtime binary was replaced by these experiments.

## Two-hand crossing and fixture correction

A mirrored independent rig now crosses the first hand in image space. The first
25-frame trial used 25mm depth separation without checking skin overlap. It was
retained as an unqualified geometric stress case:24frames with any detection,
16with two detections, one ambiguity and four observed track IDs. Do not treat
its shape/contact behavior as a natural two-hand ground truth.

The corrected trial uses 140mm separation and checks the evaluated masked skin
vertices' camera-depth intervals at every frame. Those intervals do not overlap,
so the two meshes cannot interpenetrate. In the first frame the ranges are
0.5840..0.6658m and0.7240..0.8058m. Actual first/middle/final images were inspected;
the occluding hand is visible and both separated hands remain recognizable.

Corrected result:25/25 frames with a detection,15/25 with two detections, no
ambiguity reports, three observed track IDs, no contact frames; VPS debug median
processing175.59ms. The rear hand disappears for ten frames around occlusion and
returns with a new ID. An ID count alone does not prove a switch, and nearest
reference-palm assignment is indeterminate when projections are closer than the
localization error. The current behavior cannot be called continuous two-hand
control through occlusion. Open-hand renders cannot prove held-contact ownership.
Retain the missing interval as a benchmark case for the next perception iteration.

This fixes a concrete fixture uncertainty; it is not a rerender to animate the TV.
Raw first/corrected scenes and predictions remain separately retained.

## Macro semantic core

The typed named-pose macro engine and bounded `macro-replay` command are now
implemented. A confident neutral hold must arm the engine, a deliberate held
configured pose produces one named action intent, and another neutral hold plus
cooldown is required. Tracking loss, contact activity, ambiguity, identity/time
failure cancel the candidate. Unit scenarios cover those interruptions. This
core executes no actual host action; visual pose classification, mode arbitration
and the selected native action adapters remain pending. See [MODES.md](../MODES.md).

## Executed current Tasks comparison

[MediaPipe1.1.1](https://pypi.org/project/mediapipe/1.1.1/) was published 2026-10-14.
The optional comparison executed its supported Python CPU VIDEO API on the exact
same 25 corrected separated-pair images, without a camera. Package, transitive
comparison dependencies, model URL and hashes were retained. The hand-landmarker
bundle SHA-256 is `fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1`.
Python is justified here by the provider's actual API, rather than a rewrite of
its native inference stack; the ordinary Rust runtime remains unchanged.

Tasks detected a hand in 25/25 frames and two hands in 16/25, losing the rear hand
for nine frames. Its sequential VPS run had64.19ms median processing, including
image loading. The earlier Rust debug run's175.59ms is not a fair production
speed comparison; an optimized Rust build is needed before selecting on cost.
Host load also varied between sequential runs. This comparison does not prove
a substantial occlusion-coverage improvement: one additional two-hand frame in
a 25-frame single scene is limited evidence.

The API's hand array positions are not stable identities. This run therefore
does not fabricate track IDs or report zero identity switches. It also supplies
no ambiguity classification or Rust gesture/contact result; those fields stay
unknown. Choosing a different model wrapper cannot by itself solve occlusion or
participant ownership. Physical Pi performance and comfort remain unqualified.

## Optimized native cost and rediscovery

The optimized native build on the identical separated pair retained25/25 any-hand
and15/25 two-hand detections, three IDs and no ambiguity. Processing median was
30.90ms and p95 was77.80ms in its sequential VPS pass. These observations support
retaining the existing native backend as the development baseline; the original
debug timings do not justify rewriting it in another language or changing models.
They do not establish a universal speed ranking or real Pi performance.

The sequence probe can now vary the missing-hand palm rediscovery interval
independently of the live default of five frames. The controlled comparison
retains exact interval settings, observations and results. Search cadence cannot
make hidden fingers observable; evaluate both coverage and processing cost before
selecting a production change. Source/API bounds do not change the receiving config.

| Missing-hand search interval | Two-hand frames | Median processing ms | p95 processing ms |
| --- | --- | --- | --- |
| 1 frame | 15/25 | 41.77 | 106.19 |
| 2 frames | 15/25 | 43.30 | 103.84 |
| Existing5 frames | 15/25 | 30.90 | 77.80 |
| 10 frames | 15/25 | 29.48 | 76.22 |

More frequent detection did not improve coverage in this scene and increased
cost. Keep the existing default; this experiment does not justify a production
cadence change. The10-frame candidate needs a new-hand-entry benchmark before
its lower cost could justify slower discovery. The same external-load caveat applies.

## Fine motion and a second filter iteration

Revision2 added a fine0.01-unit/second trajectory beside the original fast
0.3-unit/second trajectory, plus a speed floor for increasing the adaptive cutoff.
The floor affects the cutoff, not spatial motion: tiny intentional movement
still reaches the cursor. This addresses noise-driven high cutoff at rest.
The1080-case intermediate run is retained separately.

Revision3 fixed28 profiles, three rates, three noise amplitudes, four seeds and
two trajectories, for2016 cases. Development seeds are11/29/101; the new313 seed
is scored only after selecting a profile on development data. The selector
minimizes the worst normalized rest/fast/fine RMS against fixed5/20/10px budgets,
using10/20Hz high-noise rows. This is a ranking rule, not proof of broad robustness.

Selected candidate: cutoff0.3Hz, beta40, derivative cutoff1Hz, speed floor0.05.
Its worst development budget ratio is0.994; the separately evaluated313 seed
gives0.971. Both are below one, with a narrow development margin. On that seed:

| Input | Fast-case rest RMS px | Fast moving RMS px | Fine moving RMS px |
| --- | --- | --- | --- |
| Existing30ms /10Hz | 9.88 | 11.53 | 11.58 |
| Candidate /10Hz | 2.34 | 19.43 | 6.12 |
| Existing30ms /20Hz | 8.95 | 11.82 | 9.50 |
| Candidate /20Hz | 3.55 | 18.15 | 5.57 |

All2016 scheduled pinch cases still produce one press/release. The stationary
improvement costs fast-motion error against the low-lag baseline; it is not a
dominant winner on every measure. The candidate meets these particular analytic
budgets, not a physical acceptance gate. A new scene/noise model, pinch-induced
movement, calibration and human preference remain independent tests. Neither
the selector nor the workbench writes a receiving configuration.

### Calibrated workspace invalidates the full-frame selection

Revision5 accepts a read-only calibration profile:
`wigglecontrol-simulate NEW_DIRECTORY CALIBRATION_PROFILE`. Desired screen targets
are transformed into camera coordinates before adding the same camera-space noise;
filtered observations use the actual shared workspace mapping back to the output.
The report retains the complete profile and raw camera observations for reproduction.
A fictional half-width/height workspace regression confirms exactly doubled raw
screen noise, with no error for the zero-noise unfiltered trajectory.

The selected receiving workspace spans approximately0.568 ×0.597 of the camera
frame. The previous0.3Hz/beta40/floor0.05 candidate now gives27.77px fast moving
RMS at10Hz and28.29px at20Hz on seed313, exceeding the fixed20px budget. Applying
full-frame results directly to the calibrated controller would be incorrect.

An expanded64-profile grid, three rates, three noise amplitudes, five seeds and
two trajectories yields5760 cases. Seeds11/29/101/313 are development data;
907 was held out for this expansion. The grid and original5/20/10px rest/fast/fine
budgets were fixed before scoring907. The best development ranking is now
0.3Hz/beta80/floor0.07, but its worst budget ratio is1.447: **no candidate passes
the complete development gate**. Its separate907 ratio is1.334, also failing.

| Rate / seed907 | Fast-case rest RMS px | Fast moving RMS px | Fine moving RMS px |
| --- | --- | --- | --- |
| 10Hz | 4.41 | 24.77 | 6.51 |
| 20Hz | 5.30 | 26.67 | 7.34 |

All5760 scheduled cases preserve one press/release. A repeat with the final
profile-retaining tool reproduces every score exactly. These analytic trajectories
move horizontally; vertical/diagonal motion, edge clipping and measured real
camera noise still need coverage. The0.009 synthetic camera noise is an engineering
stress assumption, not a measured human recording. Neither ranking nor failure
justifies changing receiving thresholds, calibration or the default filter.
The existing TV view now shows these calibrated results, the actual selected
trace and explicit budget failures; earlier full-frame evidence remains retained.

## Configured pinch interpretation in direct touch

Source inspection found that the live direct-touch branch converted detections
with the raw2D observation method while pointer and image-sequence replay applied
`pinch_3d`. Thus the selected3D setting could be ignored in the live two-contact
path. Session configuration now owns one conversion used by all three consumers.
Selected3D adds the estimated world-tip gap and suppresses the separate legacy2D
scroll chord; default2D behavior is preserved. Direct touch still has no wheel
semantics.

A camera-free regression deliberately makes pixel and world distances contradict:
both independently owned contacts, and the pointer, must open/confirm/hold/release
according to the configured metric. It covers both modes and absent detections.
This fixes source integration; it does not improve landmark accuracy, deploy a
new receiving runtime or establish physical multitouch acceptance.

## Articulated skin reference and human correction

The optional geometry probe uses [python-fcl](https://github.com/BerkeleyAutomation/python-fcl)
0.7.0.11 to measure the minimum triangle-surface distance between explicitly
selected distal thumb/index skin patches. The patches contain302/312 triangles,
selected from the corresponding third-phalanx/null skinning weights with a0.5
threshold. Witness points, patch definitions and package version are retained.
This measures a defined mesh region; it does not certify anatomical tip labels,
soft tissue, whole-hand collision or human intent. Intersections are recorded
separately; FCL's negative collision sentinel is not a signed penetration depth.
Analytic parallel/edge-separated/intersecting triangles and rigid transforms
qualify the distance adapter; malformed/degenerate inputs are rejected.

The first41-frame closure sequence used the old frozen solver recipe. Its
near pose had17.568mm marker separation but7.882mm distal skin clearance.
Nicolás observed that its index bent roughly90 degrees toward the back of the
hand before returning to normal extension. **That motion is an invalid human
pinch reference.** Its41/41 detections and one contact interval remain diagnostic
outputs, not natural-gesture accuracy evidence. The same recipe generated the
older intermediate/near sweeps; their numerical geometry/model records remain,
but they do not validate a natural pinch. A selected nonintersecting skin patch
and a small IK endpoint residual had failed to catch the wrong bending branch.

The renderer now uses `forward-flex-v2`, with positive index flexion, retained
recipe bounds, a signed rest-palm-plane check and a separate skin-distance probe.
The signed check rejects the retained old pose as a negative control. The new
index joints/tip move20.53/43.96/65.16mm toward the palm side at the near pose.
These are geometric safeguards, not a certificate of complete biomechanics.

The corrected41-frame open/closing/hold/opening sequence runs at simulated20Hz.
Its distal skin gap spans81.211mm to approximately2.000mm, with no selected-patch
intersections in any frame. Actual native optimized inference detected41/41,
kept one ID, declared no ambiguity and emitted one contact interval (frames16..28)
using the offline45/65mm configuration. Processing median was23.37ms on the VPS;
this is not a Pi or camera-to-screen latency measurement.

| Corrected reference stage | Skin gap mm | Rig-marker gap mm | Model joint-gap estimate mm | Contact |
| --- | --- | --- | --- | --- |
| Initial open /frame0 | 81.21 | 113.18 | 85.29 | No |
| Half closure /frame12 | 44.84 | 65.47 | 62.35 | No |
| First emitted contact /frame16 | 2.00 | 16.50 | 32.43 | Yes |
| Near hold /frame20 | 2.00 | 16.50 | 34.43 | Yes |
| Released while opening /frame29 | 54.46 | 78.51 | 68.31 | No |
| Final open /frame40 | 81.21 | 113.18 | 85.63 | No |

The TV followed the actual finite render and inference, and retains the corrected
sequence/replay with skin clearance, estimated model gap and contact count.
Raw output and the rejected sequence's human review stay private. No camera,
microphone, OS injection, receiving binary/configuration or live calibration
changed. Human plausibility/comfort, different views, self-occlusion, whole-hand
collision and real intended actions remain independent acceptance work. This
single successful synthetic contact interval does not prove the full controller.

## Exclusive mode arbitration and replay

The new Rust controller reuses the existing pointer/contact/macro engines behind
one explicit mode selector: pointer, reduced-gain precision, explicit scroll,
one contact and two contacts. Mode changes release old output before accepting
new gestures; closed pointer reacquisition cannot move or click until opening.
Named macro intent can change the local mode or pause, with normal pinch input
taking precedence. Other actions remain unhandled requests. This is semantic
integration, not a deployed native device or visual macro classifier.

Behavioral tests cover switching while dragging or holding two contacts,
rearm, pointer/scroll exclusivity, precision gain, sticky/manual track selection,
clock regression, stale/future/duplicate/malformed input, macro provenance and
same-frame action arbitration. CLI checks retain actual release frames on EOF
and malformed/oversized messages without devices. Testing exposed a baseline
initialization bug: acquiring a hand directly in scroll cleared its scroll state
after setting it, discarding the first displacement. Identity now initializes
before scroll mode; pointer, loss and filter regressions pass.

The41 retained observations from the corrected central closure were also replayed
through the new CLI with the offline45/65mm gesture settings. No images were
rerendered and no perception inference was repeated. Pointer and precision each
produced one press/release; the precision movement path was exactly0.25 times
the pointer path. One-contact and two-contact modes each retained13 contact
frames, because this input contains one observed hand. That is not a two-hand
vision qualification. A pointer→precision change during the hold emitted the
release and blocked further movement/press until opening; no duplicate click
occurred. All cases ended neutral at EOF.

The owning contract and command examples are in [MODES.md](../MODES.md).
The following increment now routes opt-in native session controls and independent
device writers through this arbiter. Receiving real-action qualification remains
pending; see [CONTROLLER-NATIVE.md](../CONTROLLER-NATIVE.md).
Absolute precision mapping, participant roles and visual pose recognition remain
separate incomplete work. No receiving runtime, sensor, input or calibration
changed in this source-only increment.

## Native control-to-event integration

The opt-in native controller entry points now reuse the existing camera, recording,
preview and teardown loop. A same-origin queue handles mode, hand claim and
input pause/resume. Settings must agree with the session's gesture/workspace
configuration before inference/capture. Manual control time fences prevent a
queued pre-control open observation from rearming a now-closed hand.

The independent writer owns the existing evdev mouse/touch encoders and devices.
Mode transitions release both; frames require matching mode, monotonic sequence,
fresh issued/source times and one compatible input stream. Watchdog expiry
latches input off until another transition. Full pipeline replay serializes and
decodes actual packets and emits actual Linux event codes without opening uinput.
The tests include two-slot removal, cleanup after malformed input, clock/source
integrity and recovery from a stalled producer without replaying held contacts.

The native page was tested on the actual Cuevanico browser with the device-free
Rust pipeline and the corrected synthetic reference. Five modes, explicit hand
selection, switching while held, open rearm and pause/resume were observed.
1920x1080 retained all controls with scroll height1080/footer1056 and no console
exceptions;390px had no horizontal overflow and21px control text. The view clearly
said UI TEST / sensors off / system input off. Delayed feedback no longer presents
cached hold state as current. The temporary fixture/server was stopped and the
owned browser restored. No camera, microphone, virtual device, production binary,
threshold, calibration or power change occurred.

The first fixture verified source integration, protocol meaning and UI operation.
The later separately staged ARM64 candidate2f63f29 passed98 native tests and its
source-bound CI build, then help/preflight on the Pi. A finite actual combined
writer test delivered trusted mouse button release on mode change, two contacts
(client scale1.50438/rotation4.33389degrees), touch release on transition and an
independent watchdog release. Devices were removed and the workbench restored;
sensors were observed inactive. Selected original binaries/config/profile and
silent power remained preserved. Details and exact candidate provenance are in
[CONTROLLER-NATIVE.md](../CONTROLLER-NATIVE.md#receiving-candidate-checkpoint-2026-10-15).

The first stationary mouse press after touch did not reach the client; fresh
pointer movement made the repeated press/release work. The subsequent selected
local compositor patch now repairs all five controlled stationary cases in the
owned Wayland Chromium client; see [the current receiving evidence](../CONTROLLER-NATIVE.md#main-desktop-follow-up-2026-10-15).
Other clients/locks/grabs remain in#5. The initial test generator's
accidental watchdog timeout was corrected with explicit fresh synthetic test
frames and a deliberate stall. The existing named uinput grant needed manual
restoration after boot;#4 tracks persistent startup/upgrade/rollback. Live native
camera control, physical accuracy/comfort, absolute precision and visual macros
still require their own qualification and fresh acknowledged human trials.

## Reuse and research curation

The distinction between marker distance, selected skin clearance and plausible
motion was selected for collective research with the retained human correction.
The record also relates actual kernel/client handoff evidence. Operation
`controller-evidence-corrections-20261015-v1` was inspected and accepted through
the configured CompAII route, then published in curated index/UI generation5.
Exact document bytes, targeted search, corrected map and Atlas graph were verified.
The logical document is
`research/wigglecontrol/compaii/controller-evidence-corrections-20261015-v1.md`;
the shared discovery/client resolves the authorized reader. This is availability,
not another being's reading/adoption or independent corroboration. Keep this owning repository as the
canonical technical source. A curated research record should link source/version,
independent labels, selected aggregate results, failure corrections and limits;
private footage, receiving routes and native conversation history stay private.
Publishing a report does not prove collective-research ingestion or deployment.

## Articulated view comparison —2026-10-15

The forward-flex reference was expanded from its retained central sequence to
four fixed views: yaw±35° and pitch±25°. Every view has the same41-frame
articulation/hold schedule at20Hz simulated source time. The render keeps the
hand-only mask and independently measures distal skin clearance, full-skin image
bounds/depth and signed forward joint displacement. It does not use model
predictions to define the reference motion. All164 new frames retain the whole
selected skin in frame, without distal-patch intersection; the minimum gap is
approximately2mm and the open gap81.21mm. Full framing and those patches do not
certify hidden fingertips, whole-hand collision or human joint limits.

Each sequence was processed by the existing optimized native Rust/ONNX binary
(SHA256 `3b0351aa218b5593380fed66e601df417aa79295a8c2b23df01695ee84e701ba`)
and direct-touch meaning. The3D configuration retained45/65mm hysteresis.
The2D comparison changed only `pinch_3d` to false, keeping the existing default
0.25/0.4 normalized tip-gap ratios. All164 matched raw perception arrays were
identical between interpretations; only gesture interpretation differed.

| View | Detected / frames |3D contact interval, inclusive frame indices |3D contact during maximum-closure hold |2D contact during that hold |3D hold spread, RMS px |
| --- | --- | --- | --- | --- | --- |
| yaw−35° |41/41 |17–27 |8/9 |0/9 |0.95 |
| yaw+35° |41/41 |16–28 |9/9 |0/9 |0.20 |
| pitch−25° |41/41 |15–29 |9/9 |0/9 |1.33 |
| pitch+25° |41/41 |15–27 |9/9 |0/9 |1.48 |

Every3D view retained one track ID and one contiguous contact interval; the2D
baseline produced no contact in any of the164 frames. Neither interpretation
produced contacts in the72 fully open hold frames. Maximum-closure holds are
frames16–24, defined by the renderer. The central retained3D comparison was9/9,
contact interval16–28; it was reused, not rerendered as an independent replicate.
Hold spread is about the emitted-contact mean on an explicitly1920x1080 full
workspace; it includes filter settling and excludes frames without a contact.
It is not calibrated absolute accuracy, physical camera jitter or end-to-end
latency. Onset varies by two source frames (100ms) across the four new views.

Estimated3D joint gap during the same maximum-closure pose ranges19.67–35.88mm
across these views despite approximately2mm selected skin clearance. Different
definitions and view-sensitive inference remain visible. This experiment supports
retaining the current3D interpretation for further testing rather than reverting
to the default2D baseline; it does not justify a universal scale correction or
tuning live thresholds. Only one rig, lighting/background and closure recipe
were tested; no human intent labels, varied hands or physical comfort were acquired.

Reproduction: `render_hand_reference.py --hand-only --closure 1 --closing-steps 41
--skin-check --closing-view CASE`, followed by the existing `run_visible_motion.py`
with an explicit config/label. `summarize_closure_views.py` checks matching frame
order/source time and retains source-report hashes. The private comparison
SHA256 is `706a1e3cbf0db620867dc1738d8b486773efbc0857c2c0a49ce660fc2e0641b1`;
source/config/binary hashes and editable renders remain with the evidence.

Actual render/inference stages and view labels were projected to the owned TV
workbench. Starting a new run now clears the preceding run's final counters and
disables its saved replay until new measurements finish. Final2D results remain
at1920x1080 with no page overflow and complete images. All producers/transports
ended; no camera, microphone, OS input, saved calibration or live controller
configuration changed. The collective-research correction source remains the
broader evidence-method entry point; this detailed view increment stays in its
owning technical report pending the next coherent shared synthesis.
