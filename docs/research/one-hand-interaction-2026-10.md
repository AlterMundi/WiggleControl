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

## Click stabilization and sampling limits

Source inspection confirms that the provider cursor already uses the midpoint of
index/pinky MCP landmarks5/17, mirrored horizontally. It is not the index tip.
The existing palm anchor and accepted workspace are preserved. Pinch-induced
cursor drift is a synthetic counterexample and a possible physical effect, not a
verified explanation of the reported double-click failure.

An optional `cursor_output.click_stabilization` holds the preceding visible cursor
while closing/early press stays within a small radius. Radius exit or a finite
closing/pressed deadline disables locking for the remainder of that gesture, so
fine drags eventually move too. A release while locked is emitted at the held
point before hover resumes. Real observed position/source and button decisions
are preserved; `click_locked` is distinct from uncertainty `frozen`. Defaults for
this inner option are radius0.008 normalized screen units, closing200ms and
pressed150ms; the option itself remains absent by default. Session and controller
objects must match before acquisition.

Five profiles run the same14 generated command streams with5ms ticks,68ms
observations and65ms supplied delay:70 cases, identical input bytes, all neutral
at EOF. Measurement binary SHA256:
`718608081cd95067cc6840ff1558e1a2954a940af68400285e11be695c548bef`.

| Candidate | Pinch-drift press separation(px) | Fine-drag target RMS(px) |
| --- | ---: | ---: |
|Reconstruction reference, no click lock |21.65|1.25|
|Click lock, pressed80ms |0.41|1.31|
|Click lock, pressed150ms |0.26|1.45|
|Click lock, pressed300ms |0.26|2.03|

These are two emitted presses, not an application's accepted double-click. Longer
locking improves this stationary example while delaying fine movement. Gap-return
maxstep remains76.79px; the isolated noisy opening still interrupts the held drag.
The click stabilizer does not solve those separate problems.

### Temporal confirmation is limited by observations

`tools/score_click_timing.py` supplies two known closures at a stable target across
17 phases of a68ms source schedule,5ms ticks and65ms supplied delay. Each profile
runs102 cases. Actual encoded mouse events are identical under40ms and1ms press
confirmation in every case. Any positive confirmation below68ms still needs a
second closed observation. Explicit0ms can use the first eligible closed sample;
prior open evidence and onset permission remain required. It is configurable as
`gestures.confirm_us:0` and matching session `press_confirm_us:0`, not adopted.

| Closure/open interval(ms) | Two complete pairs/17 phases,40ms |1ms |0ms |
| --- | ---: | ---: | ---: |
|60/80 |0|0|14|
|80/80 |0|0|17|
|100/100 |7|7|17|
|140/100 |17|17|17|
|200/100 |17|17|17|
|140/40 |10|10|10|

All306 cases end neutral. A40ms open interval can fall between observations, so
neither policy can reliably distinguish the two closures; no extra click is
synthesized. For the140/100 case, first-gesture down occurs median165ms after its
known intent onset with40/1ms confirmation,97ms with0ms. Those include the
assumed65ms delay and sampling phase: they are not measured camera/device latency.
The measurement binary is the same as the motion table above.

The added resting-hand counterexample supplies one false closed observation while
known synthetic intent stays open. It emits no press at40ms and one unwanted
press at0ms. Shorter confirmation therefore has a concrete false-action cost;
default40000us remains unchanged. Increasing actual useful observation cadence is
another candidate, but requested capture FPS is not proof of delivered cadence.
No camera controls or receiving settings were inspected/changed during Farscape.

### Cross-check against retained provider outputs

Six device-free pipeline executions reuse the same two older windows from the
preceding retained comparison:796 and1087 normalized predictions, preserving
source/arrival intervals, ambiguity and hands. Provider inference is not repeated;
the previously saved provider receipts are translated into controller frames.
All three profiles within each window consume identical commands, use1000ms drag
grace, and end with matching releases and neutral EOF. Binary SHA256:
`73cc51bf8b28c9a9b4377494e1e757b8baa14bb6c2e8364b56b38bcfe98cecfc`.

The150ms click stabilizer preserves12/18 presses and11.382696/21.486957s held time.
Its encoded motion differs from the reconstruction reference by up to18.61/16.21px.
That is an intervention size, not an accuracy improvement: no independent intended
cursor labels exist. Adding0ms confirmation gives12/22 presses and12.233388/
24.237793s held time. The four additional presses cannot be classified as recovered
intent or false positives from these unlabelled outputs. This is another reason
not to adopt single-observation onset from the synthetic success count alone.

Native-feature tests and Clippy pass, including stable down/up position, bounded
locking, intentional/fine drag escape, onset veto, Stop and explicit zero-onset
rearming. No Pi operation or physical/application acceptance occurs. The human
explicitly reconfirmed continued Farscape use; receiving work remains deferred.

