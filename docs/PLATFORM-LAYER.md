# Shared controller and platform adapters

The Windows 11 x64 port follows [WINDOWS-PREVIEW.md](WINDOWS-PREVIEW.md), milestone2
and #18–#22. This describes the first source extraction, not Windows camera/input
support or receiving acceptance. Linux's public module paths and CLI contracts
are preserved through reexports and narrow compatibility shims.

## Common implementation

| Module | Responsibility |
| --- | --- |
| `acquisition` | Independent optional recorder/input setup, including the #17 video-off input-startup correction |
| `landmarks` | Raw geometry, shared pinch metric/posture interpretation, selected-hand evidence and the controller bridge, including the #16 detail-drag correction |
| `tracking` | Provider frame validation, source age/order, measured presence and bounded spatial hand correspondence; detector array order is never an owner ID |
| `controller`, `pointer_buttons`, `detail`, `escape`, `scroll`, `cursor`, `filter` | One gesture/motion implementation reused by both platforms |
| `contacts`, `touch_gestures` | Logical contact data and mode arbitration without an OS event code |
| `calibration`, `calibration_runtime`, `guided_calibration`, `provider_calibration`, `profile_store` | Sampling, full four-corner review, manual Save/retry/cancel, previous-profile adoption and rollback policy |
| `output` | Logical packets, absolute mapping/cursor publishing, source/sequence validation, bounded Hold, derived-motion restrictions and independent writer watchdog decisions |

`output::State<M,T>` depends on small `MouseBackend`/`TouchBackend` event conversion
interfaces. It owns the existing delivery safety state machine; adapters supply
native event representation and per-button/contact conversion. The runtime must
emit the returned batches in order, call `tick` independently of inference, and
emit `release` during cleanup including after rejected packets or native API errors.
No shared-core function claims that an event was received by an application.

The native/provider/sequence paths delegate through the same `landmarks` bridge.
`vision` retains model/image inference and reexports the extracted geometry types;
`session` retains Linux capture/process orchestration and delegates interpretation.
`provider` keeps native configured/replay integration and reexports `tracking`.
The current `native-vision` feature is still the Linux acquisition/backend path;
Windows core tests do not enable V4L or ORT to exercise gestures/calibration.

## Platform implementation

`platform/linux` contains mouse, touch, fixed-key Escape and independent device
writer adapters over evdev/uinput. Existing `input`, `touch`, `keyboard` and
`delivery` imports remain aliases to these implementations. Event conversion,
device names/classification and writer commands are not rewritten by this move.

Platform clock/profile-file mechanics are selected in `platform`:

- Linux preserves the existing `CLOCK_MONOTONIC` epoch, Unix UID/mode/no-follow
  checks, same-directory rename and directory fsync.
- Windows uses QPC for a same-host producer/writer epoch and Windows file
  replacement/reparse-point primitives. The shared store flushes and closes the
  temporary file before replacement; no Unix ownership value or permission mode
  is fabricated. The eventual Windows app must put receiving state in its user's
  local application-data directory, whose ACL new files inherit.

Windows primitives use the already lock-resolved `windows-sys`0.59.0 bindings.
[QPC](https://learn.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter),
[CreateFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
and [MoveFileEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)
are the official platform references. Linux clock/profile semantics stay unchanged.

The initial Windows event/display library is described in [WINDOWS-INPUT.md](WINDOWS-INPUT.md);
its native sender is replaced by a fake in CI. Writer process-death cleanup and
receiving acceptance remain open.

Camera negotiation/acquisition, remaining runtime input/display integration, actual process lifecycle and
launch/distribution must remain platform responsibilities as #19–#21 are implemented.
A platform trait is useful when consumed by actual code; it is not a requirement
to wrap every standard-library call or build a general plugin framework.

## Verification boundary

The native Windows core CI job uses pinned Rust1.85.1 to run common library,
controller and device-free output watchdog/Hold/Escape fixtures. It checks default
binaries without starting acquisition. The Windows CI runner is build/test
infrastructure, not the offered Legion or proof of Windows11 camera/GPU operation.
Linux's existing native/replay/Clippy/package checks remain required.

The established raw detail-drag and identity regressions move with their
implementations into the portable suite, rather than being rewritten as a Windows
gesture algorithm. Device-free sinks verify delivery policy without evdev or
SendInput. Actual Windows native output, frame acquisition, display/DPI mapping,
GPU use and human acceptance remain #19–#22 work. No machine is deployed, calibrated
or receiving-qualified merely by this extraction or a passing CI result.

## Source checkpoint — 2026-10-22

Source `481a622` passes all five jobs in
[CI39719378523](https://github.com/AlterMundi/WiggleControl/actions/runs/39719378523):
Windows core, Linux replay/native/Clippy/installation fixture, ARM64 and x86 packages.
A source/hash-verified `93ae0d4` baseline and `481a622` candidate produce identical
full controller outputs over10,221 retained observations:1,726 held and512 detail
observations, with no camera or system input. Profiling timing values are excluded
from the semantic comparison. These are prediction/regression comparisons, not
labelled intent, new physical acceptance or a Windows mouse release.

The follow-up extraction also puts the #17 optional-sink factory and its exact
four-combination/error-propagation regressions in the common library, so Windows
uses the same input-independent-of-recording setup rule. Physical camera/input
adapters, user-facing packaging and Legion acceptance remain pending.
