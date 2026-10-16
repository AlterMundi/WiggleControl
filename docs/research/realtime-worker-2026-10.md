# Real-clock worker transport: first receiving results

CompAII,2026-10-15. Continues [offline provider/worker work](provider-worker-2026-10.md)
and [the transport contract](../PROVIDER-TRANSPORT.md). This is a finite public-
synthetic JPEG source on the actual Pi, not camera capture or OS input. The
initial receiving binary e2eeb17 is source-bound to ARM64 CI38819793295; the fixed one-hand
comparison uses cb901cb, CI 38823175721. Runtime/profile/calibration
and the original human trial stay unchanged.

## What actually executes

One host owns encoded-frame publication timestamps, run IDs, one-in-flight leases,
raw-result acceptance, Rust spatial tracking, the existing gesture engines and
independent ticks. The owned Chromium runs the actual GPU worker and existing
native feedback UI. Lease/result identity and clocks are checked on the host;
the browser cannot replace source/evaluation times. Models close with an explicit
acknowledgment and every finite run ends with empty contacts.

Inputs are the preceding21-frame adverse one-hand and31-frame separated-hand
references, converted on VPS with FFmpeg to640×480 JPEG. Original/JPEG digests and
conversion command are retained; fresh inference is required after compression.
Each trajectory repeats three times at the original motion timing. These are
repeated synthetic poses, not independent human trials. Geometry is independent
of predictions; processed frames are selected by actual capture index and only
their frame/time labels are mapped to the host-clock journal for scoring.

Initiald1d4265 cannot start the worker because its HTML client insertion targets
an omitted optional body-end tag. A second attempted update leaves that same old
binary installed; its receiving hash exposes the failed replacement. Both attempts
time out before model execution and retain zero captured frames. A verified
receiving hash and manifest precede the actuale2eeb17 model/transport runs below.
Do not count those preparation failures as failed hand detections or GPU timings.

## First measurements expose a functional failure

| Source | Captured / accepted / skipped / expired | Encoded frame→meaning median/p95 ms | Accepted rate |
| --- | --- | --- | --- |
| One hand,10Hz |63 /33 /29 /1 |190.2 /251.9 |6.57Hz |
| Two hands,10Hz |93 /62 /30 /1 |170.9 /217.2 |7.62Hz |
| One hand,30Hz acquisition |189 /34 /154 /1 |161.6 /212.3 |6.55Hz |
| Two hands,30Hz acquisition |279 /62 /216 /1 |135.3 /167.5 |7.55Hz |
| Prepared one hand,30Hz |189 /45 /144 /0 |161.2 /203.0 |6.95Hz |
| Prepared two hands,30Hz |279 /76 /203 /0 |133.7 /161.9 |8.03Hz |
| Fixed one-hand graph,run1 |189 /87 /102 /0 |84.5 /102.5 |13.70Hz |
| Same source,one-contact mode,two-hand graph |189 /45 /144 /0 |154.3 /197.9 |6.97Hz |
| Fixed one-hand graph,run2 |189 /89 /100 /0 |81.4 /102.2 |13.98Hz |

Accepted rate uses first→last accepted meaning timestamps, excluding the initial
cold gap. The wall source captures at its stated rate; issued=accepted+expired
and captured=issued+skipped in these cases.30Hz acquisition holds each existing
10Hz reference image for three samples, preserving motion duration; it is not
new30Hz rendered motion. The acquisition cadence matches the installed camera's
configured cadence, without activating it.

**The initial real-clock cases fail to sustain useful contact.** One-hand
processed near-hold coverage is0/7 at both capture cadences. The two-hand run has
0/13 expected dual near holds; only16 open/open stable frames match in its38
processed stable frames. At30Hz the separated pair improves to4/13 processed
dual near holds. After explicit graph preparation and the shorter listener wait
(source58414da, CI38821556768), it reaches9/15 dual near holds,5/7 near/open and
6/6 open/near stable counts. This is still incomplete. Prepared one-hand coverage
remains0/8 despite no expired issued frames. The earlier4/5 and20/20 source-time replay results remain
true within their offline scope, but do not establish a working controller.

