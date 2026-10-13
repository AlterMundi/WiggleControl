# Camera-free baseline comparison

`tools/browser_baseline.html` is an independent developer test, not a camera
tracker or production adapter. It uses public Tasks Vision 0.10.14, the version-1
float16 hand-landmarker task, two hands, VIDEO mode and GPU by default. No private
prototype code is incorporated. Runtime/model imports happen only on manual run.
See the [official web API guide](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker/web_js).

Serve only the selected page and permitted reference from a task-owned local
directory. The page expects `reference.png`; it forms the same image plus its
horizontal mirror as the native paired diagnostic. Do not serve a repository's
private overlay or other user files. SDK/model network loading remains explicit;
selected artifact-byte verification and successful browser qualification remain
pending. The prototype's full graphics/interaction layers are excluded.

The manual button runs 30 frames; `runBaseline` permits 1–60 and explicit GPU/CPU
selection, without silently changing delegate. Result state includes current
phase, initial and final landmarks, initialization time and subsequent inference
p50/p95. Stop/close the task-owned page after the finite trial. No device/camera
permission is requested and no browser events are injected into applications.

Native comparison:

```sh
wigglecontrol image-check-touch PERMITTED_IMAGE RECEIVING_CONFIG pair 30
```

This requires the native-vision build and reports per-frame inference time,
accepted hands, IDs and landmarks. Bounds are 1–60 repeated frames. Neither tool
measures capture-to-application latency, user accuracy or comfort. Native CPU
ONNX fp32 and browser GPU/TFLite float16 use different graph/runtime/thresholds;
the result cannot isolate Rust language speed. Compare coordinate correspondence
on the same pixels, record thermal/delegate context and preserve first/tracked
cost separately. QUALIFICATION.md records actual results and unfinished checks.
