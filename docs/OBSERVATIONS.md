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

Events: `move` with normalized `dx`/`dy`, `button` with `down`, and `cancel` with
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
