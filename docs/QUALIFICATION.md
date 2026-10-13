# Development and receiving evidence

## 2026-10-13 — Gesture replay and Linux registration

Published gesture engine at b55d645, Linux adapter at f35eaa9.
VPS Rust 1.95.0 and receiving Pi 5 ARM64 Rust 1.85.1 each passed 14 tests:
pinch/drag, jitter/confirmation, identity replacement, neutral reacquisition,
stale/stop/loss/ineligible/malformed/out-of-order input release, CLI size and EOF
handling, Linux event codes and fractional movement. Local clippy passed;
GitHub Actions run 38387111298 passed fmt, tests and clippy on Rust 1.85.1.

The earlier run 38386242490 failed only an older Clippy `format_collect` lint in
a fixture-building test; its actual job output was inspected and that expression
was corrected. Do not attribute this to camera or ARM incompatibility.

Receiving temporary ACL allowed only the existing operator to create uinput
devices; original retained locally for reversal. A five-second no-event check
observed udev ID_INPUT_MOUSE=1 and automatic removal after expiry. A separate
finite check observed libinput's `Capabilities: pointer` and adaptive/flat/custom
acceleration availability. No camera or microphone was used. No application
movement/click was emitted. The existing panel remained running.

Remaining for milestone 2: actual native Wayland/XWayland application delivery,
finite watchdog/button-release/SIGKILL device removal tests, calibrated workspace
and scroll mapping. Device registration is not user acceptance.

## Native perception candidates

The separately enabled model-probe feature evaluates tract 0.21.15 with pinned
OpenCV Zoo palm/hand ONNX artifacts listed in models/manifest.json. These are
Apache-2.0 candidates, not MIT assets or a selected production backend. Licenses
are retained alongside the manifest; downloaded model binaries are ignored.
The probe uses a synthetic zero crop, opens no sensor, and separates model-only
load/timing from the eventual full pipeline. Compatibility/performance results
will be recorded after execution, rather than inferred from successful download.

Default camera-free runtime does not link the optional inference engine.
The dependency resolver honors the declared Rust 1.85 floor; an initial native
probe resolve selected kstring 2.0.5 (Rust 1.96) and was corrected to a compatible
locked dependency set without changing either host's compiler.


Offline candidate probe on VPS Rust 1.95.0: hand landmark model produced finite
outputs with shapes [1,63], [1,1], [1,1], [1,63] from the synthetic zero crop.
Debug-profile model-only timing was about 2591 ms for one run and is not a Pi
performance estimate. The palm model failed at Resize__235: expected output
[1,256,12,12], actual [1,256,6,6], both with and without graph optimization. Thus
this pinned tract candidate does not yet qualify the full two-model pipeline.
Preserve this concrete incompatibility and evaluate a different supported
backend/version before choosing production inference. No camera used.
