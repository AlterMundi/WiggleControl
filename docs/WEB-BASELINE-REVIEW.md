# Web baseline versus native feedback review — 2026-10-14

Requested by Nicolás after failed native pinch/drag acceptance. Reviewed the
WiggleControl preview introduced at 5cdf9e6, the later dc8f531 visible-window
baseline, producer bb3026b, and native UI 5629b11. This reviews source and retained
receiving evidence; it does not open sensors or qualify physical interaction.

| Stage | Source comparison and finding |
| --- | --- |
| Camera acquisition | Still 640x480 MJPEG / 30 Hz V4L2, two buffers, monotonic driver timestamps and one latest-frame mailbox. Acquisition loop unchanged from initial preview. |
| Decode and scheduling | Same MJPEG decode and configured FPS throttle (receiving 10 Hz). Scheduling sets the next due time before inference. No new intentional frame queue. |
| Inference and tracking | Palm/hand ROI, confidence gate, landmark transform, and tracking source unchanged; full-hand in-view predicate extracted without changing its meaning. Receiving models/runtime match the retained selected pair; no inference dependency upgrade in the diff. |
| Gesture engine | Same entry confirmation, close/open hysteresis, engagement gate, loss/stale release and neutral reacquisition. Current close/open .25/.4 match original defaults; wider .35/.5 and .35/.45 were later trials. Diagnostics and configurable thresholds were added. |
| Calibration and output | New accepted workspace changes the output mapping, not fingertip recognition. The latest camera-preview diagnostic intentionally has no profile/device writer. Absolute native input and profile adoption are separate from the web preview's camera-pixel skeleton. |
| Web feedback | JavaScript consumes Rust pinch_state; no independent pinch recognizer. Its PINCH label checks the producer fresh flag, while separate delayed-status logic also ages metadata. |
| Native feedback | Whole-hand green checks producer fresh AND source age plus receiving elapsed <150 ms. This ages the captured frame again for display: a valid 124 ms-old held observation has only 26 ms of green display budget, even before a new release is received. It can visually flicker independently of actual held-state transitions. |
| Lifecycle and timing | New painted readiness, bounded setup/countdown and guided calibration are explicit additions. No automatic capture or input activation. Current prayer mode is off. Native overlay draws/polls more frequently and across the full output; its contribution to latency is unmeasured. |

## Retained evidence and limits

An earlier web-visible trial reported four press intents, five stale frames and
p50/p95 age 110.78/143.15 ms. Other web trials also had many stale frames, so the
baseline does not prove universal robustness. Nicolás nevertheless reports much
better practical recognition there, which is the acceptance reference.

Latest native camera-only trial: one press intent, ten pressed frames,
stale_frame43, tracking_lost33, invalid_hand2 (palm_width), open_before_press39;
p50/p95 123.828/154.038 ms. These aggregates establish failed held acceptance
and a timing conflict with the unchanged 150 ms source cutoff. They do not
identify each release cause or prove that the renderer caused the timing change.
Previous invalid_hand72 cannot be attributed from this later two-frame diagnosis.

## Repair direction

Preserve the single shared recognizer rather than copying a renderer as gesture
code. Separate source-frame freshness, the recognizer's published held state and
feedback lifetime. A retained skeleton or flash must not keep OS input pressed;
a consumer should not independently reinterpret release as a different gesture.
Expose bounded actual release reasons/times and compare the earlier/current
producer on the same fictional observations and pinned inference reference.
Then compare finite physical runs under the same config, hand placement and
workload, waiting fresh human ready. Resolve sustained pinch before adding more
UI gestures or changing several thresholds at once. Preserve accepted calibration,
original executables and silent mode. No threshold/performance change was made
by this review.

ProyectoRPi/Barehands is a separate browser recognizer and model pipeline; its
pinch/pose/hold policies and AGPL provenance are relevant additional reference,
not the implementation that supplied this WiggleControl web preview.