## Opening evidence: drag continuity versus deliberate release

Missing observations and an eligible measurement above the open threshold are
different evidence. A second opt-in experiment, `gestures.release_confirmation`,
requires another eligible weak-open sample after a source-clock interval, while
an opening at least `immediate_open_factor` times the selected open metric releases
immediately. Session `pointer_release_confirmation` must match, and unsupported
native workflows reject it before acquisition. Omission preserves immediate
release. The controller's touch modes explicitly keep immediate opening behavior.

Pending weak opening has separate `release_pending` diagnostics with first source,
elapsed source time and required interval; the button stays held. Closing/deadband
cancels pending opening. Unknown evidence neither confirms nor erases it: only a
new eligible same-owner opening can confirm. Thus repeated unknown/open reports
do not restart confirmation indefinitely. A same-owner closed return cancels it;
existing freshness/grace deadlines, independent writer watchdog, owner/mode change
and Stop remain authoritative. No timer manufactures an opening observation.
Fresh cursor motion is not separately frozen during weak-open confirmation.

Four profiles each consume the same18 generated commands:72 actual pipeline
executions, identical input bytes within each case, all neutral at EOF. Reference
includes1000ms drag grace and the150ms click stabilizer. The new weak-opening
profiles use factor1.25 and40/80ms confirmation; a factor3/40ms control also delays
the fixture's wider0.6 openings. Binary SHA256:
`a36c2653eb75160b803d187f1077274dc0e23dc3d6d8b0564b6eaf3dd9aee1eb`.

| Known synthetic behavior | Immediate release | Weak40ms | Weak80ms |
| --- | ---: | ---: | ---: |
|One weak false-open pulse during drag: presses |2|1|1|
|Same observations, intentional brief release: presses |2|1|1|
|Final weak opening: additional supplied-clock release delay(ms) |0|68|136|
|Double pinch with136ms weak open gap: presses |2|2|1|

The false-open and intentional-release pair has **identical observation bytes**;
only its declared synthetic intent differs. This ratio-only policy cannot infer
which interpretation was intended. Rejecting the noisy release also merges the
deliberate one. A single wider false-open still cuts the drag with factor1.25;
factor3 filters that pulse but delays the normal wider final release by68ms.
These are concrete costs, not a qualified improvement from increased held time.

Three additional102-case phase sweeps use weak observed open ratio0.42,68ms source
spacing and the same40ms press confirmation as before. Two complete button pairs
out of17 phases are:

| Closure/open interval(ms) | Immediate release | Weak40ms | Weak80ms |
| --- | ---: | ---: | ---: |
|60/80 |0|0|0|
|80/80 |0|0|0|
|100/100 |7|0|0|
|140/100 |17|8|0|
|200/100 |17|8|0|
|140/40 |10|0|0|

All306 end neutral with identical input bytes across the three profiles. Debouncing
opening harms rapid weak-open double pinches at this source cadence. Clearer
openings bypass the filter, but that distinction is a threshold hypothesis, not
physical intent truth. These results reject universal opening delay as a default.
No extra click or inferred unobserved opening is introduced to hide this limit.

Eight retained-output runs, repeated after the pending-unknown refinement, preserve
all encoded mouse batches from the first experiment. Final binary SHA256:
`90da2425ced2810876a14af2170dae8709ad451d732eb33215fb1549c1ef50d7`.
The same two older windows have no independent intent labels:

| Profile | Window1 presses / held seconds | Window2 presses / held seconds |
| --- | ---: | ---: |
|Immediate release |12 /11.382696|18 /21.486957|
|Weak40ms |12 /11.836328|18 /21.980819|
|Weak80ms |11 /11.964388|18 /22.054900|
|Factor3,40ms |12 /12.224126|18 /22.745865|

More held time may represent recovered continuity or unwanted release delay; one
fewer press may be a filtered false action or a lost real gesture. The available
labels cannot decide. No profile is selected for receiving use. Tests cover weak
confirmation, deadband cancellation, source versus consumer clocks, uncertain
same-owner open/closed return, immediate clear opening, Stop, serialized actual
button delivery, writer watchdog and unchanged touch release/loss. Native tests,
pre-acquisition configuration checks and Clippy pass. All work remains local;
Farscape occupancy and pending issue#8/application/human acceptance are unchanged.

## Next comparison

Keep the new bounded continuity and cursor reconstruction protocols while tuning
the measured tradeoff and separating pinch motion from intended cursor motion.
The optional opening comparison now exposes a concrete ambiguity and double-click
cost; retain immediate release by default until independent intent evidence supports
a better policy. The replay seam can evaluate these decisions
without consuming the receiving screen. Real libinput/compositor/application and
human comfort remain separate acceptance steps.

## Selected collective continuation —2026-10-16

