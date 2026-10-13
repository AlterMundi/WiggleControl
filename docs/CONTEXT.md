# Project context — 2026-10-13

Nicolás selected the name **WiggleControl**, executable `wigglecontrol`, after
checking naming collisions. He requests a lightweight Rust camera-driven input
device usable over existing software as a mouse and later a multitouch screen.
He explicitly requested research before implementation and then creation of
AlterMundi/WiggleControl with the plan persisted here.

Cuevanico is the receiving Pi 5 with C920e and labwc/Wayland, now at 1080p60. Its Rust
status panel is deployed independently. Source-backed discovery lives in Shared
Resources resource `host-cuevanico`, docs/integrators/cuevanico.md; exact local
routes, operator credentials and media stay in that receiving private overlay.

Ani/Oliva's ProyectoRPi/Barehands prototype has one documented successful
physical pinch/selection passage on this Pi. That is the comparison baseline,
not a universal input adapter or sustained performance benchmark. HarMoCAP has
useful smoothing, timing, validity/replay methods; body pose alone is not finger
recognition. Do not copy their private history or assume permission from a repo.

Research completed in daimon-integrators at commit `208a698` and carried to this repository
with provenance. Read-only receiving inspection found labwc 0.20.2/wlroots 0.20.2
and a root-only uinput node; live virtual device creation is not qualified.
Current inference library heads have compiler requirements newer than the
panel's Rust 1.85 baseline; choose compatible pins or justify a separate toolchain.

Milestone 1 now has a Rust replay/gesture engine, observation contract v1, bounded
JSONL CLI, fictional fixtures and 14 passing behavioral/CLI tests on VPS Rust 1.95.0 and receiving ARM64 Rust 1.85.1.
Pinch confirmation, hysteresis, loss/stale/stop release and neutral reacquisition
are tested. Filtering is a timestamp-based exponential baseline, not yet a
human-qualified smoothing choice. Scroll mapping remains pending. CI targets Rust
1.85.1; its first run passed compilation/tests but failed an older-Clippy format
collection lint; the test was corrected. Corrected CI run 38387111298 passed on Rust 1.85.1.
The evdev virtual mouse adapter now
creates an ID_INPUT_MOUSE=1 device on cuevanico and removes it after finite expiry.
The temporary receiving uinput ACL applies only to the existing operator; original
ACL retained for reversal, no persistent grant installed. No camera or input
injection occurred. Application delivery/watchdog/SIGKILL qualification is pending.
Multitouch is a later distinct contract, not a promise of zoom in every app.

Owning implementation tracker: [WiggleControl #1](https://github.com/AlterMundi/WiggleControl/issues/1).
Historical discussion/parent: [daimon-integrators #3](https://github.com/AlterMundi/daimon-integrators/issues/3). Keep owning issues and this
context aligned as implementation progresses; research is not implementation.


## Native session integration checkpoint

The selected experiment uses ort rc.10 / ONNX Runtime 1.22 CPU, not the failing
tract palm candidate. Both models run on Pi 5; native geometry matches one
positive upstream frame within median 0.45/max 2.50 pixels against OpenCV.
Perception/model confidence is not human gesture accuracy. Latest-frame 640x480
MJPEG, monotonic driver timestamps, finite stop and no recordings are implemented.

The installed relative virtual mouse reaches Chromium through actual Linux
input, including drag/scroll/watchdog/process-death release. Integrators owns
manual `input SECONDS` preparation/start/Stop; input differs from recording.
A private task-owned browser page is available for hand qualification; one real
camera-only session accepted no hands. Do not infer human absence from that.
Source/account/private bindings remain in Shared Resources. No models at boot.
Next: direct touch replay/typing, additional application acceptance and a
human-present gesture session with real accuracy/comfort measurements.
