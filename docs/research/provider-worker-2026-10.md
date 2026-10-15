# Raw provider tracking and a Pi inference worker

CompAII,2026-10-15. Continues [palm/GPU measurements](palm-correspondence-gpu-2026-10.md).
The generic Rust controller remains responsible for gesture meaning and system
delivery. This increment connects raw provider points to that meaning without
inheriting native identities, confidence, eligibility or cursor. It also actually
runs the optional browser inference in a worker on Pi5. Camera capture, live
worker→Rust transport and application acceptance remain unfinished.

## Raw points to existing meaning

The opt-in native-feature command `provider-replay SESSION SETTINGS PRESENCE_FLOOR`
consumes bounded JSONL. Commands are `start`, `stop`, `tick`, `mode`, `select_hand`
and `frame:{observation:{sequence,source_us,now_us,width,height,hands}}`. Each raw
hand has21 pixel XY points,21 world XYZ points and `right_probability` in the
provider's handedness convention. That score is neither presence nor participant
identity. No model, camera, recording or virtual device opens in this command.

The adapter reuses native palm correspondence, duplicate/pair uncertainty,
handedness-change checks and the shared session observation conversion. At most
two hands are assigned using injective partial matches: corresponding-palm cost
below0.75 and best/alternative summed-cost margin at least0.25. These are explicit
experimental temporal gates, not calibrated identity probabilities. Ambiguous
assignments/overlap retire tracks. Missing hands, expired/future/regressing data,
changed image dimensions and invalid geometry release meaning; returning closed
hands do not inherit an old open/confirmation state. IDs never come from provider
array order, and track IDs do not identify people.

Tasks Vision exposes handedness scores but no individual hand-presence score in
this API. Therefore the adapter records the configured presence threshold as a
**lower bound**, explicitly labelled `configured_presence_threshold_lower_bound`.
The session/controller gate must match it; these experiments use0.5. It is not
an inferred probability or inherited native quality. Visual macros are rejected
at preflight because this wire does not provide image Z/classifier evidence.
The existing pointer/contact/mode engines, workspace, onset permission,
open-before-press, confirmation and Stop/EOF contracts remain authoritative.

Saved same-image CPU/GPU observations run through actual Rust tracking/meaning:
one-hand21 inputs retain one track and4/5 near-hold contacts with zero open-hold
contacts; separated two-hand31 inputs retain two tracks and20/20 expected stable
counts, with no unscored spatial ownership or observed owner transfers. This is
a narrow synthetic spatial-continuity result, not general participant identity.
All104 frames deliberately replay at source time; they do not prove live timing.
An artificial constant age300001µs rejects every frame against300000µs freshness.
Malformed/oversized input and EOF are each tested after an actual held replay
contact and emit an empty release. Unit controls cover array reordering with
same-handed hands, ambiguous assignments, overlap/loss, geometry/dimension/time
failure, watchdog expiry, handedness change and positive-open rearm.

## Actual classic worker on receiving Pi5

The optional [worker](../../tools/synthetic/task_hand_worker.js) owns one Tasks
model and an OffscreenCanvas. The parent transfers one ImageBitmap at a time,
retains source timestamps, awaits points and a close acknowledgment, then
terminates the worker. There is no acquisition, unbounded work queue or OS output.
The [comparison helper](../../tools/synthetic/browser_hand_sequence.mjs) selects
`execution:"worker"` explicitly; its existing main-thread path remains available.

Two actual startup failures precede the successful run. Chromium rejects worker
loading across the original `file://` workbench and the HTTP asset origin. Serving
the owned workbench temporarily from the same receiving loopback origin resolves
that restriction. An ES-module worker then fails `ModuleFactory not set`.
Inspection of the pinned1.1.1 loader finds its `importScripts`/global factory
initialization; a classic worker with dynamic import of the unchanged SDK succeeds.
No SDK patch or new dependency is installed. Failed receipts remain diagnostic
evidence, not model executions or fallback successes.

| Worker delegate | One hand,21 inputs: median/p95 ms | Two hands,31 inputs: median/p95 ms |
| --- | --- | --- |
| CPU |218.2 /316.8 |206.5 /213.3 |
| GPU |131.4 /152.4 |102.6 /110.5 |

Processing includes image fetch/hash/decode, bitmap transfer, inference and
readback. Worker-only GPU inference medians are107.8/78.7ms; parent-side
submission/inference/readback medians115.2/86.3ms. Initialization is separate.
The main thread executes animation callbacks during **every** worker inference:
GPU medians7/5 callbacks per inference, minimum4; CPU medians12/11. This observes
main-thread scheduling, not full display framerate or input latency. Temperature
peaks68.85°C under unchanged1.5GHz/fan-off selection; sustained qualification is
still absent. Same package/model/input hashes as the preceding report apply.

Worker outputs also pass the new Rust adapter on all104 inputs. Every emitted
contact matches the saved main-thread provider replay; one-hand4/5 and paired
20/20 holds persist. Rust runs offline on the VPS in this increment. The worker's
internal browser messaging works on Pi; a live same-clock camera/worker/Rust
pipeline is not implemented or measured here.

## Next connection

Have the receiving producer own acquisition timestamps and frame leases, with a
bounded latest-frame slot and one inference in flight. Feed returned raw points
to this adapter using the same host clock; retain actual frame age and independent
release/watchdog behavior when the worker is delayed, missing or stopped. Measure
whole-path cost and coverage on broader views/negatives/crossings before selecting
it. About103–131ms processing alone does not meet a consistent10FPS budget.
Keep crowding/unknown ownership explicit. Complete image-depth/macros and actual
mode/participant/application/human qualification as separate roadmap requirements.
The owned workbench retains the actual worker→offline-Rust result; the original
file viewer is restored and finite worker/asset tasks end. Working runtime,
calibration, power and original prepared human trial remain unchanged.
