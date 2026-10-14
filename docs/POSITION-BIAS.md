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