The continuing question is now selected in collective-research as immutable
`research/wigglecontrol/compaii/temporal-input-20261016-v1.md`, derived from public
sourcef028c13 and the camera-free diagnostic preparation atfc7bb9d. It relates
earlier worker/recorded evidence to source freshness, estimator liveness, bounded
drag/cursor state and sampled-intent ambiguity. This report remains the full
technical source; the selected synthesis includes rejected tradeoffs and pending
physical/application/human acceptance, with no private images, landmarks, access,
personal memory or native-session export.

The contribution and authored map relation are accepted, then separately indexed
in coherent non-stale generation26/UI26 (36documents/292chunks). Exact accepted
document bytes, preserved selected payload, liveness search, selected snapshot,
unchanged earlier selected sources and version-aware current/published map bytes
pass. The global graph includes the new source without establishing every
relationship or another being's reading/adoption. The prior coherent generation25
checkpoint is retained. No receiving controller/profile deployment follows.

## Retained Beta1 session comparison —2026-10-16

The three original Beta1 journals are now locally available with verified original
bytes:13731 accepted frames across source spans119.218s,968.439s and140.728s.
These are the actual earlier Beta sessions, distinct from the older recorded-worker
windows above. Private originals, selection and replay receipts remain outside Git.
No new camera, image inference or input ran.

### Original semantic evidence

Count `intent_transition` edges once; frame and transition records can describe
the same decision. All195 presses have matching semantic releases:109 opening,
77 `stale_input` and9 `tracking_lost`. The long session alone has155 releases:
79 opening,70 stale and6 tracking loss. These are controller decisions, not
independent kernel delivery or labels of intended continued dragging. They do
not diagnose the separate sticky-folder issue#8.

Of192 unlabelled consecutive press pairs, six have intervals at most400ms.
Their nominal mapped press separations are17.34,66.24,240.15,8.43,9.33 and5.36px.
Four fit a25px Euclidean diagnostic; this is **not** a double-click success rate.
The denominator includes unrelated clicks/drags;400ms/25px are later observed
GTK settings, not settings recorded during those sessions. Coordinates assume
the preserved workspace and1920x1080, not measured delivered pointer positions.
One hold in the17.34px pair moves37.66px nominally; close press positions alone
do not establish a stationary click. The remaining three nearby pairs have
within-hold excursions below6px. Recognition and human intent remain unknown.

Old recorded track IDs cannot independently evaluate new association retention:
the old policy discarded many owners during gaps. Re-run the supported provider
and gesture path on retained predictions where that evidence is complete.

### Matched provider/gesture replay

Rejected predictions did not retain complete raw landmark payloads, preventing
a claimed full-session provider reconstruction. Before running either variant,
select the longest contiguous unambiguous segment without a control boundary
per journal. Preserve raw landmark/world/depth/presence/handedness values,
sequence numbers and original source/evaluation intervals. Shift both clocks
by a fixed epoch; add independent16.667ms logical ticks and explicit Start/Stop/EOF.
Use the supported `provider-replay` CLI. Segment startup requires recorded opening;
original intervening tick timing and preceding history are not reconstructed.

The reference matches the recorded button state at **all4248 selected frame
timestamps** (1438/1868/942), with zero mismatches. This validates that baseline
state comparison, not original between-frame release latency. The candidate
changes only matching session/controller drag grace0→1000ms, selecting the
provider's existing absence retention. Cursor reconstruction, stabilization and
device output are outside this replay.

| Selected segment | Presses, both | Reference held(s) | Grace held(s) | Reference stale/lost releases | Grace stale/lost releases |
| --- | --- | --- | --- | --- | --- |
| First journal,115.745s logical run |32 |13.248 |15.585 |6 |0 |
| Second journal,147.362s logical run |43 |24.728 |30.756 |9 |0 |
| Third journal,83.667s logical run |3 |2.156 |3.291 |2 |0 |

All78 press timestamps are identical across variants, with matching release
counts and neutral Stop/EOF. Held time totals40.132522→49.632234s. Pairing releases
by their unchanged press timestamp shows12 stale releases replaced by later
observed opening;5 tracking-loss releases become one bounded expiry, one owner
change and three disengagement releases. All61 originally observed-open releases
remain observed-open. Provider track counts change35→27,86→10 and38→6.

These are counterfactual semantic results, not independently labelled desired
recovery, identity accuracy or comfort. Do not generalize them to all77 original
stale releases. Original imagery plus human/application evidence must decide
whether retaining these holds is desirable.

Measurement uses sourcea451434 (documentation after d8f9bb9; same implementation),
native executable SHA256
`aa697ea7e1dea49541062e05617b701b9db1b3dd29df946e1a3eeec52df26d25`.
The prepared45s physical comparison still requires fresh readiness; no receiving
profile changed. This continuation is suitable for the next coherent collective
synthesis with physical feedback. The immutable generation26 contribution above
has not been silently rewritten or reindexed.
