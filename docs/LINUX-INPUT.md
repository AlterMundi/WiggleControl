# Linux virtual mouse qualification

The pinned `evdev` 0.13.2 adapter reuses its supported VirtualDevice builder and
`emit` frame synchronization. [API reference](https://docs.rs/evdev/0.13.2/evdev/uinput/struct.VirtualDevice.html).
The device advertises BTN_LEFT and REL_X/Y/WHEEL/HWHEEL; it does not pretend to
be an absolute pointer or touchscreen. Closing its sole uinput file descriptor
removes this device; real process-failure behavior still needs receiving proof.

`wigglecontrol device-check SECONDS` creates a mouse for 1–120 seconds and emits
no movement/button/wheel events. Use this first for libinput/udev classification.
It requires a receiving grant to open /dev/uinput. Do not run perception as root,
make the node world-writable or presume the node is operational without testing.

`wigglecontrol device-stream SECONDS GAIN` consumes one JSON intent per line.
This command **does inject input** and must be coordinated with desktop use.
GAIN is 1–10000 relative counts per normalized movement unit; it is not pixels.
There is no claim to bypass compositor acceleration or know absolute position.

Intent types: `move` with finite `dx`/`dy` each bounded ±1; `button` with `down`;
`scroll` with integer `vertical`/`horizontal` each bounded ±10 steps;
`heartbeat`; `stop`. Lines are limited to 16 KiB and the reader's queue to one.
Malformed input and EOF end the device session; stop and expiry release the
button before device removal. An independent 20 ms writer loop releases held
input after 150 ms without an intent, even if stdin blocks. Stationary tracking
must still deliver fresh heartbeat frames. MouseState preserves fractional
motion to avoid systematic rounding loss and emits idempotent button edges.

This is the device-side intent seam, distinct from source-time observations.
The producer must enforce observation freshness; the writer watchdog enforces
producer availability. A future live connection must fence stale queued intent
across disconnect/reconnect and retain neutral reacquisition. The replay CLI is
not yet a live bridge and should not be piped unpaced into this device writer.

Unit tests verify codes, button release, bounded inputs and fractional movement.
They do not prove libinput classification, actual application delivery, fullscreen
behavior, physical usability or held-button recovery after SIGKILL.

## Receiving evidence — 2026-10-13

Pi 5 ARM64 with its existing extracted Rust 1.85.1 compiled the pinned package
and passed all 14 gesture, replay and mapping tests. Production panel/toolchain
configuration was not changed. VPS development used Rust 1.95.0.

A temporary named-operator ACL was applied to /dev/uinput, retaining its exact
prior ACL in the receiving private state. Other users gained no access. This is
an operator-scoped raw virtual-input grant, not enforcement of mouse-only intent;
a more constrained writer service remains a deployment decision. No persistent
udev rule was installed. Reboot removes this temporary setup; reversal restores
the retained ACL using the receiving administrator route.

A five-second device-check created the device; udev classified ID_INPUT=1 and
ID_INPUT_MOUSE=1. The virtual device was absent after expiry. No movement/button/
wheel events, camera or microphone were activated. Application delivery, actual
watchdog release and SIGKILL removal are separate remaining tests.
