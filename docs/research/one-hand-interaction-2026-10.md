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

## Next comparison

Keep the new bounded continuity protocol while developing a separate cursor output
cadence and evaluating short-horizon extrapolation alongside the measured filter
tradeoff. Distinguish noisy opening from absent evidence and fast deliberate
release before changing confirmation. The replay seam can evaluate these decisions
without consuming the receiving screen. Real libinput/compositor/application and
human comfort remain separate acceptance steps. The separation of observation
freshness, estimator liveness and bounded drag state is a candidate for selected
collective research; preserve its synthetic/retained limits and publish with the
integrated comparison rather than claiming physical improvement now.
