# Modes and macro meaning

The current runtime separately selects pointer or direct-touch output. The mode
expansion and application constraints are in [ROADMAP.md](ROADMAP.md).
The new controller arbiter joins existing pointer, contact and named macro engines
in camera-free replay. The opt-in [native controller session](CONTROLLER-NATIVE.md) now connects acquisition,
manual controls and the independent writer. A separately staged ARM64 candidate has partial scripted device qualification;
stationary Wayland Chromium handoff is qualified with the selected local compositor
patch, while other clients/locks/grabs and live hand control remain pending.

An optional [trained pose backend](research/gesture-classifier-2026-10.md) now
reuses native landmarks through a parity-checked530KiB ONNX model. Sensor-free
image/sequence probes and an opt-in native classifier→macro connection now exist.
[Its integration evidence](research/native-visual-macros-2026-10.md) does not yet
qualify timed recognition: neutral/action poses have view-dependent failures,
including contact output on a separated-fingertip macro reference.
Unknown is not neutral, and Open_Palm does not establish pinch
openness. Four tutorial inputs and164 synthetic frames do not qualify physical
macro comfort or false-action rates.

## Exclusive controller modes

`wigglecontrol mode-replay CONFIG < controller.jsonl` uses
[the example settings](../tools/controller-modes.example.json). It opens no sensor,
virtual device, application or shell. Settings are at most8KiB; the shared
queue-one reader bounds each JSONL message to16KiB. Nested gesture settings reuse
the actual engine configuration/defaults and validation. Workspace mapping uses
the existing direct-touch rectangle; example bounds are not a receiving calibration.

| Mode | Emitted meaning |
| --- | --- |
| `pointer` | One selected hand's relative movement and pinch button |
| `precision` | Same pinch; relative movement multiplied by `precision_gain` (default0.25, allowed0.05..1) |
| `scroll` | Explicit vertical wheel; no pointer movement, click or contacts |
| `one_contact` | One selected hand's complete direct-contact set |
| `two_contacts` | Up to two independently owned contacts; no mouse stream |

The producer supplies `start`, `stop`, `tick` and `frame` commands, with the same
multi-hand observation used by touch replay. Additional commands are:

```json
{"type":"mode","mode":"precision"}
{"type":"select_hand","hand_id":9}
{"type":"action","action":"cycle_mode"}
```

Changing mode or an explicit track claim releases the old button/contacts in a
transition frame before accepting new input. Pointer modes wait for a fresh,
eligible open hand before moving or clicking; each touch engine retains its own
open-before-press rule. Existing contacts cannot migrate into the new mode.
Identities, timestamps and freshness remain checked across transitions; changing
mode does not reset the source watermark. Stop, EOF and malformed JSON emit a
neutral terminal frame. Source/consumer clock failure, stale input and ambiguity
release state and require rearm. Replay clocks are producer-supplied; native
writer watchdog qualification remains a separate integration requirement.

Automatic primary selection is sticky across array reorder. `select_hand` claims
a track ID for one-hand modes and never falls back to a different visible hand
when that claim is missing. Clearing the claim uses `hand_id:null`. This is not
participant identification; participant-specific workspace/role routing remains
pending. Two-contact mode continues to use both independent tracks.

The [tracking experiment](research/tracking-recovery-2026-10.md) now includes
two same-side hand references and an opt-in palm-matching search policy. Separated
simulation supports two inputs without requiring opposite handedness. Real
participant identification, explicit role/workspace assignment and physical
collaboration are still unqualified; chirality is not a person or a second OS cursor.

An optional frame-level `pose` is classifier evidence, using the existing
`HandPose` schema: ID, confidence and named pose (or positively observed neutral).
Omitting `pose` means missing evidence; it cannot arm a macro. Its hand must match
the actual selected observation, and its confidence cannot upgrade that hand's.
A pending pinch, held button or contact takes precedence over a macro hold.
The optional [onset veto](research/pinch-onset-veto-2026-10.md) blocks new input
without making a raw pending pinch neutral for macros. It does not establish
neutral/action recognition or allow a macro to interrupt an existing hold.

The optional macro engine can locally handle `toggle_precision`, `toggle_scroll`,
`cycle_mode` and `pause_controller`. Cycle is pointer→one-contact→two-contacts→pointer;
precision/scroll return to pointer on cycle. Toggle from the corresponding mode
returns to pointer. A macro mode change discards same-frame movement and requires
rearm. Local pause requires explicit Start to resume and never starts a camera.
Other typed actions return `handled:false` for a future native adapter; they do
not execute. `handled:true` proves only this semantic state change, not an OS effect.

