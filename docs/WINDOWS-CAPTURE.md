# Windows capture integration

Implementation route for [#20](https://github.com/AlterMundi/WiggleControl/issues/20),
not an installed/qualified camera backend. The configured provider CLI is portable
through [PR27](https://github.com/AlterMundi/WiggleControl/pull/27); camera transport,
worker launch and the complete Windows calibration/mouse app are still pending.

## Reuse the existing worker transport

Use a small Windows Media Foundation source-reader adapter to supply native MJPEG
to the existing Rust latest-frame/lease transport and pinned Tasks GPU worker.
This is a reuse choice based on the existing implementation, not measured Windows
performance. The Linux path already publishes JPEG bytes with source timestamps,
checks actual dimensions and runs one in-flight detector request. Keep its
landmark interpretation, identity, gestures, calibration and input publisher.
Do not add JavaScript gesture decisions or a second detector.

At explicit Start, activate the selected device, inspect native media types and
select native1920x1080 MJPEG for the offered C920e. Preserve requested versus actual
format/rate in status and retained evidence. Reject an unavailable selection or
changed format; do not quietly use a square crop, substitute an unrelated device,
re-encode full frames or enable MF color conversion. Reuse camera-free synthetic
warmup before capture. Hardware/driver support for that mode remains a Legion
check. Enumeration should use activation metadata without starting capture;
format probing that activates a device belongs after the manual Start boundary.

Media Foundation's source reader can expose native compressed data without owning
a display/render pipeline. Inspect the exact native type and keep conversion
explicit; an encoder/decoder is not necessary merely to pass JPEG to the already
working browser decoder. Reference: [Source Reader](https://learn.microsoft.com/en-us/windows/win32/medfound/source-reader)
and [native capture format selection](https://learn.microsoft.com/en-us/windows/win32/medfound/how-to-set-the-video-capture-format).

## Preserve the acquisition clock

Use `MFSampleExtension_DeviceTimestamp`: Microsoft specifies the capture driver's
QPC-epoch timestamp in100ns units. Convert to the same-host microseconds used by
`platform/windows::monotonic_us`, then retain actual source order/age through the
existing frame lease and output guard. Validate missing, future, regressing and
stale values. `IMFSample::GetSampleTime` is relative to streaming start and cannot
be used as an absolute QPC timestamp. Do not substitute receive time or browser
inference completion to make freshness pass. Actual attribute availability on the
offered device/driver still requires verification.

Reference: [device timestamp contract](https://learn.microsoft.com/en-us/windows/win32/medfound/mfsampleextension-devicetimestamp).
Browser video callbacks alone do not establish this driver/QPC contract, so moving
capture into `getUserMedia` is not selected merely to avoid native integration.

## Bound driver and process lifecycle

A blocking driver read must not prevent Stop, four-corner retry or independent
button cleanup. Isolate native capture in an owned child and keep bounded frame
messages/latest-frame consumption. The parent must observe child exit before
claiming camera closure, retain an unconfirmed state on failure, and never restart
capture automatically. Reuse the existing independent input writer/supervision;
source silence and source age remain separate from process liveness.

Use maintained Microsoft Rust COM bindings rather than writing COM vtables.
`windows`0.58.0 is the candidate pin: its upstream manifest declares Rust1.70 and
MIT OR Apache-2.0, compatible with the pinned1.85.1 project compiler. Existing
`windows-sys`0.59 primitives need not be replaced. Inspect the exact methods and
resolve/checksum the dependency before implementation; no new dependency or native
camera source is installed by this design note. Reference: [upstream manifest](https://github.com/microsoft/windows-rs/blob/0.58.0/crates/libs/windows/Cargo.toml).

Fake-device/packet tests can verify selection, payload bounds, timestamps and
lifecycle faults. They cannot establish camera busy/privacy/USB behavior, native
Full HD pixels, GPU use, background behavior or physical hand/calibration comfort.
Keep those receiving checks, along with fullscreen visible-camera calibration,
explicit Save/rollback and mouse acceptance, in #20–#22. Normal use/calibration
must not record video, and opening the app never starts camera or input.
