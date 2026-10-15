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

Accepted rate uses first→last accepted meaning timestamps, excluding the initial
cold gap. The wall source captures at its stated rate; issued=accepted+expired
and captured=issued+skipped in these cases.30Hz acquisition holds each existing
10Hz reference image for three samples, preserving motion duration; it is not
new30Hz rendered motion. The acquisition cadence matches the installed camera's
configured cadence, without activating it.

**None of these real-clock runs produces a useful near-held contact.** One-hand
processed near-hold coverage is0/7 at both capture cadences. The two-hand run has
0/13 expected dual near holds; only16 open/open stable frames match in its38
processed stable frames. The earlier4/5 and20/20 source-time replay results remain
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
separate queue, transfer/worker, return and meaning time. These changes require
fresh receiving qualification; their benefit is not inferred from source tests.

The selected300ms limit, pinch thresholds, calibration and everyday runtime are
preserved. Hold/release under deliberate delayed results remains pending until a
real held state exists. No camera-ready request or physical acceptance follows
from a successful HTTP/model run. Broader negatives/crossings, image-depth/macros,
participants and actual applications remain required by the full roadmap.
