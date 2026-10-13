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
JSONL CLI, fictional fixtures and 11 passing behavioral/CLI tests on VPS Rust 1.95.0.
Pinch confirmation, hysteresis, loss/stale/stop release and neutral reacquisition
are tested. Filtering is a timestamp-based exponential baseline, not yet a
human-qualified smoothing choice. Scroll mapping remains pending. CI targets Rust
1.85.1; receiving compiler adoption must be checked separately. No camera or input
injection occurred. Next: Linux mouse adapter and receiving classification.
Multitouch is a later distinct contract, not a promise of zoom in every app.

Owning implementation tracker: [WiggleControl #1](https://github.com/AlterMundi/WiggleControl/issues/1).
Historical discussion/parent: [daimon-integrators #3](https://github.com/AlterMundi/daimon-integrators/issues/3). Keep owning issues and this
context aligned as implementation progresses; research is not implementation.
