# Developing WiggleControl

Read docs/CONTEXT.md, docs/DEVELOPMENT.md and docs/research/state-of-the-art.md
before selecting dependencies or architecture. Ordinary checkout: ~/Projects/WiggleControl.
Inspect supported APIs/configuration, model preprocessing and relevant tests before
writing equivalents. Reuse Rust evdev/uinput and qualified inference libraries.

Rust is preferred; justify native FFI or other language components using concrete
compatibility/performance evidence. Pin source/model/runtime/compiler versions.
Do not silently upgrade the existing panel's production toolchain or dependencies.

The product is a system input device for existing applications. Browser tracking
may be a baseline/comparison source, not the final product boundary. Keep input
observation, gesture meaning and OS output separate, with replayable fixtures.

Platform modularity — human request, 2026-10-22: the Windows11x64 port follows
docs/WINDOWS-PREVIEW.md and milestone2. Keep one OS-independent Rust
observation/gesture/calibration/filter/detail/scroll/delivery-safety core;
platform adapters own camera/input/display, clocks/storage primitives and
process/launch behavior. Extract existing logic and preserve Linux contracts;
do not implement another Windows gesture engine or silently resume the cleared
Linux native goal. Windows support requires actual receiving qualification.
Current source boundaries and replay/CI scope: docs/PLATFORM-LAYER.md. Reuse
landmarks/tracking/output and the shared calibration/controller modules; Linux
input paths are compatibility reexports of platform/linux. `session_config` owns
shared configuration/controller admission; `provider-runtime` exposes camera-free
configured raw-provider replay on both platforms. `native-vision` still includes
Linux capture. Do not enable V4L/Unix session code to make Windows use the provider.
`windows-camera` is the optional native-MJPEG/QPC capture adapter and supervised
latest-frame receiver, with camera-free process fixtures. `browser-provider`
connects capture/writer to the same worker/calibration service on both OSes; reuse
that engine and session_io. Windows requires explicit matching monitor selection
and confirmed child cleanup, keeps recording off and omits per-frame stdout.
Current APIs: docs/WINDOWS-SESSION.md. Ordinary-user launch/package and actual
Legion camera/GPU qualification remain open.

Use real monotonic timestamps. Bound messages/queues, validate observations and
release held buttons/contacts on loss, stale data, stop or process failure.
Do not invent a click on reacquisition. Keep physical inputs usable.

Keep new device/camera tests finite and human-directed. Research or replay tests
must not activate a camera. No automatic startup capture, Matrix attention, model
wakeups or autonomous peer conversations. Coordinate shared desktop use before
live input injection; preserve current SSH, sudo and desktop services.

Private host routes, keys, credentials, device serials and actual recordings stay
outside Git. Fictional fixtures and source-linked aggregate results belong here.
Use the existing Shared Resources private overlay for receiving bindings.

Original source is MIT; preserve separate upstream/model licenses when reusing
code. Barehands/ProyectoRPi provenance is AGPL, not implicitly relicensed by Rust
translation. Inspect actual files before choosing derivation versus an independent
implementation of the event contract.

English code/docs, Spanish conversation with Nicolás. Validate only relevant
behavior: cargo fmt, meaningful unit/replay tests and clippy once Rust exists.
Do not claim source tests prove real libinput recognition, gesture accuracy or
latency. Record source, runtime/device acceptance and human feedback separately.

TV-facing interfaces prioritize the actual integrator display: 1920x1080
fullscreen, readable from the seated viewing distance, all primary controls and
status visible without whole-page scrolling, and space for the top-right compact
status panel. Preserve the chosen Star Trek/LCARS style. Check real viewport
bounds including dynamically inserted controls before asking for a physical trial.
Scrollable content inside the explicit scroll exercise is intentional.

For each live human gesture trial, describe one finite test and wait for the
human's explicit ready acknowledgment before acquisition. Presence in a prior
message and a delivered instruction do not establish that the human read it or
is attending the new test. Separate preparation/build/layout work from capture.

Finish receiving preparation and relevant checks before asking for the human's
ready acknowledgment. The human may be attending another session; do not ask
for readiness while still building or leave them waiting after they reply.
Timed trials must show a conspicuous running indicator and producer-tied
countdown, with terminal state and retained result on completion.

Nicolás superseded automatic trial video retention on 2026-10-21 after an
unbounded recording exhausted Legion storage. Ordinary use and calibration must
not record video by default. Capture dataset samples only on explicit request:
finite maximum 30 seconds, 256 MiB per take, 2 GiB per recording directory and
2 GiB filesystem free-space reserve. Directory configuration alone is not opt-in.
Preserve only retained footage linked to concrete code tests when doing authorized
cleanup; keep private recordings outside Git. Do not restart stopped acquisition.

Nicolás explicitly selected manual continuous usage on2026-10-14: when the human
requests input until they say to exit, use the named until-stop session/renderer
commands. This overrides finite-trial timing for that requested session only;
do not simulate it by silently chaining finite captures or add automatic restart.
Keep observed activity, recording and task-local Stop available.
