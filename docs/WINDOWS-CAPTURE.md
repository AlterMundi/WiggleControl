# Windows capture adapter

Source implementation for [#20](https://github.com/AlterMundi/WiggleControl/issues/20).
`platform/windows/camera` supplies native MJPEG, driver/QPC timestamps and a bounded
latest-frame receiver through an independently supervised child. This is a library
and internal commands, not the finished Windows calibration/mouse application.
No Legion installation, actual camera mode or hardware acceptance is claimed.

## Native capture boundary

The optional `windows-camera` feature uses Microsoft's locked `windows`0.58.0 COM
bindings, plus the already-pinned image header reader. Existing `windows-sys`0.59
primitives remain in use. The new dependencies/checksums are in Cargo.lock; no
handwritten COM vtables or new hand model/gesture implementation is introduced.
The upstream binding declares Rust1.70 and MIT OR Apache-2.0, compatible with the
project's pinned1.85.1 compiler. [Upstream manifest](https://github.com/microsoft/windows-rs/blob/0.58.0/crates/libs/windows/Cargo.toml).

`devices()` enumerates activation metadata without `ActivateObject`/`ReadSample`.
On explicit Start, `Capture::spawn` opens its child with the selected camera ID,
resolution, rational frame rate, source-freshness budget and run ID. The child
activates only that device, enumerates native types, disables converters and selects
MJPEG with exactly the requested dimensions/rate. Equivalent rational rates match;
29.97 is not silently treated as30. The offered C920e preview targets1920x1080/30fps;
actual availability is still a hardware/driver check. No square crop, alternate
device, full-frame re-encode or MF color conversion is used as an implicit fallback.
Actual post-selection media type and every JPEG header must match the selection.

The source reader passes compressed bytes to the existing JPEG/worker architecture
without owning a display pipeline. The next app integration must reuse the Rust
lease transport, pinned Tasks GPU worker, identity/gesture core and calibration,
with camera-free synthetic warmup before capture. Windows renderer/RTX operation
and whole-pipeline performance remain unmeasured. References:
[Source Reader](https://learn.microsoft.com/en-us/windows/win32/medfound/source-reader),
[native capture format](https://learn.microsoft.com/en-us/windows/win32/medfound/how-to-set-the-video-capture-format).

## Driver clock and bounded transport

`MFSampleExtension_DeviceTimestamp` is the capture driver's QPC-epoch timestamp in
100ns units. Convert it to the microseconds used by the existing Windows clock;
never substitute arrival time. `GetSampleTime` is stream-relative and is not used
as an absolute QPC value. Missing/zero timestamps, future values, duplicates or
regressions fail explicitly. Source age is checked on receipt and again when a
consumer takes the latest frame; stale frames do not refresh camera liveness.
Actual attribute availability still needs verification on the offered driver.
[Official timestamp contract](https://learn.microsoft.com/en-us/windows/win32/medfound/mfsampleextension-devicetimestamp).

A pipe message has at most8KiB of JSON header and2MiB of JPEG bytes. Bound the
native sample before MF allocates a contiguous copy, then verify the locked buffer,
length and image dimensions. Retain one latest frame; replacing or expiring it
increments dropped count. `received` counts structurally valid payload receipts;
`fresh_frames` counts accepted fresh ingress, not hand detections or application
input. No video/metadata file is created by this capture adapter.

`camera_frame::Frame` is shared with the Linux capture session through a compatibility
alias. Gesture/calibration policy does not depend on a native buffer or OS API.
The independent input writer still owns buttons and its own watchdog separately.

## Stop and process failure

The child runs without a console window. EOF requests normal capture/source shutdown;
the parent independently observes process exit, protocol errors, startup/fresh-frame
silence and close grace. It can terminate only its owned child, using the same
bounded helper as the Windows input supervisor. A child EOF watchdog also ends its
own process if its owner dies while a driver call is blocked. Its forced exit24
uses `TerminateProcess` on the current capture child, avoiding DLL-detach locks;
the fixture verifies that exact exit code through its retained process handle.
Normal shutdown remains graceful. No restart follows. References:
[ExitProcess lock behavior](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-exitprocess),
[TerminateProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess).

`Status::camera_opened()` is false only after observed child exit. Opening or
unconfirmed failure is unknown; a close message alone does not prove process exit.
Retain negotiated format, counters and original diagnostics after shutdown. A
failed termination remains unconfirmed; do not use it as permission to open another
capture. This status describes this session, not every camera consumer on Windows.

Internal commands are `windows-camera-list` (metadata), `windows-camera-stream
SPEC_JSON` (explicit native capture), and device-free fixture commands. They are
not the ordinary-user preview launcher; the internal stream function belongs
only in its dedicated child, never the UI/input owner. Build/test the adapter on Windows with
`cargo test --locked --features windows-camera --lib --test windows_capture`.
CI fixtures generate JPEGs and never call MF camera enumeration/acquisition or
SendInput. They test latest-frame replacement, full-aspect payloads, clock/format/
message bounds, normal EOF, startup failure, crash, partial/stalled streams and
hard parent exit while its camera child is blocked. The process-handle assertion
is evidence of process exit, not physical camera/USB qualification.

## Remaining integration and acceptance

Connect this adapter to the common configured provider/worker, consume status in
manual controls, reuse visible-camera full-screen diagonal/four-corner calibration
and reliable retry/cancel/Save/rollback, then package ordinary-user launch (#21).
Camera busy/privacy/disconnection, actual Full HD pixels/cadence, GPU/background
behavior and physical mouse/comfort remain #20–#22 receiving checks. Opening the
app must never start capture/input; normal use/calibration must not record video.

Source05e14f6 passes all five jobs in
[CI39728792920](https://github.com/AlterMundi/WiggleControl/actions/runs/39728792920),
including native-Windows adapter compilation and device-free process tests,
plus existing Linux native/installation and ARM64/x86 package checks. Follow-up
source/check outcomes are retained with the owning PR; this does not establish
actual camera or Windows application readiness.
