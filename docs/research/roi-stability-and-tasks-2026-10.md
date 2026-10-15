# Stationary crop feedback and complementary Tasks estimates

Investigation by CompAII,2026-10-15. Continues
[the onset-permission experiment](pinch-onset-veto-2026-10.md) and
[#7](https://github.com/AlterMundi/WiggleControl/issues/7). These are camera/device-
free comparisons, not a new selected runtime or physical acceptance.

Subsequent [implemented fusion/native-crop qualification](native-pinch-fusion-2026-10.md)
executes the previously analytic union through Rust and tests a default-off
shared-model dual-crop producer. The analytic-only statements below describe
this earlier checkpoint; they do not imply fusion remains unimplemented.

## A stationary image is not a stationary tracking crop

The new MCP-flexed/distal-straight pinch at yaw−35° has nine maximum-closure
frames with identical decoded640×480 RGB pixels. Their PNG file bytes differ;
file hashes alone would incorrectly classify this appearance as changing.
FFmpeg RGB24 decoding verifies the identical pixels. Selected distal skin
clearance stays1.502mm. Native recursive tracking nevertheless varies the
estimated thumb/index gap35.87–55.00mm and its crop across that hold.

A separate cold→30-tracked inference on one unchanged image produces gaps
28.93–54.79mm. The existing `position-sweep` then checks nine integer translations
with corresponding fixed crops: maximum model-input tensor difference is exactly
zero and every fixed-crop gap is41.132662819542034mm. Normal tracking frame10
varies42.717–53.345mm; the existing proximal-recipe probe varies21.078–45.156mm.
The shifted-image detector also sees changed background boundaries. These
controls support crop/history sensitivity, not a missing screen-position factor,
a universal proximal-crop fix or calibrated physical fingertip distance.

## Rejected fresh-palm experiment

A temporary native branch detects palms every frame and refreshes a track crop
only for mutual unique corresponding-palm matches below0.25 normalized RMS.
IDs are retained; ambiguous matching does not choose by proposal order.
Three complete image/controller sequences are actually run:

| Scene | Native baseline | Fresh-palm experiment |
| --- | --- | --- |
| Adverse yaw: near-hold contacts |1/9 |0/9 |
| Separated V: contact frames |26 |27 |
| Central distal-straight: near contacts |9/9 |9/9 |

The adverse-yaw sequence records31 refreshed attempts overall but none during
its nine-frame near hold. A skipped palm proposal has cost0.2798 against a
current accepted hand; correspondence remains an actual limitation. No threshold
is enlarged afterward. This is a rejected experiment, not an improved controller.
The prototype and its runtime option are removed from current source; the
VPS development binary is restored. A
[base-pinned patch](../../tools/synthetic/experiments/README.md) retains reproduction
and its passing mutual-uniqueness/proposal-order test. Nothing was deployed on Pi.

## Pinned Tasks comparison and conditional meaning

The already cached MediaPipe1.1.1 CPU VIDEO API is reused with its pinned
`hand_landmarker.task`, SHA256
`fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1`.
Its bundle contains hand-detector and landmark TFLite assets of2,339,878 and
5,478,949bytes. The current native ONNX landmark artifact is4,099,621bytes.
This compares different released bundles, preprocessing and tracking; it does
not isolate trained weights or prove that file size explains accuracy.
Dependency pins and provider API are the existing
[Tasks comparison](controller-evolution-2026-10.md#executed-current-tasks-comparison).

Three pilot scenes are followed by registered remaining configurations: eleven
positive trajectories and a second separated V, with unchanged0.38 onset ratio.
Together these are thirteen positive41-frame trajectories and two negative
61-frame trajectories:655 images. All655 receive one Tasks hand. Remaining
configuration image hashes are registered before their Tasks execution and
verified afterward; geometry is matched against retained native inputs.
These are the same rig and synthetic configurations, not independent anatomy,
human intent or an unseen real-camera test set.

`compare_tasks_metric.py` runs the actual Rust controller while retaining native
confidence, engagement, eligibility, ID, cursor and source time. It substitutes
only Tasks' estimated world gap, optionally deriving onset permission from
Tasks' **own** projected tip-gap/palm ratio. Using native pixel cues with a
different model's world estimate would mix the interpretations. Missing Tasks
hands remove the observation; multiple hands or mismatched source frames are
rejected. Provider array order never becomes identity. Baseline contact frames
must reproduce exactly; input hashes, full geometry, Stop and EOF are checked.
This is conditional single-hand meaning, not a complete Tasks backend.

| Retained20Hz observations, interpreted at | Native near contacts | Tasks gap only | Tasks gap + own onset cue |
| --- | --- | --- | --- |
|20Hz |108/117 |107/117 |107/117 |
|10Hz phase0 |52/65 |55/65 |55/65 |
|10Hz phase1 |47/52 |47/52 |47/52 |
| Original V contacts20Hz /10Hz phases0 /1 |26 /12 /13 |27 /13 /13 |0 /0 /0 |

The second negative and all completely open holds remain contact-free. The
10Hz rows here subsample retained20Hz **observations**; they do not rerun the
perception trackers at10Hz. Actual source intervals are retained by the tool.

Tasks repairs the adverse yaw1/9→9/9 at20Hz but loses a different negative-pitch
distal-straight hold9/9→0/9. That hold's Tasks gap varies42.94–47.08mm and pixel
ratio0.406–0.457, while the selected skin stays1.502mm apart. A prior forward-flex
positive-pitch control also loses one near frame. Tasks is not adopted as an
unconditional metric replacement. Lower isolated gap error in a pilot scene
does not establish a better complete controller.

Observed Tasks API/decode median is71.69ms, p95126.43ms across655 calls on VPS.
These sequential source runs differ from the retained native run's load and
classification work. They are not a fair total-pipeline speed ranking, achieved
20FPS, Pi timing or proof that two models fit the receiving latency budget.

## Counterfactual combination, without oracle selection

Both metric hypotheses run separate actual Rust confirmation/release decisions,
each with its own fixed0.38 onset permission and unchanged45/65mm/40ms settings.
A Boolean OR of their contact indicators gives117/117 near frames at20Hz,
61/65 at resampled10Hz phase0 and52/52 at phase1, with zero contacts in either
negative or completely open holds. The rule does not use reference labels to
select a model per frame; labels are used only for scoring afterward.

This is an analytic development result. It is **not** implemented contact-ID
fusion, shared lifecycle, asynchronous scheduling, ownership, macro integration
or OS delivery. The remaining10Hz misses and per-case onset/release traces are
retained. An OR can preserve a false held state and amplify a new false positive;
the current onset veto does not repair conflicting release estimates. New
conflict/negative configurations and real input remain necessary.

## Actual10Hz tracking follow-up

Fresh10Hz source sequences are prepared from the adverse yaw, negative pitch
and separated V, at both source-clock phases. Both perception stacks restart
for each sequence; image pixels and geometry are retained, with original source
indices. This addresses the tracking-history limitation above:

| Fresh10Hz stack run | Native near / negative contacts | Tasks + own onset | Counterfactual union |
| --- | --- | --- | --- |
| Adverse yaw phase0 |0/5 near |4/5 near |4/5 near |
| Adverse yaw phase1 |0/4 near |4/4 near |4/4 near |
| Negative pitch phase0 |5/5 near |0/5 near |5/5 near |
| Negative pitch phase1 |4/4 near |0/4 near |4/4 near |
| Separated V phase0 |5 contact frames |0 |0 |
| Separated V phase1 |12 contact frames |0 |0 |

The complementary errors survive fresh tracker execution. The union covers
17/18 near frames across these two positives/both phases, with no negative
contacts. Native V counts5/12 differ from the12/13 obtained by decimating20Hz
observations; the metric-only Tasks V counts are13/3. Tracker cadence/history
therefore materially changes the raw result. It is not adequate to qualify
receiving10FPS by subsampling observations alone.

All twelve fresh native/Tasks runs complete,286 image-stack inputs in a finite
139.045s wall/31.151s CPU group, approximately207MiB peak and zero task swap under
200%CPU/2GiB limits. These are sequential VPS workers, not a resident dual-model
Pi runtime or measured concurrent memory/latency. Actual inference was visible;
the final comparison remains explicitly saved rather than pretending work runs.

## Reproduction and next implementation

Use the pinned existing synthetic inputs and Tasks environment. Run
`mediapipe_sequence.py SEQUENCE TASK --expected-sha256 SHA`, or the existing
visible-motion observer with `--mediapipe-model TASK --model-sha256 SHA`.
Then use `compare_tasks_metric.py NATIVE_RESULTS TASKS_RESULTS --images IMAGES
--binary BINARY --settings SETTINGS --onset-ratio 0.38 --output NEW_REPORT`.
It writes component comparisons and separately labelled counterfactual unions;
no model inference or system input occurs in this scoring command.

Stationarity can be checked by decoding each near PNG with
`ffmpeg -v error -i IMAGE -f rawvideo -pix_fmt rgb24 -` and hashing the resulting
bytes. The existing `position-sweep IMAGE SESSION NEW_DIRECTORY` provides the
fixed-tensor/normal/proximal controls. A constant pixel hash alone does not
certify the rig's whole-hand anatomy or intended human gesture.

Next: compare efficient native crop hypotheses/the actual Tasks landmark model
before accepting a costly two-stack runtime. If evidence fusion is retained,
implement explicit same-hand/frame matching, independent confirmation, one
contact owner, lifecycle/Stop/loss/rearm and freshness handling; test new
conflicting negatives before adoption. Multi-hand association, macro recognition,
Pi resources and fresh acknowledged physical/application acceptance remain open.
Selected Pi binaries,45/65 settings, calibration, pending original trial and
silent power policy are preserved. All original goal requirements remain active.
