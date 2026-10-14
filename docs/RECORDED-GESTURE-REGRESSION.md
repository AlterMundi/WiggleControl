# Recorded gesture regression proposal — 2026-10-14

Nicolás proposed recording repeated pinches/holds and iterating offline toward
correct recognition of every labeled gesture. This is a proposed workflow, not
an implemented recording/replay command or consent to start acquisition now.
Physical recording still waits for fresh human ready after receiving preparation.

## Public references

- [Dexter+Object](https://handtracker.mpi-inf.mpg.de/projects/RealtimeHO/dexter%2Bobject.htm): six hand/object sequences, including pinch, with annotated fingertips. Useful tracking/occlusion reference; grasping an object differs from free-air mouse pinch.
- [InterHand2.6M](https://mks0601.github.io/InterHand2.6M/): 30 fps sequence release and 3D joint annotations, useful pose/tracking reference. Not labels for our mouse-down/hold/up contract.
- [HaGRID](https://github.com/hukenovs/hagrid): static gesture-image collection, useful negative/posture diversity; does not by itself test temporal drag continuity.

These official sources were inspected. No bulk dataset download or retraining
was performed. Select relevant bounded subsets with their published terms and
provenance; do not imply every dataset directly qualifies our interaction.

## Local corpus

Record video only, on the existing camera/viewpoint/resolution, without injecting
OS input. Preserve raw frames/timestamps independently of tracker decisions so
missing/invalid detections remain inspectable. Retain config, source/model/runtime
hashes and measured processing times; recordings stay outside Git on the local
content disk. Existing capture/Stop infrastructure should be reused where it
provides the required timing; inspect it before adding another capture path.

Two short takes: development and untouched validation, split by whole take rather
than adjacent frames. Include each hand, ordinary/quick pinches, 3–5 second holds,
movement while held, deliberate release, resting/open hand and relaxed closed
poses with no intended click. Include a separate tracking-loss/return case.
On-TV prompts/countdown start after actual capture readiness. Expected actions
are human intent plus reviewed footage, never prompts or recognizer outputs
silently treated as ground truth. Mark exact down/up intervals and uncertainty.

## Offline evaluation

Feed the same native inference/gesture pipeline from recorded input; retain a
landmark/observation replay for cheaper gesture-only experiments. Keep perception,
gesture logic and OS adapter separate. Replay recording time for gesture duration;
evaluate measured processing latency and injected delay/stall cases separately,
since fast offline playback cannot establish real-time Pi performance.

Report missed/extra presses, release timing, false activations during rest and
premature releases or split presses during each labeled hold. Count successful
holds as uninterrupted gesture intervals, not aggregate pressed frames. Pinpoint
failures on an annotated timeline with fingertip geometry, ratios, identity,
eligibility, freshness and actual release reason. Preserve failing clips as tests.

Target zero labeled event errors on the development recordings, then evaluate
untouched takes and a fresh live OS-input trial. Perfect performance on a finite
corpus is useful regression evidence, not proof of universal 100% accuracy.
Change gesture code first when landmarks are correct; inference failure needs
its own correction and possibly additional model data. Retraining is not assumed.