One-hand10Hz processing changes spatial track identity23 times; every change
follows a previous source observation older than the selected300ms freshness
limit. Each new closed identity correctly waits for open rearm, so good raw gap
estimates cannot become sustained contacts. Increasing acquisition cadence alone
reduces queue age but still fails. Cold graph execution also loses the early
positive-open input before the first accepted frame. This motivates reducing
actual delay and preparing the graph, not extending freshness to hide cost.

## Next measured correction

The next source candidate explicitly warms the graph on the displayed public
synthetic reference before publishing real-clock input; warmup points never enter
Rust meaning. Preparation duration stays separate. The optional extension listener
checks for work every2ms instead of10ms; existing non-extension preview cadence
stays unchanged. Additional host-issued timestamps and bounded browser stage traces
separate queue, transfer/worker, return and meaning time. The actual prepared runs above qualify execution, not general adoption. Warmup
costs about1.5seconds separately and removes the initial expiry. The listener
change is not isolated as a performance cause; steady-state latency barely changes.

Prepared one-hand medians are18.1ms queue,145.6ms issued-frame→returned points,
and2.5ms dispatch/meaning. Component medians are not additive. Browser diagnostics
measure125.2ms inner model call and135.0ms worker roundtrip. This identifies
perception/return cost as the remaining dominant lane, rather than Rust semantics.

