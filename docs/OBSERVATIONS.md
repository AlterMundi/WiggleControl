# Observation and replay contract v1

`wigglecontrol replay` reads bounded JSONL commands on stdin and emits JSONL event
intent on stdout. It does not open a camera or input device. Commands are `start`,
`stop`, `tick` with `now_us`, and `frame` with `now_us` and `observation`. Unknown
fields are rejected. Each input line is at most 16 KiB; no frame queue is retained.
EOF closes the manually enabled session and releases held state. Malformed input
cancels current interaction; oversized input cancels and exits unsuccessfully.

Observation fields: `version: 1`, `source_us`, `hand` (object or null). Time uses
integer microseconds from a shared monotonic epoch; never Unix wall clock or
assumed FPS. Native producers must translate their clock to that consumer epoch.
`source_us` must increase, be no later than consumer `now_us` and be younger than
150 ms. `tick` is the explicit watchdog clock; production integration must call
it even when a producer stalls. Frame age is validated separately from the
150 ms interval since the last accepted delivery, so a steady stream of fresh
frames with real processing latency does not reset a drag on every update. The replay executable alone is not that live
watchdog and cannot measure real camera latency.

Hand fields: stable `id`, finite `confidence` in [0,1], explicit `engaged`,
`pinch_eligible`, `cursor`, `thumb_tip`, `index_tip` and positive `palm_width`.
Optional `pinch_onset_allowed` defaults to true (and is omitted when true).
False vetoes a new press after raw-metric confirmation without cancelling open
rearm, restarting confirmation, freezing the cursor or releasing an existing
hold. Loss, invalidity and unknown identity still cancel normally. This is an
onset permission, not an alternate distance or proof of intent; a late allowed
sample can admit a previously confirmed raw close. The opt-in native screen
producer and measured limits are documented in
[the onset-veto report](research/pinch-onset-veto-2026-10.md).
Optional `secondary_pinch` carries one matched same-hand/frame metric hypothesis:
`source_id` (estimator identity), `source_us` (exact parent timestamp),
`confidence`, `eligible`, `distance_m` and optional `onset_allowed` (default true).
Distances must be finite in [0,1] meters and confidence in [0,1]. Invalid or
wrong-frame evidence cancels the observation. Missing, weak or ineligible
secondary evidence resets only that estimator; loss/invalidity of the primary
hand cancels both. An estimator-ID change cannot inherit its predecessor's
open rearm, confirmation or hold. Each estimator needs its own positive open
and continuous close confirmation. Their held states emit one button/contact
per hand, with one primary cursor and no extra participant. A hold releases
when neither estimator holds; an open primary alone cannot release a secondary
hold. Existing loss, stale, Stop and mode/identity cancellation still apply.
This union can prolong a wrongly estimated hold; the onset veto does not solve
release conflicts. Controller pointer readiness accepts any usable open estimate;
macro neutrality requires every usable estimate to be positively open.
See [implemented fusion and qualification](research/native-pinch-fusion-2026-10.md).
The secondary field is omitted when absent, preserving legacy serialized input.
Both estimates use the engine's explicit `min_confidence`; scores from different
models are not automatically comparable or rescaled. Experimental native sessions
can set `gesture_min_confidence` (default remains0.7) and a `secondary_model` with
an absolute `path`, its explicit `min_presence` and distinct `source_id` above1.
Selection requires dual3D and a compatible engine threshold; preflight rejects
contradictory thresholds. Primary native inference retains its0.8 presence filter.
See [model/crop isolation and actual source qualification](research/fixed-tensor-mixed-model-2026-10.md).


Older strict readers do not accept new secondary evidence or the explicit false field; keep such
observations within a source-compatible experimental pipeline. No receiving
binary upgrade is implied by the default omission.
Cursor points use normalized display-workspace x/y in [0,1], x increasing toward
screen right and y down. Fingertips use a separate square-padded camera metric,
normalized by the larger sensor dimension; palm width uses that same metric.
The producer transforms camera mirroring into this display-oriented coordinate
space exactly once. Palm width and fingertip separation use the same coordinate
metric; compensate camera aspect ratio before normalizing if needed. The initial
contract assumes a calibrated rectangular workspace and uses relative cursor
intent; it does not establish screen pixel coordinates or compositor acceleration.

The producer must conservatively establish pinch eligibility from visible,
valid finger landmarks and posture; the engine does not recognize fists from
three points. Missing/ambiguous fingers, weak confidence, disengagement and lost
tracking cancel interaction. Stable identity must come from tracking, never an
array position. A changed identity starts a new movement baseline.

A new/reacquired hand must show an open pinch before it can press. Default close
ratio is 0.25 and open ratio 0.4 of palm width. Closing must persist for 40 ms;
intermediate ratios preserve an already held button but cancel a pending press.
Moving while pressed is drag. No click is synthesized from an already closed
hand on acquisition. Open, stop, loss, invalid input and 150 ms staleness release.

Cursor smoothing currently uses a timestamp-based exponential filter with a
30 ms time constant. The fictional 10 ms jitter test reduces a 0.01 displacement
to less than 0.004. This is mathematical replay evidence, not measured human
noise/lag or a claim to implement the 1€ filter. Native qualification will compare
filter choices and cursor anchors. Scroll mapping is deliberately still pending.

Events: `move` with normalized `dx`/`dy`, legacy left `button` with `down`, and `cancel` with
`reason`. OS translation/gain belongs to the Linux adapter. Replay release events
do not prove an OS button was released. A device writer must independently own
and release its state on producer/process failure.

Try `cargo run --locked -- replay < tests/fixtures/click-drag.jsonl`.
All fixture inputs are fictional; no actual camera content is committed.


The optional v1 `scrolling` hand field defaults false for earlier fixtures.
The native producer selects it while thumb/middle are pinched and thumb/index
remain open. In this mode palm vertical movement emits wheel intent (12 steps
per workspace unit, fractional accumulation); upward motion scrolls upward.
Pointer motion is parked, held clicks release, and exit resets the pointer
baseline without a jump. This initial mapping requires human qualification.

## Explicit button delivery extension

The additive `named_button` event includes `button` (`left`, `right`, `middle`)
and `down`. It is supported by the independent delivery/direct Linux writer;
ordinary pinch observations still emit the legacy left event. Unknown event
fields/buttons are rejected. Per-button held state and all-button cancellation
are described in [MOUSE-ACTIONS.md](MOUSE-ACTIONS.md). Source/replay support is
not physical right-click gesture acceptance or receiving adoption.
