# One-hand interaction baseline —2026-10-16

Nicolás's Beta1 feedback motivates the current [one-hand goal](../ONE-HAND-GOAL.md):
good-looking hand/pinch tracking and useful target reach, but unreliable double
pinch double-click, interrupted drags and visibly stepped cursor movement.
The human is watching Farscape on the receiving desktop. All work in this
increment is local and device-free; no receiving screen, injection, camera,
deployment, service restart or heavy receiving experiment is used.

## Reproducible source baseline

`pipeline-replay SETTINGS GAIN WATCHDOG_MS --absolute` now uses the same
`Publisher::absolute_packet` mapping and delivery state as Beta1, including actual
Linux event codes, sequence/source fences and source-independent release. It
creates no devices or camera. The existing replay without the flag stays relative.
The full-pipeline CLI regression checks calibrated axes before press, EOF release
and release before rejecting an unsupported absolute-mode switch.

Run `tools/one_hand_baseline.py --binary target/debug/wigglecontrol --output-dir
NEW_DIRECTORY` after building. It retains settings, every synthetic observation/
tick, actual Rust receipts, binary/tool digests and a summary. Six deterministic
cases use68ms source spacing, a separately modelled65ms delivery delay and16.667ms
ticks. Workspace is0.2..0.8 on both axes, projected ratio0.38/0.40, freshness300ms,
ordinary exponential smoothing30ms and1920×1080 metric conversion. Synthetic
intent/motion is known by construction, never inferred from private video.

| Case | Max output step (px) | Target RMS (px) | Movement interval median (ms) | Presses | Releases during known closed intent |
| --- | ---: | ---: | ---: | ---: | ---: |
| Stationary, alternating ±0.002 source noise | 11.48 | 5.20 | 68.0 | 0 | 0 |
| Linear motion | 34.83 | 51.87 | 68.0 | 0 | 0 |
| Reversal, then stop | 54.41 | 70.39 | 68.0 | 0 | 0 |
| Two complete pinches at a fixed cursor | 0 | 0.015 | — | 2 | 0 |
| Drag,272ms last-good→return interval | 874.41 | 461.55 | 68.0 | 1 | 1 |
| Drag,816ms last-good→return interval | 874.41 | 461.55 | 68.0 | 1 | 1 |

Target RMS is weighted by elapsed command time after250ms, against the known
current-time trajectory; it includes the modelled source delay and sample hold.
It is not measured physical latency or prediction accuracy. The large gap-case
jump occurs when the hand finally opens again, not when the missing frames return.
Every case ends neutral. Receipt-level button presses in the double-pinch case
are340ms apart; no application was asked to recognize a double-click.

These results isolate current source behavior: absent hands immediately cancel
the established drag and require open-before-press on return. Absolute pointer
motion advances only on observed frames; faster independent ticks do not currently
fill the intervals. This supports the proposed temporal interaction work, without
establishing the cause of each physical symptom or issue#8. The old implementation
and a future candidate must be compared on identical clocks/trajectories, including
misses, reversals, premature release, stationary noise and prediction overshoot.

## Terminal activity correction

The camera-worker producer's mailbox retains whether its camera was ever opened;
the bridge similarly retains whether an input writer was started. Those historical
flags previously leaked into terminal activity fields. After producer join, current
camera activity is now false. After confirmed writer reap, current system input is
false; a failed wait after input startup returns unknown/null. Separate
`camera_was_opened`, `input_was_started` and `writer_reaped` retain history and
teardown evidence. Source tests cover terminal HTTP state and unknown input.

This corrects receipt meaning. It does not fix or explain the PCManFM drag that
survived device removal. Actual receiving kernel/application qualification remains
pending until the human makes the desktop available; no release is accepted yet.

## Bounded drag continuity candidate

The source-only candidate adds `gestures.drag_grace_us` (default0, disabled),
bounded to source freshness..2000000us when enabled. The camera-worker session
uses matching `pointer_drag_grace_ms` and requires explicit absolute-provider
selection. No receiving configuration or binary is changed.

An already pressed pointer/precision drag retains its original eligible-hand
source stamp, owner and fixed source-derived deadline during missing or low-quality
samples. Uncertain motion freezes. Same-ID valid return resumes without another
press; a valid open measurement releases immediately. Confirmed disengagement,
ambiguous identity, another selected owner, invalid input, Stop and mode changes
release. Missing samples cannot create a press, refresh the evidence stamp or
extend the deadline. Touch contacts retain their prior loss behavior.

The writer's new `hold` packet carries estimator liveness separately from source
evidence. It contains no movement or button press, can only maintain an already
held pointer from its last delivered source, and cannot extend that source's first
deadline. The independent300ms producer watchdog still releases if this heartbeat
stops. The writer also expires the fixed hold deadline even if heartbeats continue.
Both sides reuse the same executable in native operation; the new packet is not
sent by old/default profiles. Eight device-free controller→wire→encoder regressions
exercise those distinctions, including deadline expiry without new observations.

