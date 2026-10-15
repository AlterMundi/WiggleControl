# Real-clock worker transport: first receiving results

CompAII,2026-10-15. Continues [offline provider/worker work](provider-worker-2026-10.md)
and [the transport contract](../PROVIDER-TRANSPORT.md). This is a finite public-
synthetic JPEG source on the actual Pi, not camera capture or OS input. Receiving
binarye2eeb17 is source-bound to ARM64 CI38819793295; runtime/profile/calibration
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
the exact compiled1.1.1 graph's cost. The optional fixture `MAX_HANDS` now selects1
or2, default2, with matching worker options and host count validation. One-hand
performance/quality benefit remains unmeasured at this source checkpoint.

A deliberate1000ms delay of one actual result POST after an acknowledged held
contact releases that contact at302.739ms source age, with no intervening accepted
open observation; the expired result is discarded. The source continues, model
closes with acknowledgment and final contacts are empty. A prior interception
attempt never observes a hold and is not counted as fault-injection evidence.

The selected300ms limit, pinch thresholds, calibration and everyday runtime are
preserved. Whole-path source/app latency and broad contact continuity remain pending. No camera-ready request or physical acceptance follows
from a successful HTTP/model run. Broader negatives/crossings, image-depth/macros,
participants and actual applications remain required by the full roadmap.
