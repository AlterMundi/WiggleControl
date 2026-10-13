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