The provider can retain one geometric association during absence, without emitting
a synthetic hand or upgrading an old observation. It must still pass the existing
spatial/handedness match on return; expiry, invalidity and ambiguity retire it.
This is geometric track continuity, not proof of participant identity. It does not
solve larger displacement or ambiguous returns. Session/config/tracker source
tests and the native suite pass; receiving application acceptance remains pending.

### Identical-input comparisons

The baseline tool now accepts `--drag-grace-ms` and
`--filter exponential|one-euro`, and covers ten cases. Forty actual pipeline
executions compare grace0/1000ms with exponential filtering, plus grace1000/400ms
with the existing One Euro settings1Hz/beta10/derivative1Hz. All use identical
observations/clocks per case and the same native-feature binary SHA256
`27ca96087ff74cde9beaaa2c58a3b02dd2693a670e496910216c1c54320ff4f6`.

| Case | Baseline closed-intent releases | 1000ms grace, exponential | Remaining limit |
| --- | ---: | ---: | --- |
|272ms explicit missing-hand interval |1|0|Return still jumps143px |
|816ms explicit missing-hand interval |1|0|Return still jumps422px |
|272ms interval without any source frames |1|0|Return still jumps143px |
|1496ms missing-hand interval |1|1|Bounded timeout intentionally expires |
|One-frame false open measurement during a known synthetic drag |1|1|Not an absence; opening still releases |

One-second grace retains a single press throughout the three recovered cases;
400ms grace retains the272ms cases but expires in the816ms case. Uncertain motion
remains frozen, so catch-up steps still need a separate output-cadence treatment.
The noisy-open case still releases then presses again; no release-confirmation
policy is adopted from this test alone, since extra confirmation also delays
genuine opening and can merge fast clicks.

One Euro reduces stationary RMS5.20→1.27px and maxstep11.48→4.28px, but raises
linear current-target RMS51.87→79.62px and reversal RMS70.39→99.81px. Both filters
still output at68ms observation spacing. A synthetic pinch-correlated cursor drift
places consecutive presses25.34px apart with the exponential filter,12.45px with
One Euro; both are340ms apart. These are generated counterexamples, not the
measured cause of Nicolás's double-click problem or acceptance by any application.
Do not select One Euro just for its stationary improvement.

### Retained observations: no demonstrated drag improvement yet

Six local provider-replay executions reuse two older recorded-trial journals:
796 accepted predictions over60.081s and1087 over80.000s, each under0/400/1000ms
grace. Original landmark/world-point values and source/arrival intervals are reused;
the replay epoch is explicitly shifted and independent16.667ms ticks inserted.
No image inference, camera, device or receiving work occurs. These are not the
three later Beta recordings, which remain on the occupied receiving host.

Held time remains11.382696s and21.486957s under all three settings, with12/18
presses and matching releases including EOF. Every replay ends neutral. None of
these established holds enters grace:29 releases come from eligible observations
whose projected ratio reaches0.4; the other release is EOF Stop. Track counts
remain10 in the first window and drop12→8 in the second. Fewer IDs and more held
time would not independently prove better ownership or intent accuracy.

The retained evidence therefore supplies no physical drag benefit for this
candidate yet. Intent-labelled Beta footage and release-metric stability remain
necessary; source-level synthetic continuity alone is insufficient. The useful
next comparison is timestamp-aware cursor cadence and click/hold interpretation,
including isolated noisy measurements and rapid genuine opening.

## Independent absolute cursor reconstruction

The next source-only increment adds optional `cursor_output` settings. Matching
worker sessions use `pointer_cursor_output`; mismatches and unsupported native
workflows are rejected before acquisition. The outer option defaults to absent,
so the installed/default profile is unchanged. Controller observation filtering,
pinch decisions and event timing remain separate from this output stage.

The publisher reconstructs absolute positions on its own nominal16.667ms schedule,
using existing independent controller ticks. New observations update a screen-space
target and smoothed finite-difference velocity. A short-horizon estimate feeds an
output low-pass filter and normalized speed bound. Source stamps stay unchanged;
the writer's separate `cursor` packet accepts only its last fresh absolute source,
an armed pointer and bounded coordinates. It creates no presses or observations.
Derived motion also reserves one20ms writer-poll interval before the tighter
controller/writer source fence; near that boundary it freezes and permits only
existing hold liveness. A regression covers publication crossing the source fence
after a still-fresh controller tick, without pretending the image became fresh.

