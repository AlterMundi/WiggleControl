# Goal: a fluent, responsive one-hand camera controller

Evolve WiggleControl's existing one-hand Beta1 into the most accurate, responsive,
comfortable and reliable everyday desktop controller we can achieve on the
current receiving hardware. Prioritize this goal before returning to the broader
multihand/multipoint touchscreen objective, which remains preserved in ROADMAP.md.
Iterate autonomously through recorded trials, simulation, mathematical models,
implementation and measured receiving experiments. Complete human/application
acceptance before declaring the controller finished.

## Human feedback motivating this goal — 2026-10-16

Nicolás reports that the current beta generally works reasonably well: hand
tracking looks good, even during rotations, and the pointer reaches its intended
target. Pinch state feedback looks accurate. However:

- A double pinch does not reliably produce an application double-click.
- Drags often end prematurely during an apparent tracking interruption, then do
  not resume when the closed pinch returns.
- Pointer movement looks like discrete jumps rather than continuous motion;
  stationary stability and pinch-induced cursor displacement need improvement.
- A desktop folder drag also remained attached after release and source shutdown.
  This separate delivery failure is tracked in issue #8; its cause is unknown.

These are attributed human observations, not frame-level intent annotations or
proof that perception is perfect. Check them against retained images, predictions,
source clocks and application events. Three actual beta sessions have retained
private camera recordings and controller journals; do not claim they include the
desktop, audio, every intermediate instant or independently labelled intent.

## Intended architecture and behavior

Develop the proposed middle layer between observed hand data and delivered mouse
input, extending the existing perception/controller/writer separation and filters.
Keep observed evidence, estimated interaction state and actual delivered effects
distinct and inspectable. Reuse supported configuration and behavior before
adding code; avoid rewriting already working perception without evidence.

1. Establish trustworthy end-to-end button delivery first. Reproduce issue #8
   using empty owned test folders without camera acquisition. Compare semantic
   decisions, the writer's actual kernel events and real application drag/drop.
   Qualify press, release, Stop and device/producer failure. Fix terminal receipt
   fields that currently retain historical camera/input acquisition flags.
2. Make click, double-click and drag deliberate and consistent. Measure the
   actual application's timing and spatial requirements, pinch confirmation and
   rearm, and cursor movement during pinches. Preserve normal click/drag behavior
   while making double-click easy. Compare natural repeated pinches and an
   explicit configurable alternative only if useful; do not silently introduce
   extra clicks to hide recognition failures.
3. Add temporal continuity for an already established drag. Distinguish a measured
   open-hand release from a missing/uncertain sample. Test a bounded grace period,
   including Nicolás's proposed approximately one-second candidate, rather than
   dropping immediately on a transient gap. Freeze uncertain motion initially;
   resume the same drag only with adequate same-hand evidence within the grace
   interval. End it on confirmed release, timeout or explicit Stop. Do not
   re-press after a completed release or transfer a held button to another hand.
   Design this separately from the independent writer/process-health watchdog;
   an alive estimator may retain uncertain interaction state without relabelling
   old observations as fresh or leaving input held after its process dies.
4. Improve cursor placement, stability and visual continuity together. Compare
   the existing exponential and One Euro filters, hand anchor choices, click
   stabilization, timestamp-aware velocity estimation and an independent smooth
   output cadence. Evaluate short-horizon prediction/interpolation with bounded
   speed, acceleration and overshoot; a one-second drag grace is not permission
   to extrapolate motion for one second. Stop or decay prediction when evidence
   disappears, handle reversals/stationary targets and reconcile measurements
   without snapping. Avoid buying apparent smoothness with excessive lag.
5. Qualify the integrated experience on the receiving desktop. Keep existing
   calibration, working physical input, power profile and reversible deployment.
   Retain visible state for actual experiments: observed/estimated cursor, drag
   held/grace state, prediction age and actual input activity when relevant.
   Let Nicolás contribute visual and comfort observations during real work.

## Iteration and completion

Use the retained private footage and journals first, preserving original bytes,
timestamps and prediction/annotation provenance. Add deterministic trajectories,
dropout durations, ambiguous returns, rotations, reversals, tremor and stopped
producer cases. Compare candidates on identical inputs and actual elapsed time.
Measure click/double-click success, unintended actions, drag completion and
premature release, stationary jitter, target error, motion continuity, latency,
overshoot and resource use. Do not optimize only an aggregate score or one clip.

Keep real experiments visible on Cuevanico while doing useful work, not by
replaying demonstrations solely for display. Research/replay must not acquire a
camera. New physical trials require the established human-ready/manual Start
boundary and private video retention. Coordinate finite desktop injection;
protect human files with owned scratch tasks. Do not resume the old workbench,
start capture automatically, contact peers or initiate a global Stop.

Maintain BETA-1.md, ROADMAP.md, the relevant issues and the receiving runbook with
what is implemented, deployed, observed and still uncertain. Evaluate substantial
reusable findings for collective research; keep raw private recordings and native
conversation history out of that corpus. Do not turn routine delivery into a
research publication requirement.

The goal is complete when the integrated one-hand controller passes reproducible
regressions and real application tasks, and Nicolás accepts that click,
double-click, sustained drag, gap recovery and pointer movement feel reliable,
responsive and comfortable in seated everyday use. Report measured limits and
remaining failures honestly. If hardware/perception imposes a demonstrated limit,
document the evidence and best working tradeoff; do not claim perfection.
Only then return to the broader multipoint touchscreen roadmap under its own
goal. This file defines the replacement objective; it does not edit or resume the
native harness goal by itself.
