# Direct-touch intent contract

The Linux `touch-stream SECONDS` adapter is separate from the relative mouse.
It creates `WiggleControl Touch` with INPUT_PROP_DIRECT, BTN_TOUCH, absolute X/Y
and type-B multitouch slots. Sessions accept 1–120 seconds. `touch-check SECONDS`
registers the device without sending contacts. Both require the receiving uinput
grant; perception does not run as root.

The producer sends JSONL, bounded to 16 KiB per message and one queued message:

```json
{"type":"frame","contacts":[{"id":17,"x":0.3,"y":0.6},{"id":29,"x":0.7,"y":0.6}]}
{"type":"frame","contacts":[]}
{"type":"stop"}
```

At most two contacts have distinct IDs in 0..2147483647, and finite coordinates
in 0..1. Every frame is the complete current contact set. Array order does not
change slot identity; omitted IDs end their contact. Coordinates map linearly
onto the kernel's 0..65535 absolute ranges. Display mapping belongs to compositor
configuration. The receiving qualification used one 1920x1080 output; multi-output
selection is not yet qualified and must not be inferred from normalized values.

No contact frame or explicit heartbeat for 250 ms releases all held contacts.
Malformed input, EOF, Stop and finite expiry release before destroying the
device. Closing the only device file on writer death removes it. A producer must
not send heartbeats indefinitely after perception fails. This writer accepts
already-authorized contact intent, not raw hand detection; it does not decide
engagement or fabricate a contact from two visible fingertips.

## Two-hand gesture replay

`wigglecontrol touch-replay < tests/fixtures/two-hand-touch.jsonl` opens no camera or
device. It emits contact intent using the same bounded reader and pinch engine
as pointer mode. The source frame is explicit: version 1, monotonic source_us,
at most two valid distinct hand IDs, and an `ambiguous` flag. `start`, `stop`,
`tick` and `frame` commands supply a deterministic consumer clock. EOF and malformed
input emit a final neutral contact frame.

Each hand must be eligible, first visibly open, then closed for the configured
confirmation duration. Each confirmed pinch creates a new contact ID; reordering
hands preserves ownership. Release/loss ends only that hand's contact. Staleness,
identity ambiguity or invalid input neutralizes held state. A closed reacquired
hand cannot press until it opens again. Scrolling is disabled in touch mode, which
emits no parallel mouse stream. Smoothed palm cursor coordinates map through an
explicit normalized workspace rectangle, defaulting to the entire camera view.
The rectangle is not a qualified seated-comfort calibration.

The current camera session still controls a **mouse**. Native two-hand association,
producer ambiguity detection during crossing/occlusion and joining this replay
engine to camera-driven touch remain implementation/qualification work. The
producer must actually establish identity rather than claiming certainty from
detector order. A touch-aware application decides how to interpret zoom/rotation;
ordinary applications may emulate a single mouse.