During explicit drag uncertainty, motion/prediction freeze and existing `hold`
liveness continues. Recovery resets velocity and reconciles from the frozen output
with the configured speed bound. Stop, source expiry, mode changes and ownership
changes reset reconstruction. A held-owner handoff releases at the old visible
position before initializing another cursor. Worker feedback separates the original
`observed_screen_target` from `screen_target` and reports observed/predicted/estimated
positions, original source, estimate time, effective prediction horizon and frozen
state. These are estimator/publisher receipts, not proof of OS delivery.

An initial seven-profile, twelve-case comparison at16.667ms replay ticks found a
tradeoff: time-only80/100ms prediction reduced linear target RMS51.87px to26.98/
20.66px, but overshot a stop by43.84/55.59px. The unselected prototype and exact
input/output/digests are retained privately. It is not adopted as a daily profile.

The revised estimator also bounds predicted lead by distance and requires two
successive consistent velocity intervals. Direction disagreement or marked speed
changes withdraw prediction; interpolation continues. Experimental reference
defaults are interval16667us, output smoothing30000us, prediction80000us,
maximum predicted distance0.01 normalized screen units, consistency checking on,
velocity smoothing80000us and maximum speed2 normalized units/s. The first target
initializes directly because there is no previous owned cursor estimate.

### Five-millisecond scheduling comparison

Seven further profiles each execute the same twelve actual Rust pipeline inputs
with5ms logical ticks,68ms observations and65ms source→delivery delay. This exposes
the existing loop's scheduling granularity instead of assuming exact60Hz output.
Input bytes match across all seven profiles; all84 cases end neutral and retain
the same button counts and closed-intent releases. Measurement binary SHA256:
`e4f4db2f0b8b0966f50f6d26383d904611c9d678ba3394ef6d3156935c1a1aaa`.
It predates added predicted-target diagnostics, receiving config discovery and the
transport-time reserve. A final native-feature binary
`77031e2835a2435c01e53416236746dba0358ccfb3e048bd044895e097d2e006`
repeats baseline/reference cases: all within-table metrics below remain identical;
the no-source-gap case changes RMS62.15905→62.15996px and maxstep48.692→48.721px.

| Profile | Stationary RMS (px) | Linear RMS (px) | Reversal RMS (px) | Stop overshoot (px) |816ms return maxstep (px) |
| --- | ---: | ---: | ---: | ---: | ---: |
|Baseline, observation-driven |5.20|54.49|73.80|0.01|421.82|
|80ms time-only prediction |5.34|29.67|51.97|43.90|76.79|
|Guarded80ms, lead0.01, output30ms |2.76|50.08|79.03|9.95|76.79|
|Guarded80ms, lead0.005, output30ms |2.76|59.45|85.63|0.34|76.82|
|Guarded80ms, lead0.01, output15ms |3.89|42.46|68.99|14.25|76.82|
|Guarded80ms, lead0.02, output15ms |3.89|24.74|56.91|33.44|76.82|
|Existing One Euro + guarded80ms/output30ms |0.77|78.87|109.77|0|76.82|

Other rows use the unchanged30ms observation exponential filter. The guarded
reference changes linear movement spacing from68ms median/p95 to15/20ms, and
reduces stationary maxstep11.48→4.42px. The parameter alternatives show why one
scalar score is insufficient: tighter lead reduces overshoot but increases lag;
stronger smoothing helps rest and hurts motion. The One Euro combination reduces
synthetic pinch-correlated press separation25.34→7.30px, but worsens moving-target
error substantially. It is not adopted for that isolated benefit.

An abrupt synthetic step remains speed-limited: maxstep1147→77px, but target RMS
249→385px. This deliberately discontinuous input tests reconciliation, not a
physically achievable hand trajectory. Large recovery still takes time; this is
not a solved UX. The displayed RMS values depend on the identical within-table
logical observation schedule; the earlier16.667ms table is not a different physical
performance measurement. These experiments use supplied replay clocks, not a new
wall-clock Pi run or demonstrated display fluency. Seven integration tests cover
source provenance, speed bounds, direction gating, uncertain freeze/recovery,
expiry, Stop and owner handoff. Config checks and native regressions pass locally.

No candidate is installed or physically accepted. The receiving screen remains
reserved for Farscape. The next iteration must address pinch-induced cursor drift,
release evidence stability and real application double-click/drag requirements,
then compare the integrated candidate using actual receiving timing and human use.

## Next comparison

Keep the new bounded continuity and cursor reconstruction protocols while tuning
the measured tradeoff and separating pinch motion from intended cursor motion.
Distinguish noisy opening from absent evidence and fast deliberate
release before changing confirmation. The replay seam can evaluate these decisions
without consuming the receiving screen. Real libinput/compositor/application and
human comfort remain separate acceptance steps. The separation of observation
freshness, estimator liveness and bounded drag state is a candidate for selected
collective research; preserve its synthetic/retained limits and publish with the
integrated comparison rather than claiming physical improvement now.
