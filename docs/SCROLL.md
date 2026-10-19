# Held fist scroll: direct content movement

Nicolás selected this interaction on2026-10-19: a closed fist grabs the page;
vertical, horizontal and diagonal hand motion moves its content in the same
screen direction. Opening while moving releases a short decelerating glide.
Opening after stopping has no inertia. This is a source candidate; receiving
application direction, gesture comfort and phone-like fluency remain unqualified.

## Recognition and ownership

Controller settings opt in with `"scroll": {}`; omission/null disables the gesture.
The selected fully in-view hand supplies raw world finger geometry and its
normalized mirrored-image wrist position. Existing native/GPU/provider-replay
bridges share admission and meaning; no new model or inference is introduced.
All four fingers must be curled and the thumb tip inside the palm rather than
extended outward as for Escape. Geometry is scale/rotation/reflection invariant;
that is not a claim about anatomical realism or perception accuracy.

Default positive open rearm is200ms and fist confirmation300ms. Open means four
extended fingers and both configured pinches positively open. Missing, stale,
out-of-view, low-confidence, ambiguous and changed-owner input cannot arm the
fist or initiate inertia. Acquisition with an already closed fist does not enter
scroll. An existing press/drag is not converted into scroll by curling fingers;
Escape/Stop and established bounded drag release remain separate.

The temporary effective mode is exclusive Scroll, reusing the writer's existing
mode/source fences. It suppresses pointing/clicks and freezes the last delivered
absolute pointer. Opening returns to Pointer with a continuous screen/detail
anchor at the visible point. Detail remains an independent pointer modifier.
Escape first neutralizes gesture scroll and arms Pointer before its one-shot key
transaction; it does not compete with wheel delivery.

## Fine motion and optional acceleration

The gesture emits `scroll_fine` with vertical/horizontal integer units,120 units
per wheel detent. Both Linux device variants advertise matching high-resolution
axes. The adapter accumulates legacy detents and emits compatible coarse events;
legacy `scroll` intents emit both representations. Release clears wheel residuals.
This follows the [kernel high-resolution wheel contract](https://www.kernel.org/doc/html/latest/input/event-codes.html#ev-rel).
A new producer requires its matching writer; previous vertical-only replay JSON
retains its serialization and behavior.

In screen coordinates, direct upward content motion becomes negative vertical
wheel units, while leftward content motion becomes positive horizontal units.
The native axes have different signs; do not invert both with one multiplier.
[libinput documents scroll direction and natural-scroll configuration](https://wayland.freedesktop.org/libinput/doc/latest/scrolling).
Actual compositor settings/application content movement still require receiving
verification. Do not change physical-device settings to manufacture a passing test.

The wrist avoids fingertip-articulation displacement. Per-axis radial play absorbs
bounded rest noise without accumulating creep, while sub-detent remainders retain
small deliberate travel. A real reversal can have a bounded settling displacement;
this is not a promise of zero physical jitter. Large discontinuities reanchor
without queued wheel bursts. Both axes are emitted together, without axis locking.

`steps_per_unit` defaults to30 detents per normalized image unit, `rest_radius`
to0.004 and `max_step` to0.2. `acceleration_max:1.0` is linear. Values above1
provide bounded smooth speed gain, with default reference speed0.8image units/s
and velocity smoothing80ms. Actual comfort, small-motion usefulness and cadence
need matched physical comparisons; more gain is not inherently better.

## Release inertia

Inertia is off by default. An explicit comparison candidate can use:

```json
{"scroll":{"acceleration_max":2.5,"inertia":{}}}
```

Only a positive open pose after recent measured fist motion starts coasting.
A short positively observed unfolding interval can bridge to open, emitting no
wheel itself. Unknown/lost input cannot initiate or sustain it. Fresh same-owner
open observations advance an exponential integral; source timestamps stay original.
There is no autonomous wheel extrapolation from a tick or stale frame.

Defaults:180ms decay,700ms maximum duration,0.18image units maximum travel,
150ms unfolding window and0.15image units/s minimum release speed. A resting
grip clears release velocity immediately; tiny boundary noise cannot preserve
old speed. Regrabbing arrests inertia and anchors the new grip without a burst.
A different gesture, pinch intent, loss, ambiguity, owner change, mode change,
Escape or Stop cancels it. The writer's independent300ms source-freshness policy
is preserved for the selected beta; inertia does not extend it.

Fine output currently advances on fresh observations, including coasting. Its
perceived smoothness is not a measured60Hz animation claim. Qualify actual content
motion before deciding whether bounded independent output reconstruction is needed.

## Source evidence and remaining qualification

235 native-feature tests pass, zero failures, one existing opt-in benchmark ignored.
Regressions cover geometry/visibility/real0.5admission, default-off behavior,
nonoverlapping Escape geometry, both axes/signs, fine/coarse wheel encoding,
rest/no backlog, loss/owner/duplicate/gap, pointer freeze/anchored return with and
without cursor reconstruction, moving/resting release, decay/regrab/cancellation,
acceleration, mode fences and Escape during scroll. Relevant cheap-status JS
checks pass. Public geometry fixtures are algebraic tests, not model ground truth.

Paired private provider replays cover3095 retained observations:248+393+1271+1183.
Scroll enabled with acceleration/inertia preserves all compared mouse/cursor/
readiness/button/owner/detail/transition/contact/mode/Escape values. Thirteen
press intents (11left/2right),478 detail observations and12 Escape outputs match;
no scroll candidate/action occurs. This compares retained predictions, with no
new inference, camera, intermediate writer ticks, application effects or newly
labelled human intent. It is not a measured ordinary-use false-activation rate.

Next: exact-source CI/artifact, separate receiving fine-wheel/classification and
actual content-direction checks, then a fresh manual-Start recorded human trial
of grab/pan/release/rest/regrab. Preserve accepted calibration/beta. Compare linear
and accelerated/inertial settings for comfort rather than declaring phone-like
quality from source tests. No Pi deployment or sensor session follows from this
source record. Full release scope remains in LINUX-PREVIEW.md and issue10/#13.
