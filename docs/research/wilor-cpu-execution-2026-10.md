# CPU execution cost and a lightweight screen-space comparison

CompAII,2026-10-15. Continues the [mesh comparison](mesh-reconstruction-2026-10.md),
the [pinch interpretation evidence](native-visual-macros-2026-10.md) and#7/#3.
No receiving runtime, live threshold or calibration is selected by this report.

## Measure before changing precision

The unchanged pinned WiLoR-mini provider has a1280-wide,32-block transformer,
already using PyTorch scaled-dot-product attention. Its original source/checkpoint
and Python3.10.20/PyTorch2.5.0+cpu environment remain pinned by the mesh report.
No detector or fresh camera is loaded. The same eight retained native-landmark
crops and independently known right-hand side are used for a paired CPU probe.

Optional `wilor_probe.py --profile` uses supported CPU forward hooks on backbone,
refinement and MANO. Profiling reproduces **all21 world points exactly** against
the prior eight-input float32 result. Median total forward time is4792.71ms;
median backbone share is97.09%. Refinement is approximately0.1–0.2s, while the
backbone consumes approximately4–5s. This is an observed bottleneck, not a claim
that Python overhead or mesh drawing explains the delay.

`--execution dynamic_int8` uses PyTorch2.5's actual
[`quantize_dynamic`](https://github.com/pytorch/pytorch/blob/v2.5.0/torch/ao/quantization/quantize.py)
API with `inplace=True`, the available x86 engine and qint8 Linear modules **only
inside backbone.blocks**. All128 eligible layers convert; the remaining backbone
heads/refinement/MANO stay float32. The complete original trained model is loaded
first; no random or replacement trained weights, additional download, environment
upgrade or serialized quantized checkpoint is introduced. This is a research
execution variant, not another provider model or an ARM deployment.

| Selected static input | Float32 gap | Dynamic-int8 gap |
| --- | --- | --- |
| Macro open |107.98mm |106.38mm |
| Macro false onset |104.81mm |100.59mm |
| Macro held |98.89mm |95.11mm |
| Macro opening |105.20mm |103.85mm |
| Near yaw−35° |19.22mm |20.73mm |
| Near pitch−25° |24.39mm |26.86mm |
| Near curled |31.97mm |27.68mm |
| Near PIP-dominant |30.18mm |28.76mm |

Both variants separate these four far and four near samples across45mm. Maximum
absolute gap change is4.29mm. Quantized median forward time is2893.12ms, roughly
1.66times faster here, still far outside the interactive budget. Float32/quantized
load+preparation take12.84/23.16s. Finite process wall times are72.935/66.066s.
Both bounded tasks finish successfully under200%CPU/5GiB/zero-task-swap limits.
Both cgroup peaks round to5GiB: quantization does not remove original-model load
cost. Process `ru_maxrss` is5,375,234,048/5,446,049,792bytes; these are distinct
measurements from cgroup accounting. VPS contention and quota remain in timing.

Decision: this preserves a useful static accuracy comparison, but does not make
WiLoR a practical CPU runtime. No full temporal quantized inference, Pi speed,
end-to-end detector, ownership, physical intent or human comfort is established.
An accelerated accurate model remains a separate architecture question.

## Screen-space hypothesis and the cancellation trap

The retained native model's thumb/index pixel gap divided by wrist→middle-MCP
pixel distance is distinct from the previously tested **world** gap/palm ratio.
In development controls, the separated macro's minimum screen ratio is0.4015;
the six near-hold maxima range0.1942–0.3748. This suggests a lightweight signal,
but the interval was inspected after these examples: it is not held-out evidence
or a calibrated physical metric. Foreshortening and predicted-point errors remain.

A counterfactual first kept actual3D-open observations eligible and otherwise
set native `pinch_eligible=false` above0.38. Actual Rust replay suppresses every
negative contact **and every positive contact** across20Hz/both10Hz phases.
Cancellation destroys armed state before the hand becomes close; later eligibility
is not a fresh observed open. This rejects that placement of the guard. It does
not justify weakening open-before-press/reacquisition rules.

The existing interpretation helper now supports `--screen-ratio CLOSE OPEN` to
compare one declared pixel ratio against the unchanged raw baseline, replacing
the exploratory world-ratio grid for that invocation. It encodes ratio×0.1 into
the existing offline replay distance field, with thresholds scaled likewise.
These values are explicitly **not meters**. Native eligibility/confidence/ID,
cursor, source time, baseline reproduction and Stop/EOF checks remain intact.
The actual Rust controller runs at20Hz and both10Hz source-clock phases.

At fixed0.38/0.40, development macro contacts26/12/13 become0/0/0, but positive
near-hold coverage53/54→52/54,26/30→24/30 and23/24→22/24. An extra onset delay is
not a free improvement; this is not selected as a replacement. New geometry
configurations were registered before their rendering/inference to test that
fixed interpretation without adjusting it to their outcomes. They retain the
same rig/recipe, so this is configuration diversity, not independent anatomy or
human intent.

## Fixed-parameter new configurations

Before creating their observations, the private registration fixed0.38/0.40,
40ms confirmation and two previously unprocessed configurations: a61-frame
separated-finger V at yaw−35°, and a41-frame forward-flex pinch translated12cm
to camera left. Existing renderer/asset/forward-recipe pins remain unchanged.
Both first pass geometry-only checks, then render and run the native controller
with the same session/macro settings. Full original/geometry marker matching is
exact (zero error); skin is fully in frame, no selected-patch intersection, and
every frame detects one hand. The V's minimum selected skin gap is77.236mm;
the positive minimum is2.000mm. These remain selected-surface bounds, not full
anatomical plausibility or independently labelled human action.

| Fixed new configuration | Native20Hz | Screen20Hz | Native10Hz phase0 /1 | Screen10Hz phase0 /1 |
| --- | --- | --- | --- | --- |
| Separated V: contact frames |0 |0 |0 /0 |0 /0 |
| Left pinch: near-hold coverage |9/9 |9/9 |5/5 /4/4 |4/5 /4/4 |

Fully open positive holds remain contact-free in both interpretations. The V
already works in the native baseline, so its success is a retained negative,
not a new correction. The side-position positive confirms a cadence-dependent
loss at unchanged parameters; no threshold was adjusted afterward. Replays
reproduce baseline contacts and release on Stop/EOF. This small new configuration
set does not promote the screen interpretation or close#7.

The finite geometry/render/native controller unit completes successfully in
299.832s,249.078s CPU and962.1MiB cgroup peak under200%CPU/2GiB/zero task swap.
The first launch failed before geometry because its transient unit lacked `uv`
on PATH; an explicit existing executable path corrected that verified error.
The original registration/failure remain, with the successful retry separate.

Actual rendering and processing were projected. The V observer reports no
projection errors; the left observer later records SSH transport exit255 with
no diagnostic stderr. Its model still completes, and results remain retained.
Do not infer a failed model, a particular network cause or restart inference
because that observer failed. A later projection uses retained data explicitly.
The recovered owned page is observed at1920×1080/document1080 with the original
near-pinch frame loaded, explicit saved/no-inference text, recorded native contact
state and the measured rejection summary. Old replay is disabled rather than
reusing the preceding sequence after an incomplete image transfer.
Both model profiling probes, the new finite rendering/controller task and their
observers end; no camera, microphone, system input or Pi inference service is
started. The selected profile/runtime and pending human trial remain preserved.

The generic finite `project_mesh_probe.py` now follows actual producer snapshots
and incrementally consumes complete JSONL rows. A partial append is retained until
its newline and cannot be silently skipped or treated as an entire prediction.
Regression coverage reproduces that specific failure class without loading a
model. The projector starts no inference, sends only a small completion summary
and ends only its own observation on timeout; caller inspection of the actual
model task remains necessary. It replaces private per-experiment observer copies.