A reference MediaPipe [streaming graph](https://github.com/google-ai-edge/mediapipe/blob/998a5d77bb3d0e06211b6e9cc1fd6094e6051bfb/mediapipe/tasks/cc/vision/hand_landmarker/hand_landmarker_graph.cc#L126)
triggers palm detection while tracked count is below configured maximum. This
source reference motivates testing an explicit one-hand graph; it does not prove
the exact compiled1.1.1 graph's cost. The optional fixture `MAX_HANDS` now selects 1
or 2, default 2, with matching worker options and host count validation. One-hand
comparison now shows an actual benefit on this reference. The controlled
comparison uses the same cb901cb binary, one-contact settings, 300ms freshness,
geometry and 30Hz publication; only the fixed detector maximum changes. Two-hand
maximum yields 0/9 accepted near holds; one-hand maximum gives 19/22 and 18/21 in
two executions, with no contacts in 42/42 accepted fully open frames in either run.
These repeats establish neither independent pose diversity nor a general latency
guarantee. The graph-source explanation remains a hypothesis about the runtime
cost; the configuration effect itself is measured.

A deliberate1000ms delay of one actual result POST after an acknowledged held
contact releases that contact at302.739ms source age, with no intervening accepted
open observation; the expired result is discarded. The source continues, model
closes with acknowledgment and final contacts are empty. A prior interception
attempt never observes a hold and is not counted as fault-injection evidence.

The selected300ms limit, pinch thresholds, calibration and everyday runtime are
preserved. Whole-path source/app latency and broad contact continuity remain pending. No camera-ready request or physical acceptance follows
from a successful HTTP/model run. Broader negatives/crossings, image-depth/macros,
participants and actual applications remain required by the full roadmap.

## Wall-time coverage prevents an optimistic accepted-frame score

The reusable `tools/synthetic/score_provider_fixture.py` validates capture-bound
identity, host timestamp order, count conservation, successful producer completion,
acknowledged worker close and final neutral state. It reuses the existing independent
single/pair geometry scorers. It also compares actual controller output with **all**
captured stable geometry intervals, including skipped images and processing delay.
The final capture bounds the observation window. Counts do not prove ownership,
and skin proximity does not certify human intent.

The one-hand graph sustains the expected count during 968.1/1500.1ms and
978.6/1500.6ms of near geometry: about 65%, despite 86% accepted-frame coverage.
The first contacts follow each of three near-plateau starts by 151.6–191.4ms in
run 1 and 161.1–194.0ms in run 2, including perception and existing confirmation.
Neither run has contact during its captured fully open hold intervals. The
otherwise matched two-hand graph has 0/1500.3ms near coverage. Prepared separated
two-hand control reaches 1456.7/3299.9ms expected near counts and retains 286.3ms
of contact into fully open intervals; this exposes release latency that its
23/23 accepted open/open counts alone hide. These are semantic output times,
not kernel, application or physical-camera latency.

A further one-hand fault run delays one real result POST 1000ms after a held
acknowledgment. The independent controller releases at 303.779ms source age,
rejects one expired lease and finishes with acknowledged worker close and no
held state. The earlier two-hand fault remains 302.739ms. No freshness extension
or threshold change is used.

Reproduce scoring with the retained journal and its independent, acquisition-indexed
geometry (private receiving recordings are never inputs to public examples):

```sh
python3 tools/synthetic/score_provider_fixture.py \
  RUN.jsonl SOURCE-reference.json SESSION.json NEW-output-directory
python3 -m unittest discover -s tools/synthetic -p test_score_provider_fixture.py
```

The TV workbench can replay accepted results with their measured inter-result
timing; older reference replays retain their explicit 20Hz source playback. Saved
playback performs no new inference. The current one-hand source is the retained
`forward-flex-custom-v1` recipe, with index angles within the forward-only bounds
and measured distal skin clearance; the rejected dorsal solver is not reused.
Whole-hand anatomy and actual comfort remain unqualified.

Next: connect the existing timestamp-validated camera and private recorder to the
qualified lease/worker seam; prepare explicit one/two-hand graph selection and
manual mode controls without automatic capture. Broaden negatives and crossings
before interpreting this narrow one-hand gain as robust control. Then qualify
finite camera-to-input use, real applications and human comfort. The receiving
manual artifact is usable for finite camera/device-free fixtures only; it is
not an enabled everyday camera controller or boot service.

## Camera-source preparation and terminal visibility — 2026-10-16

Source18674ef connects the existing driver-monotonic V4L capture, private raw
recorder, controller modes and independent native writer to this worker seam.
Sourceb6cbc18 distinguishes driver-stamp timing from encoded fixture publication.
The new finite camera command and sensor/device-free preflight are documented in
[the transport contract](../PROVIDER-TRANSPORT.md#explicit-finite-camera-source).
It requires private recording, explicit graph capacity and a fresh human ready
acknowledgment; system input remains a separate explicit option. Original runtime,
profile/calibration, power selection and earlier prepared trial remain intact.

Actual receiving preflight checks the saved calibrated workspace, explicit0.5
provider floor, public warmup/SDK/model assets and existing device access without
acquiring sensors or creating an input device. The code reuses recording/v1;
synthetic graph warmup never becomes a camera frame or human annotation. This
is preparation evidence, not a successful physical stream or application test.

A deliberate nonexistent camera path then exercises the actual Pi GPU worker
and live-source error path. It retains zero driver/published frames, a real
producer error, neutral final input and acknowledged worker close. Its first
terminal DOM instead says reconnecting: worker-close acknowledgment can end the
listener before the preview polls its terminal metadata. Holding only that
POST acknowledgment1000ms on the sameb6 source makes the correct SESSION ERROR
view persist. This isolates a visibility/lifecycle race; it is unrelated to the
earlier delayed-result held-contact watchdog test.

Correction4385fdd (ARM64/native/replay CI38827867803) adds a run-scoped terminal
view acknowledgment after two animation frames. The listener waits for both
view and worker-close receipts, bounded by2seconds, after camera/input teardown.
A stale or pre-publication acknowledgment is rejected. Missing view receipts
stay explicit and do not become observed human perception. No new capture follows.

Actual source-bound ARM64 receiving verification reproduces the nonexistent-
camera error **without** interception, with terminal_visible=true, worker_closed=
true, zero frames/contacts/input and retained SESSION ERROR / camera not acquired
feedback. The displayed page fits1920×1080 without scroll. The normal public-
synthetic regression captures189/accepts88/skips101/expires0, at13.93Hz and
80.576ms median/101.748ms p95 encoded-publication→meaning. Full-source near
coverage is984.949/1500.656ms (65.6%), with zero open-hold contact time. Both
terminal receipts are true and final contacts empty. This confirms the new
source/recording/writer connection does not remove the narrow fixture gain;
it does not establish physical hand accuracy or OS/application delivery.

The manual experiment artifact is usable for finite synthetic qualification and
preflight, and prepared for a fresh human-directed camera trial. It is not a
selected everyday controller or boot capture service. Broad negatives, crossings/
ownership, visual macros/image-depth, participant roles, continuous GPU use and
human/application comfort remain required by the full roadmap.
