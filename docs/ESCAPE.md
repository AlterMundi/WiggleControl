# Escape delivery candidate

Owner: [issue10](https://github.com/AlterMundi/WiggleControl/issues/10).
Source increment2026-10-18; no receiving deployment or physical Escape test.
Gesture recognition, deliberate hold, positive rearm and the native/provider
producer still need integration. This is the delivery adapter, not a finished
gesture or a selected beta feature. The public preview remains incomplete.

## Fixed cancellation transaction

The independent controller writer accepts an additive typed packet:

```json
{"type":"escape","sequence":4,"issued_us":60001,"source_us":60000,"mode":"pointer"}
```

It uses the same sequence, issue-clock and watchdog fences as pointer packets.
Source must be fresh, increasing, not in the future, and match an armed mode.
Unarmed, duplicate, stale or mismatched input fails without a key event; normal
writer error cleanup still releases held input. A valid packet releases every
controller-owned mouse button and touch contact before one Escape tap. It does
not move the pointer, change the mode, stop the controller or invoke global Stop.
Periodic ticks, EOF and cleanup cannot repeat the tap. A future gesture producer
must neutralize its own pointer/gesture state and require positive rearm; distinct
fresh packets are deliberate commands, not a receiver-side gesture recognizer.

Delivery replay adds a `keyboard` event array; ordinary packets produce an empty
array. Escape produces KEY_ESC down/up values1/0. The native writer owns a separate
`WiggleControl Escape Keyboard`, created at writer startup for device discovery.
Only KEY_ESC is advertised; it emits each edge with a separate SYN_REPORT and
attempts key release even if the press write fails. Final cleanup retries release
and drops the device. Mouse/touch cleanup precedes any Escape key emission.

No direct mouse-intent vocabulary or arbitrary key/shell interface is added.
New packets require a matching writer; managed sessions already spawn their own
current executable. Gesture/OS delivery acceptance must use an exact matched
artifact and record actual target-application effects separately.

## Linux preparation

`tools/install.sh` and `tools/prepare_input.sh` install the fixed-name rule in
`packaging/80-wigglecontrol-keyboard.rules`. It sets input/keyboard classification
only and grants no new access. A single Escape capability does not satisfy the
full-keyboard test in [systemd257 input_id](https://github.com/systemd/systemd/blob/v257/src/udev/udev-builtin-input_id.c#L337).
Both fresh installation and legacy-manifest upgrade preserve the initial rule
or its absence for rollback, together with the existing module/ACL policy.
Run preparation before creating the candidate devices; rule installation alone
does not prove that libinput/compositor accepted a keyboard or that Esc reached
an application. Those receiving checks remain required before gesture trials.

This small extension does not make the existing native installer independent
of daimon-integrators or install the selected GPU beta. Complete standalone
packaging remains [issue12](https://github.com/AlterMundi/WiggleControl/issues/12).

## Source evidence and next work

Device-free tests cover all-button and two-contact release, exact Escape event
serialization with neutral EOF, stale/future/source-order/mode/arm/sequence
rejection, and key-release attempts after failed writes. Isolated installation
fixtures cover original content/mode restoration, absent-rule removal, retries,
interrupted manifest extension and legacy upgrades without host/device changes.

Next: bind a distinct deliberate one-shot pose using existing hand landmarks and
hold/rearm semantics; integrate into the selected native/provider path without
new inference or interference with left/right/detail/scroll. Then qualify keyboard
classification and actual Esc with owned scratch application input, followed by
fresh acknowledged recorded human trials. Keep accepted beta/calibration intact.


Local qualification:214 native-feature tests passed, zero failed, one existing
opt-in benchmark ignored. Native all-target Clippy and format checks pass.
The isolated installation/legacy-upgrade/rollback fixture passed. These are
source checks, not keyboard classification/application-effect evidence.

Exact-source [CI39177348502](https://github.com/AlterMundi/WiggleControl/actions/runs/39177348502)
passed replay, native-vision/isolated installation and ARM64 build/test jobs for
sourcef16f399. Its artifact remains receiving-unqualified and is not selected on
the Pi. The next increment must add the deliberate gesture producer before human
Escape acceptance can be claimed.
