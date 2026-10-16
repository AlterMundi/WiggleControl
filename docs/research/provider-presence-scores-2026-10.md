# Actual provider presence scores —2026-10-16

CompAII, continuing[#3](https://github.com/AlterMundi/WiggleControl/issues/3) and
[#7](https://github.com/AlterMundi/WiggleControl/issues/7).

The [previous measured shortcut](provider-visual-macros-2026-10.md) raised the SDK
presence floor0.5→0.8 to meet the existing macro confidence gate. It lost useful
near holds even with the classifier disabled. This investigation instead exports
the detector's actual per-hand presence score while leaving its floor unchanged.
The macro/classifier confidence gates are not reduced.

## Source and installed capability

The upstream[Web implementation](https://github.com/google-ai-edge/mediapipe/blob/7dd8fe00c17e7732818ea1406c4fb64beb237e6c/mediapipe/tasks/web/vision/hand_landmarker/hand_landmarker.ts)
returns landmarks/world coordinates/handedness without presence scores. Its
[inner detector graph](https://github.com/google-ai-edge/mediapipe/blob/7dd8fe00c17e7732818ea1406c4fb64beb237e6c/mediapipe/tasks/cc/vision/hand_landmarker/hand_landmarks_detector_graph.cc)
contains presence/score vectors; the[outer graph](https://github.com/google-ai-edge/mediapipe/blob/7dd8fe00c17e7732818ea1406c4fb64beb237e6c/mediapipe/tasks/cc/vision/hand_landmarker/hand_landmarker_graph.cc)
deduplicates hands. Those sources guide interpretation; the installed bundle is
separately qualified, not claimed built at that source commit.

Actual receiving Tasks Vision1.1.1 module SHA256 is
`a84921e02321e582484c1f8126d344b5f33b791fa09c640a52e20ee8632068f1`.
Model bundle SHA256 remains
`fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1`.
The installed object exposes a binary graph reader:51 expanded calculator nodes
were inspected. Deep `toJSON` triggers an actual retained asynchronous serializer
assertion; reading only the known name/stream fields avoids it. The exact
underlying unsupported option/type is not diagnosed.

## Qualified export method and failed alternatives

Rebuilding the expanded graph fails with actual ModelResources cache/service
errors. Attaching a listener without declaring the internal output leaves scores
unavailable, despite successful inference. Neither is reported as a working
export. The successful method intercepts the ORIGINAL hierarchical binary graph
when supported `setOptions` regenerates it, adding repeated output-stream fields
before normal SDK resource construction. No SDK/model bytes are changed and no
second hand model executes. Internal access is not a promised public SDK API;
the exact module digest and expected topology are mandatory guards.

The[reusable observer](../../tools/synthetic/provider_presence.mjs) discovers
score, presence-flag and pre-deduplication landmark streams with the same loop
boundary. It copies final serialized landmark packets while preserving the
SDK's own parser/callback. Presence flags first filter rejected ROI attempts;
exact byte correspondence then associates scores with final hands, including
SDK deduplication. Array index alone never supplies this association. Missing,
ambiguous or stale quality stays unavailable. The native bridge actually emits
booleans as numeric0/1; only those exact values and booleans are normalized.

Initial one-hand actual GPU execution at floor0.5 yields presence scores
0.906738/0.970215/0.970703 with matching1/2/3ms test timestamps. Model-close is
acknowledged. Cold processing536.5ms and two warm39.3/40.4ms samples are diagnostics,
not sustained transport or acquisition-to-application latency.

## Same-image baseline and observer comparison

Four fresh GPU models process two synthetic scenes, baseline and observer for
each. Each25-input case uses three identical warmups,21 JPEG trajectory frames
and a final black640×480 negative.100 actual model executions therefore supply
50 matched baseline/observer pairs, not100 independent images/human trials.
All graphs use maxHands2 and detection/presence/tracking floors0.5; source-time
VIDEO stamps advance100ms. Source images/outputs, code hashes and actual display
receipts remain private. Original reference/recordings/calibration are untouched.

| Scene /25 inputs per profile | Baseline counts | Observer counts | Exact public-output pairs |
| --- | --- | --- | --- |
| One hand |24 single,1 absent |24 single,1 absent |25/25 |
| Two hands |16 dual,8 single,1 absent |16 dual,8 single,1 absent |25/25 |

The complete public result (image/world landmarks and handedness) is exactly
identical in all50 pairs.64 actual returned hands receive attributable same-frame
scores, with no quality mismatch. Four one-hand frames have two admitted raw
predictions but one final SDK hand; correspondence survives actual deduplication.
A rejected second ROI example has scores0.928711/0.302002 and flags1/0; the sole
returned hand receives0.928711. One dual example receives0.993164/0.994629.
Both final black negatives contain no hand. The pair scene's eight single-hand
observations remain detection misses/limitations, not repaired by score export.

Warm medians over the21 trajectory frames are112.8/108.6ms baseline/observer for
one hand and86.2/78.2ms for two. These short sequential model-only runs do not
establish a speedup or runtime latency ranking. An earlier six-input jump test
failed to establish any dual-hand case; it is retained rather than counted as
two-hand qualification. Actual boolean0/1 normalization was corrected after a
separate first-frame validation failure. Those failures do not become successes
because a later fresh-model comparison passes.

All four models close and the worker acknowledges final closure. The experiment
uses the owned TV workbench, but its retained DOM shows the preceding experiment:
the workbench polls an older run-state file every600ms, which can overwrite the
probe's updates. This run does not qualify visible progress or the intended blank
negative presentation. Subsequent probes must isolate that old state producer;
the numerical inference/closure evidence remains separate. No camera,
virtual input, background model attention or human gesture trial occurs.

## Rust meaning connection

Opt-in session `provider_presence_scores:true` connects the observer through the
bounded raw provider, worker transport and shared classifier. The explicit SDK
floor still matches the controller's input gate (the comparison uses0.5). A
provided measured score must be finite and in `[floor,1]`; providing it without
selecting the adapter is rejected. Missing quality uses only the explicit floor.
Presence scores are model outputs, not calibrated probabilities or participant
identity.
At floor0.5 that cannot invent0.8 macro evidence. Actual hand confidence still
caps classifier evidence in the existing controller; ownership, positive-neutral,
input priority, hold/cooldown and freshness rules are unchanged. Disabled profiles
do not install the observer. No native hand-model path or everyday default changes.

Actual local source-time Rust replay consumes the retained observer outputs,
including64 measured hands, with source/config/binary hashes retained. Matching
floor-only and measured-quality/classifier profiles preserve contacts/buttons,
mouse events, mode/running/action/owner/reason across all28 commands/EOF outputs
per scene. Contact-containing frames remain5 and13 respectively; there are no
macro actions and Stop/EOF ends neutral. This is conditional semantic parity,
not another image test, physical intent label or live transport measurement.

New JS tests cover ROI filtering, reversed final order, dedup/ambiguous bytes,
missing/stale data, numeric flags and bounded graph export. Rust tests cover
selection, measured-score ownership, unavailable quality, malformed scores and
unchanged visual policy0.8 with tracking floor0.5. The complete native-feature
suite and Clippy pass. Full ARM64 worker→Rust transport and recognition/physical
qualification remain separate receiving evidence.

## Remaining goal work

Qualify the full real-clock receiving connection and useful contact coverage at
unchanged0.5 input floor, then broaden continuous neutral/action recognition,
negatives, ownership and participant modes. Existing weak synthetic pose scores
remain weak; this export does not train or improve the classifier. An actual
timed macro must follow recognized neutral and hold. Fresh acknowledged camera,
application delivery and seated comfort remain required. Preserve the separately
prepared physical trial and existing runtime/profile/power throughout.
