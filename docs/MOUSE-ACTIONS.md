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

## Still to implement and qualify

The selected live gesture producer still generates the existing left pinch.
A right-click gesture/selector and its held-button ownership must be connected;
this output foundation alone is not a working right-click gesture. Escape/key
output, nonconflicting gesture scroll, natural double-click comfort and the
complete action vocabulary remain issue10 work. No receiving binary, gesture
profile, calibration or resource policy was changed by this increment.

Actual right/middle device classification, client effects, loss/Stop cleanup and
human gesture comfort need coordinated Linux receiving qualification. Pi tests
remain serialized with realtime-conversation work; development/replay stays
off-Pi until a coordinated window is needed.
