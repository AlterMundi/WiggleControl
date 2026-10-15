# Palm correspondence and receiving GPU comparison

CompAII,2026-10-15. Continues [fixed tensors/mixed models](fixed-tensor-mixed-model-2026-10.md),
[#6](https://github.com/AlterMundi/WiggleControl/issues/6) and
[#7](https://github.com/AlterMundi/WiggleControl/issues/7). All experiments use
retained synthetic images; no camera, microphone or system input is activated.

## A false extra hand

The prior experimental `palm_landmarks` policy matched detector proposals using
five corresponding palm points but compared reconstructed duplicates using all
21 points. Two crop-dependent reconstructions of the **same** pinching hand had
normalized all-point RMS0.115542, versus palm-point RMS0.078207. Their handedness
scores differed by0.041059. The existing duplicate threshold0.10 missed the
duplicate, so `pair_overlap` cancelled an otherwise usable one-hand pinch.

Duplicate comparison now uses the same five proximal points `[0,5,9,13,17]`.
Scale,0.10 tolerance and handedness gate remain unchanged; no extra inference is
added. The existing default `roi` policy is unaffected. A regression separates
crop-sensitive distal motion from a genuinely neighboring palm and opposite
handedness. The targeted actual10Hz adverse scene recovers **4/5** near-hold
contacts, versus0/5 with the old experimental policy; the retained ROI control
also reaches4/5. This repairs correspondence, not occluded-hand observability.

The corrected producer then processes all **1042** retained images across25
trajectories:655 broad20Hz inputs,143 actual10Hz inputs and244 paired inputs.
Every input uses `palm_landmarks` and separation guard1.2. Against the previous
mixed-model reports (whose single-hand scenes used ROI/default guard and paired
scenes used palm matching/1.2), all primary observations, ambiguity and emitted
contacts remain exact. This is a uniform-policy compatibility result, not a
claim that every scene previously failed or a matched one-variable comparison
across the complete set. The targeted factor control above isolates the failure.

Near-hold coverage remains116/117 at20Hz and17/18 on the hard actual10Hz scenes;
V/open controls remain contact-free. Both separated pair layouts retain61/61
dual detections and41/41 correct stable-hold counts. Both crossings retain0/13
dual near holds. Same-side crossing still has five contact frames with unscored
ownership; an empty observed owner-transfer list does not establish continuity.
The correction is retained in the experimental policy, with default/runtime
selection unchanged. Full native-feature tests, fmt and clippy pass locally;
receiving and physical adoption remain separate.

## Actual browser CPU/GPU measurements on Pi5

The receiving Chromium reports hardware WebGL2 through ANGLE/Broadcom V3D7.1.7.0.
WebGPU is exposed but returns no adapter; no WebGPU inference was demonstrated.
The optional [browser sequence helper](../../tools/synthetic/browser_hand_sequence.mjs)
runs pinned Tasks Vision1.1.1, `VIDEO`, `numHands:2`, detection/presence/tracking
thresholds0.5, fresh CPU then GPU models per scene. Actual GPU graph/context
initialization and closure were observed. This does not establish that every
operation runs on the GPU.

| Requested delegate | Adverse one-hand,21 inputs: median/p95 ms | Separated two-hand,31 inputs: median/p95 ms |
| --- | --- | --- |
| CPU |221.6 /317.4 |206.0 /215.8 |
| GPU |132.2 /142.4 |102.9 /121.0 |

These processing times include loopback image fetch, SHA verification, decode,
synchronous inference/readback; initialization is separate. Inference-only
medians are201.9/184.8ms CPU and113.2/85.8ms GPU respectively. Both delegates
detect the expected hand count on every input. Different detector/reacquisition
work means the one-hand scene need not be cheaper than the two-hand scene.
The Pi retains its1.5GHz cap and selected fan-off mode; observed finite-test
temperature peaks are62.25/64.45°C CPU and66.65/67.75°C GPU. This is not sustained
fanless qualification or capture-to-event latency.

The package comes from the pinned
[npm1.1.1 archive](https://registry.npmjs.org/@mediapipe/tasks-vision/-/tasks-vision-1.1.1.tgz),
verified against npm SHA512
`jwntgT5tzDrKOkqg6xNDNpRcTUnGvmylZE5DFnzPXossiWOIDdZX+C81LI75hlXhsG58YZW+lGzcmjH1qrQE/Q==`.
Its Apache2 license and individual asset hashes are retained privately.
The unchanged public Tasks model has SHA256
`fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1`.
No package scripts, system installation or new live backend are required for
this optional comparison. Models close after each finite scene; the read-only
loopback asset server has a finite lifetime and is no longer running.

## Does the accelerated estimate retain useful meaning?

The saved CPU and GPU one-hand predictions are replayed through the actual Rust
controller against exact matched images/time/independent skin geometry. Both
provider metrics recover4/5 near-hold contacts, with no open-hold contacts; the
first contact occurs at frame9 with selected distal skin separation about1.5mm.
The retained primary metric alone reaches0/5. A same-frame conditional fusion
also reaches4/5. Native identity, eligibility, cursor and confidence are inherited
explicitly for this test; Tasks array order is never an identity. This qualifies
a narrow metric experiment, not provider ownership, noisy-human precision,
asynchronous timing or a complete accelerated system-input path.

## Decision

Hardware WebGL gives a reproducible speed improvement on this Pi. It remains a
candidate for an optional perception worker feeding the generic Rust controller;
it does not make WiggleControl a browser-only input device. The
[official web guide](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker/web_js)
documents synchronous `detectForVideo`; UI responsiveness requires a separate
execution path. Worker/IPC integration, bounded freshness, explicit hand ownership,
loss/rearm/Stop and whole-path latency still require implementation and evidence.
About103–132ms per image remains insufficient to claim a consistently fluent
10FPS controller, much less touchscreen latency. Crossing/occlusion, diverse
hands/backgrounds, useful macros and human/application acceptance stay open.
No selected receiving runtime, calibration or original prepared trial changes.
