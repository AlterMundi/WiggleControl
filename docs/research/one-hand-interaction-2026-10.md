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

## Next comparison

Retain source-age truth and the independent process-health watchdog while adding
a bounded estimator-owned drag grace and a separate cursor output cadence. Start
with frozen uncertain motion, confirmed-open release and same-hand return; compare
approximately one-second grace with shorter candidates. Evaluate existing One Euro
and exponential filters before introducing bounded short-horizon extrapolation.
The replay seam can evaluate those decisions without consuming the receiving
screen. Real libinput/compositor/application and human comfort remain separate
acceptance steps. This baseline is repository qualification; selected reusable
continuity findings can become collective research after a measured comparison.
