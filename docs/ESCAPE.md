# Escape delivery candidate

Owner: [issue10](https://github.com/AlterMundi/WiggleControl/issues/10).
Source candidate2026-10-18; no receiving deployment or physical Escape test.
The fixed-key delivery adapter and opt-in raw-landmark gesture now connect through
the native/provider controller. Keyboard classification/application effects and
actual gesture recognition/comfort still need receiving qualification. The
accepted beta is preserved and the public preview remains incomplete.

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
Periodic ticks, EOF and cleanup cannot repeat the tap. The gesture producer
neutralizes its own pointer/gesture state and requires positive rearm; distinct
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

Next: qualify keyboard classification and Esc with owned scratch application
input, then fresh acknowledged recorded human gesture trials. No new model or
inference stream is required. Keep accepted beta/calibration intact.


Local qualification:214 native-feature tests passed, zero failed, one existing
opt-in benchmark ignored. Native all-target Clippy and format checks pass.
The isolated installation/legacy-upgrade/rollback fixture passed. These are
source checks, not keyboard classification/application-effect evidence.

Exact-source [CI39177348502](https://github.com/AlterMundi/WiggleControl/actions/runs/39177348502)
passed replay, native-vision/isolated installation and ARM64 build/test jobs for
sourcef16f399. Its artifact remains receiving-unqualified and is not selected on
the Pi. The later gesture increment below connects that producer; human Escape
acceptance is still pending.


## Deliberate one-hand gesture source candidate —2026-10-18

Controller settings can opt in with `"escape": {}`; omission/null keeps the
feature disabled. It is independent of generic/visual macro configuration and
works in pointer/precision/scroll modes, not touch modes. The shared raw-landmark
bridge used by native sessions, GPU provider and retained provider replay supplies
pose evidence from the selected fully in-view hand. No classifier/model is loaded.

| Setting | Default | Meaning |
| --- | --- | --- |
| hold_us |700000 | Continuous fresh thumb-out observations before Esc |
| neutral_us |200000 | Continuous positive open pose before acquisition/rearm |
| folded_ratio |0.55 | Maximum chord/path ratio for each curled finger |
| extended_ratio |0.8 | Minimum ratio for straight fingers/thumb |
| thumb_clearance_widths |0.35 | Thumb-tip projection beyond index MCP, along little-to-index palm axis |

The candidate requires all four fingers curled, a straight thumb and the thumb
outside the palm on its anatomical side. Exact geometry is rotation/reflection/
scale invariant, not an assertion about model accuracy or physical comfort.
Neutral is all four fingers extended and both configured pinch inputs positively
open. A valid alternate pose (such as ordinary pinching or ring/little detail)
is `other`: it cancels pending Escape progress without disarming an already
neutral-armed gesture. It cannot itself arm/fire or be bound to a macro action.
Missing, out-of-view, low-confidence, wrong-owner, stale or ambiguous evidence
requires fresh neutral; neither dropout nor a still-held gesture can rearm.

The existing macro hold/rearm engine supplies timing. This geometric path uses
the actual configured observation-admission threshold, including the selected GPU
provider's0.5 lower bound; it never invents a per-hand score. The public named-pose
macro constructor/visual policies retain their0.8 confidence requirements. First
acquisition and reacquisition while already holding thumb-out cannot emit Esc.

A confirmed-owner cancellation candidate suppresses pointer motion/pinch guesses.
While a held input changes through a positively observed in-view ineligible pose,
only the existing same-source bounded drag grace is retained. These poses cannot
refresh/extend its deadline. The pointer still releases at expiry; an independently
fresh Escape hold may finish afterward. Stop, owner/mode changes, invalid frames
and stale poses still reset it. Ordinary ineligible poses with no held input
preserve normal pointer rearm behavior. Non-candidate click/detail behavior is
checked against retained observations rather than inferred from geometry alone.

Confirmation neutralizes pointer/detail/pending gesture state and emits a single
`Output.escape` with its original observation time. Publisher emits the typed
Escape transaction instead of ordinary transition/motion; the pre-publication
freshness guard also applies and can replace a late Escape with neutral only.
A short semantic action receipt is not evidence of actual key delivery. Raw
controller replay frames can supply optional `escape` evidence:

```json
{"hand_id":7,"source_us":1000001,"pose":"thumb_out"}
```

`pose` is `neutral`, `thumb_out` or `other`; omission means unknown. Evidence from
the shared bridge is visibility-qualified and checked against the selected hand's
actual confidence/source. `Command::Action` with Escape is not a key injection
shortcut: deliberate output comes from the fresh pose path. Lightweight provider
configuration exposes configured Escape and source-derived hold/rearm feedback;
terminal state clears that feedback. Optional video/skeleton display is unnecessary.

Public synthetic tests cover geometric transforms/degeneracy, deliberate hold,
left/right held cancellation, positive rearm, no repeat, loss/owner/confidence/
source/ambiguity guards, native raw visibility, real configured0.5 admission,
nonextended drag deadlines, completion after pointer expiry and late-publication
suppression. Geometry fixtures are algebraic contracts, not biomechanical or
hand-model recognition ground truth.

Paired private Rust provider replays cover1912 retained observations (248+393+1271).
Escape-enabled and disabled runs preserve mouse events, cursor, state/readiness,
button identity/held state, owner, detail, transitions and contacts on every frame.
Thirteen press intents (11left/2right) and478 detail observations match. No Escape
candidate or action occurs; the detector is neutral-armed on1206 observations.
Original video/landmarks, configs, source/binary hashes and derived outputs stay
private. These are comparisons with retained predictions, not new inference,
intermediate writer ticks, OS effects, pose intent labels or false-action rates.
Actual Escape gesture/keyboard delivery remain unqualified on the receiving host.
