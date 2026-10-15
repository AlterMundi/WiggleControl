# Modes and macro meaning

The current runtime separately selects pointer or direct-touch output. The mode
expansion and application constraints are in [ROADMAP.md](ROADMAP.md).
This increment adds a replayable macro semantic engine, not a deployed camera
gesture classifier or desktop macro executor.

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

Remaining work: compare a real pose classifier on independently labelled images;
show candidate/hold/action progress in the native view; implement explicit mode
arbitration that neutralizes old output; connect selected configured actions to
supported native adapters; qualify actual actions and comfort. The existing prayer
candidate/calibration workflow remains separately owned and disabled pending its
physical acceptance. No macro can start an idle camera.