Outputs retain mode, running/ready/held state, primary hand, transition reason,
mouse events, complete contacts and optional action outcome under
`wigglecontrol.controller-output/v1`; replay explicitly reports
`camera_opened:false` and `system_input:false`. An adapter must process the old
release frame before routing subsequent observations. Precision currently scales
relative intent; applying it to saved absolute calibration needs its own adapter.
The installed session still selects its fixed pointer/touch path. Source adds an
opt-in native controller entry point; receiving adoption remains pending.

## Standalone macro replay

`wigglecontrol macro-replay CONFIG < fictional-poses.jsonl` consumes the same
bounded 16KiB JSONL seam as other replay tools. It emits named macro intent with
`executed:false`. It does not open sensors, devices, applications or a shell.
Start/Stop are explicit; EOF ends replay. Configuration is at most 8KiB with 1..16
distinct pose bindings and bounded hold, neutral, cooldown and freshness times.
See [the example config](../tools/macro-bindings.example.json).

```json
{"type":"start"}
{"type":"frame","now_us":0,"observation":{"version":1,"source_us":0,"ambiguous":false,"contacts_active":false,"hand":{"id":7,"confidence":0.99,"pose":null}}}
{"type":"frame","now_us":100000,"observation":{"version":1,"source_us":100000,"ambiguous":false,"contacts_active":false,"hand":{"id":7,"confidence":0.99,"pose":null}}}
{"type":"frame","now_us":200000,"observation":{"version":1,"source_us":200000,"ambiguous":false,"contacts_active":false,"hand":{"id":7,"confidence":0.99,"pose":null}}}
```

With the example config, these three fresh neutral observations arm the engine.
Subsequent fresh `victory` observations over 500ms emit one `toggle_precision`
intent. Holding the pose cannot repeat it. Another positively observed neutral
hold is required, and cooldown also applies. `pose:null` with a confident hand is
neutral; `hand:null` is missing tracking and cannot arm/rearm. Confidence below 0.8,
ambiguity, active contacts, identity replacement, stale/gapped data or malformed
time cancel pending recognition and require neutral again. Ordinary input takes
precedence; a drag cannot simultaneously complete a macro candidate.

The pose vocabulary matches common named pose classifiers (open palm, victory,
thumbs up/down, pointing up and fist); this engine does not establish their visual
recognition accuracy. Actions are typed names: recenter, precision/scroll toggles,
mode cycle, media play/pause, volume steps, launcher, calibration and local pause.
They are never arbitrary command strings. `pause_controller` means this local
controller; no coordinated global Stop action exists in the vocabulary.

Source now exposes observed candidate/neutral/hold/cooldown feedback and bounded
historical local-action receipts. Visual configuration, ownership and positive
open-neutral validation are detailed in [CONTROLLER-NATIVE.md](CONTROLLER-NATIVE.md).
The examples leave visual classification disabled by default.

Remaining work: qualify neutral/action recognition and pinch exclusion on independently labelled sequences;
qualify the connected native controller/session on the receiver; connect selected
configured actions to supported native action adapters; qualify actual actions and comfort. The existing prayer
candidate/calibration workflow remains separately owned and disabled pending its
physical acceptance. No macro can start an idle camera.

## Optional two-finger mouse buttons

Controller settings `"right_click": true` enables the source candidate described
in [MOUSE-ACTIONS.md](MOUSE-ACTIONS.md): thumb/index for left and thumb/middle
for right, with the other pinch open or clearly less closed under the documented
dominance rule. It defaults false. An optional
`middle_pinch` on controller frame commands carries same-track, same-source finger
evidence; the native/provider bridge supplies it automatically from raw landmarks.
Button ownership stays with the pressed finger until release. Both pinches must
reopen before a different already-closed finger can arm; missing evidence is not
neutral. This is separate from movement precision and from a scroll gesture.
No receiving adoption or physical acceptance is claimed by the source option.

## Held detail modifier

The optional `detail` controller setting now connects the actual calibrated
absolute path, independently of exclusive modes and held buttons. Ring/little
flexion is the initial source gesture candidate, not physical acceptance.
See [DETAIL.md](DETAIL.md) for configuration, evidence validation, confirmation,
continuous entry/exit, rest/subpixel behavior and limits. The legacy exclusive
relative precision mode above retains its transition/rearm contract.
