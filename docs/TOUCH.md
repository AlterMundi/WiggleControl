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

The current camera session controls a **mouse**. Two-hand association, gesture
engagement for touch, crossing/occlusion policy and camera-driven direct touch
remain implementation/qualification work. Do not claim the contact writer itself
provides those behaviors. A touch-aware application decides how to interpret
zoom/rotation; ordinary applications may emulate a single mouse.
