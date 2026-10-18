# One-hand mouse actions

Owner: [issue10](https://github.com/AlterMundi/WiggleControl/issues/10),
[Linux public preview](LINUX-PREVIEW.md). Source, replay, receiving device effects
and human gesture acceptance are separate stages.

## Explicit button output —2026-10-18

The source now distinguishes left/right/middle buttons through the independent
Linux writer. Previously the absolute virtual device advertised right/middle,
but intent application always emitted BTN_LEFT. The relative device now declares
the same three supported mouse buttons. Each button has independent held state;
legacy and named left share one state, preventing duplicate edges.

The additive controller event and direct-writer command are:

```json
{"type":"named_button","button":"right","down":true}
{"type":"named_button","button":"right","down":false}
```

`button` is one of `left`, `right`, `middle`. Existing
`{"type":"button","down":true}` remains left-only. Unknown button values and
extra event fields are rejected rather than silently becoming left clicks.
New variants require a matching writer; the source launches its own exact
executable, and older artifacts must not be paired with this event producer.

Cancel, Stop, EOF/error cleanup, source expiry and writer watchdog cleanup release
every held button. The direct writer also accepts `{"type":"release"}` to
neutralize all buttons without ending the writer; session cancellation uses it.
The calibrated delivery path preserves position-before-button ordering, bounded
same-source drag holds and source-age/sequence/mode fences. Scroll may release a
named button but cannot press one. Calibration without a valid target permits
release/cancel events and excludes new presses.

Device-free regressions cover mixed legacy/named button state, duplicate edges,
individual release and all-button cleanup; absolute right-press ordering, hold
expiry, watchdog, mode transition, cancellation and rejected frame cleanup; CLI
EOF, malformed JSON and invalid button fields with multiple buttons held; stale
calibration release; native-session serialization into the direct writer.
These tests generate Linux event values without opening sensors or uinput.
Local full native-feature run:186 passed, zero failed, one existing opt-in
benchmark ignored (83 library tests plus integration suites). The aggregate
includes unchanged tests; it is not186 new tests or physical acceptance.

## Independent index/middle pinch candidate —2026-10-18

Nicolás clarified the intended right gesture: thumb + middle, without requiring
thumb + index + middle together. The source now offers `"right_click": true`
in the controller settings (not the camera/session configuration). It defaults
false; the selected receiving beta/settings are unchanged.

With that explicit source option:

- Thumb/index pinch with middle separated selects left; thumb/middle pinch with
  index separated selects right. Both pinches closed before an action is unknown,
  not a new three-finger chord. Independent finger comfort/threshold coverage
  still needs physical qualification.
- Both finger pinches must be positively open before acquisition/rearm. Closing
  another finger during a drag cannot transfer its button. Releasing the active
  finger while the other remains closed releases only the active button and
  cannot arm that already closed finger for a new press.
- The selected button and its measured finger remain consistent until release,
  including Stop, tick expiry, mode change and same-hand brief-loss grace.
  Missing middle evidence during a right drag is unknown input under the existing
  grace/watchdog rules, never a measured middle opening or index-based release.
- Button selection and the existing primary pinch each use the existing temporal
  confirmation budget. Unknown/mismatched finger evidence cannot fall back to a
  new left click. Source time, selected track, confidence/eligibility and unit-kind
  checks remain in force. This button ownership does not impose precision-mode
  locks; independent detail motion remains later work.

One shared native/provider/recorded-sequence bridge extracts the middle tip and
its gap using the same selected SessionConfig metric/normalization as index.
It does not load a pose classifier or add a second inference stream. An index
secondary-crop estimate cannot serve as middle evidence: opt-in dual-index
estimation is rejected before acquisition. Perception quality is not promoted.

Controller JSONL frames may provide optional `middle_pinch` with exact `hand_id`,
`source_us`, normalized `middle_tip`, optional `pinch_distance_m` or
`pinch_distance_ratio`, and optional `onset_allowed` (default true). Metric kind
must match the primary hand; omission uses the same normalized tip/palm fallback
only when the primary observation uses that fallback too. The native bridge
supplies this from the selected tracked hand. It is another finger observation,
not the existing `secondary_pinch` alternative estimate of the index gesture.

Preflight reports the right-click selection. Meaning feedback reports
`right_click` and the selected/held `pointer_button`; optional preview labels a
held right button and highlights thumb/middle. The raw ratio label remains
explicitly index-based. Mouse-only operation still needs no preview rendering.

Local qualification:196 full native-feature tests passed, zero failed, one
existing opt-in benchmark ignored; native all-target Clippy, format and preview
recovery/timer checks passed. New coverage includes raw-landmark-to-controller
routing with the correct claimed hand (and actual configured0.5 confidence),
serialized middle-only input through the absolute event pipeline/EOF release,
exclusive finger selection, no button transfer, explicit dual-open rearm, unknown
middle evidence and default-left behavior. These are constructed observations,
not model recognition accuracy, real OS client effects or human acceptance.

## Still to implement and qualify

The source candidate is not installed/enabled for the human. Actual device/client
classification and effects, pinch comfort, false activation, natural double click
and the complete vocabulary remain issue10/#13 work. The earlier thumb/middle
scroll candidate must not also select right: choose/qualify a separate scroll
gesture before release. Existing explicit scroll mode remains separate. Escape
and detail motion remain planned work; no selected runtime/profile changed.

Pi tests remain serialized with realtime-conversation work. Continue off-Pi
source/replay work and obtain a coordinated, freshly acknowledged physical window
when the selected increment is ready. No camera or uinput was opened here.
