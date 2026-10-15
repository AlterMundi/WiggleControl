# Position-dependent pinch investigation

Date:2026-10-14. This is an offline diagnostic, not a deployed gesture correction.
The human reports horizontal position-dependent estimates for closed/open poses;
vertical variation in actual camera use has not been tested by the human.

## Coordinate contract

The pinned OpenCV adapter identifies model output3 as metric world landmarks.
It rotates world XY when returning image-aligned coordinates; this rigid rotation
cannot change Euclidean fingertip distance. The native path keeps world points
in crop coordinates and computes norm(world[4]-world[8]), without image-width,
image-height or ROI-side multiplication. No missing world-distance scale factor
was found in that path. The world output is an estimate, not calibrated contact.

Sources:
- [Pinned OpenCV adapter](https://github.com/opencv/opencv_zoo/blob/47534e27/models/handpose_estimation_mediapipe/mp_handpose.py)
- [MediaPipe coordinate contract](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker/python#handle_and_display_results)

## Retained real recordings

Examined four private takes,3582 detected-hand observations in total. Raw gap,
world bone dimensions, projected hand position and projected fingertip separation
were derived without new inference, sensor access or OS input. In the longest
recording, for fully visible hands with projected tip separation under0.2 palm
widths, the left-center region had35 observations with median estimated gap22.0 mm
and wrist-to-middle-MCP length87.98 mm. The center region had32 observations with
median gap49.7 mm and length93.74 mm. Their index-MCP-to-little-MCP widths were
71.69 and70.15 mm respectively. Regions refer to raw image coordinates, not mirrored
screen coordinates. These are associations, not independent sample counts.

The whole hand does not simply scale by the same factor as the gap. Several
small-projected-gap frames have world fingertip separation dominated by predicted
Z, with approximately90–98 mm in that component. Frame inspection retains pose
and occlusion uncertainty: projected overlap is not proof of physical contact.
The footage changes pose/orientation/position together and has no reviewed event
labels. Therefore it cannot yet establish a position-only correction law, even
though it contains variation consistent with the human's report.

## Controlled input generator

```sh
wigglecontrol position-sweep IMAGE CONFIG NEW_DIRECTORY
```

Requires the existing native-vision build and pinned models/runtime. It reads an
image, detects a fully contained reference crop, copies that unchanged patch to
nine integer-translated positions on the same-size canvas, and saves synthetic
PNG frames and a JSON report. The output directory must not exist. No rescaling,
pose change, camera, microphone or virtual input device is used. Human-derived
inputs and generated frames stay private, outside Git.

Two separate branches prevent circular reasoning:

1. Known translated ROI: compare exact model input tensors and raw world outputs.
2. Cold palm detection followed by ten identical frames of normal tracking.

A third experimental branch uses MediaPipe's proximal-joint ROI recipe, rather
than all fingertip extrema, to test crop sensitivity. This is confined to the
probe, not enabled in live inference. Generated frames control translation and
constant appearance, not absolute physical fingertip distance. They are not a
rendered3D human or a continuous natural-motion validation set. The copied scene
also changes background boundaries, so inspect fixed-crop and end-to-end results
separately.

## Actual model results

Two different recorded poses were tested at all nine positions; ten repeated
frames per position and per tracking strategy. Values below are estimated gap mm:

| Pose | Fixed corresponding ROI, all positions | Cold detection range | Normal tracking frame10 range | Experimental proximal frame10 range |
| --- | --- | --- | --- | --- |
| A |68.826 at every position |53.626–91.926 |98.698–99.435 |85.753–99.581 |
| B |153.385 at every position |79.028–166.754 |158.708–159.005 |152.607–152.788 |

All corresponding fixed-crop input tensors matched exactly (maximum element
difference0), and their predicted world distances matched exactly. Cold detection
introduces position-dependent crop differences; normal tracking converges closely
across positions for these stationary inputs. It also changes the distance over
repeated identical frames. This demonstrates crop/model sensitivity, not a simple
erroneous final Euclidean transform or proof of actual camera lens distortion.
The proximal alternative is not uniformly better and remains experimental.

The diagnostic source uses the recipe from these pinned upstream references:
- [Proximal landmarks and ROI orientation](https://github.com/google-ai-edge/mediapipe/blob/f37ca10a57ac215431bf1448129054869fc191d9/mediapipe/modules/hand_landmark/calculators/hand_landmarks_to_rect_calculator.cc)
- [ROI expansion](https://github.com/google-ai-edge/mediapipe/blob/f37ca10a57ac215431bf1448129054869fc191d9/mediapipe/modules/hand_landmark/hand_landmark_landmarks_to_roi.pbtxt)

## Correction criteria and next experiment

A fitted correction can compensate a repeatable model bias even if its origin is
not optical. It may require an offset as well as a gain. However, do not fit a
screen-position-only function to footage with changing posture or silently use
recognizer states as reference truth. A global palm-size normalization is not
supported by the unequal gap/bone changes above. No live thresholds or production
crop policy have changed during this investigation.

Next use deterministic articulated geometry with explicit finger-tip surface
markers, joint transforms, camera projection and known contact/gap. Sweep pose,
view angle, depth, position and crop scale independently, retaining no-detection
cases. Compare controlled renderer inputs with the existing real takes to expose
domain mismatch before fitting a correction or qualifying pinch/drag. Generative
video alone cannot establish these physical labels. The image-translation probe
already tests its narrower invariants without human annotation.

## First articulated3D render control — 2026-10-15

An actual MHR LOD1 rig was imported and rendered through the isolated Python3.11 /
bpy4.5.3 CPU backend, using two render threads. `tools/synthetic/` retains the
portable source, public asset/license/hash manifest and reproduction instructions.
The cached environment and downloaded assets are outside source Git; no system
packages or receiving integrator software changed.

Eleven rendered frames vary horizontal/vertical translation, distance, yaw and
pitch independently while holding hand articulation constant. Evaluated rig
marker gap stayed at113.184 mm, invariant within0.001 mm. All reference markers
projected inside the images; that is not proof of unoccluded skin visibility.
The21 marker mapping is an explicit rig-joint convention, not qualified physical
fingertip contact. These are image controls, not a pinch animation.

Actual cold-detection results (no tracking warmup) were:

| Case | Estimated gap mm | Detection |
| --- | --- | --- |
| Reference |89.39 |yes |
| Horizontal negative /positive |80.06 /93.46 |both |
| Vertical negative /positive |90.05 /86.61 |both |
| Distance negative /positive |92.18 /87.09 |both |
| Yaw negative /positive |76.63 /96.47 |both |
| Pitch negative /positive |unavailable /99.46 |one missed |

This establishes estimate variation with known constant synthetic articulation
and marker distance, including a detection miss that must stay in the result.
It does not identify real-camera lens parameters or directly qualify a correction
for a closing finger, a different human hand or warmed-up live tracking. The
recorded-pixel and articulated controls answer different questions; neither
justifies claiming the live pinch problem solved.

The optional near-finger pose candidate was subsequently rendered at all eleven
views as well, holding17.568 mm between the declared rig markers. Pose joint
limits and skin contact are not qualified; those frames are geometric controls,
not labeled real pinches. `image-check IMAGE CONFIG --repeat 10` now reuses the single-hand tracker for ten identical inputs,
retaining cold and warm outputs without opening sensors. Default image-check
remains a single inference; repetitions are bounded to1..60.

For the open/near candidates, after ten repetitions at the central camera, gaps
were87.07 /35.15 mm; at negative horizontal translation80.71 /31.13 mm; at positive
horizontal translation90.95 /32.95 mm. More significantly, negative yaw gave
76.50 /71.05 mm: a near pose's estimated gap almost reaches the open pose's. One
pitch direction failed detection for both poses. These retained failures make
a position-only correction insufficient to qualify this view range. A gain or
offset learned only from the horizontal sweep would conceal, not resolve, this
orientation-dependent separation loss. No such filter was enabled live.

An independent intermediate interpolation was also rendered and evaluated as a
third eleven-view sweep. Its marker gap is84.474 mm; after ten repeated inputs,
the central estimate is44.742 mm, horizontal controls39.433 /54.182 mm, and yaw
controls21.257 /96.896 mm. The same pitch direction remains undetected. Across
all33 cases,30 were detected after warmup and three misses were retained.
The two endpoint poses therefore cannot establish a general scalar gap
correction: held-out articulation behaves differently. Source-level rendering
and observation are reproducible, but anatomical pose/contact mapping and
synthetic-to-real transfer remain unqualified. No fitted correction is deployed.

## Human-visible review corrects the render interpretation — 2026-10-15

Nicolás inspected the experiment on Cuevanico's TV and reported that the final
pitch case showed a very close patch of skin rather than a recognizable hand.
The initial response cited hand-marker distances without inspecting the actual
displayed input. That response was inadequate: projected/in-frame joint markers
and their distances do not establish which surface is visible.

Inspection confirmed the negative-pitch input is body-occluded. In the retained
near-finger scene, a center-camera ray hits `body_mesh` at **3.718 cm**, while the
wrist marker is **61.852 cm** from the camera. The corresponding reference-view
ray hits at **63.246 cm**. Rotating the whole body about the hand brought another
surface into the camera's view. This is a defective hand-visibility fixture,
not evidence of a detector failure on a visible hand. The three earlier
negative-pitch misses remain retained, but are **invalid inputs for hand-detector
evaluation**. The original 30/33 count describes raw outputs, not acceptance over
33 valid hand images. Other view-dependent results remain observations with
their original geometry and input limits.

The optional renderer now supports `--hand-only`, masking geometry with summed
right-wrist/finger skinning weights >=0.5, and `--yaw-steps N` for bounded new
views from -35 to +35 degrees. Original full-body inputs remain preserved.
The mask changes image context, including the forearm, so differences from
older full-body estimates cannot be attributed solely to angle or a corrected
model. Masked hand geometry is not independently qualified anatomy/contact.

A new nine-view hand-only near-finger sweep completed. Its declared marker gap
remained approximately **17.568 mm**; frame10 model estimates ranged from
**25.183 to 35.729 mm**. At -35 degrees the masked input yielded **28.312 mm**,
where the earlier full-body input yielded **71.051 mm**. This motivates isolating
image/crop/context effects; it does not validate a live correction. The live
producer projected new rendered images and measurements to the RPi. A network
interruption was retained and resumed using completed renders/results, evaluating
only the remaining three model cases.

A separate six-case central/yaw comparison extended identical-input tracking to
60 frames: central open/intermediate/near estimates were87.087/44.765/32.919 mm;
negative-yaw estimates were76.495/22.743/92.387 mm. Waiting longer therefore did
not resolve the older near-pose error. These use the original full-body inputs,
not the new hand-only fixture. No sensor, input device, threshold change or
anatomical acceptance was involved.

The reusable [visible-work tools](../tools/synthetic/README.md#visible-experiments)
display actual render/inference stages and retain completed/error state. A static
or replayed result must not be described as ongoing processing. Human observation
is part of experimental validation: inspect the actual image, visibility and
occlusion before interpreting metric landmarks or missed detections.


## Human review rejects the old closure direction —2026-10-15

Nicolás observed dorsal index bending in the first articulated closure sequence.
It used the same frozen solver recipe as the older intermediate/near sweeps.
Those records retain exact geometric/model observations, but their poses must
not be treated as valid natural-pinch references. Skin clearance alone did not
validate the direction of flexion. The corrected forward-flex recipe, signed
negative control and separate skin/model/contact measurements are documented in
[the evolution report](research/controller-evolution-2026-10.md#articulated-skin-reference-and-human-correction).
