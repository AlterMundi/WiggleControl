# Shared browser-provider session on Windows

Milestone 2 source integration (#19–#22). This is a developer-facing service,
not yet a portable user application or physically qualified Windows release.
No Legion camera, input or GPU has been exercised by this source change.

Build with `--features browser-provider`. The existing provider_fixture engine
runs on both systems: common worker leases, identity, calibration, gestures,
source freshness, output publisher and independent feedback views. session_io
connects the existing Linux writer or supervised Windows writer; the native Linux
session also reuses its packet construction. platform/linux/camera retains the
MJPEG/MMAP producer; platform/windows/provider_camera forwards supervised JPEGs
through the same latest-frame slot.

Opening the service starts no camera or input. Manual Start prepares the worker;
readiness starts the selected camera and, for mouse use, the independent writer.
Windows requires explicit monitor selection matching calibrated dimensions.
Stop/error checks confirmed camera child exit and neutral output separately.
Joining a camera-management thread is insufficient proof of child exit: uncertain
closure is unknown, preventing calibration Save. Windows mouse/calibration rejects
recording and emits lifecycle/terminal diagnostics without per-frame raw stdout.

Existing full-screen camera-visible diagonal and all-four-corner calibration,
progress/countdowns, Save/Retry/Cancel and profile rollback are served unchanged.
Hiding feedback changes rendering demand, not inference.

## Developer commands

Enumeration reads metadata, acquiring no camera or input and changing no layout.
Choose actual camera and monitor identifiers from the lists.

```powershell
.\wigglecontrol.exe windows-camera-list
.\wigglecontrol.exe windows-display-list
.\wigglecontrol.exe windows-provider-preflight session.json settings.json assets warmup.jpg 8765 'MONITOR-ID' 0 calibrate profile.json
.\wigglecontrol.exe windows-provider-session session.json settings.json assets warmup.jpg 8765 'MONITOR-ID' 0 calibrate profile.json
.\wigglecontrol.exe windows-provider-session session.json settings.json assets warmup.jpg 8765 'MONITOR-ID' 0 mouse
```

Put the selected camera symbolic link in session config, with camera_resolution
[1920,1080]; native MJPEG/30 fps is verified without square cropping or silent
camera fallback. pointer_output must match the physical selected monitor. Mouse
use consumes its selected saved calibration through existing settings. Zero
seconds is until Stop; finite mouse trials allow 1–90 seconds. Calibration is
camera-only, bounded to 150 seconds per attempt.

Open http://127.0.0.1:8765/provider/worker for inference and /provider/calibration
or /provider/control for controls. Managed browser launch and ordinary-user
configuration/selection UI are still pending. Preflight validates configuration,
output selection and pinned assets without a listener, camera or input; it does
not qualify actual GPU or camera availability.

## Portable primitives and evidence

Pinned RustCrypto sha2 0.10.9 replaces external sha256sum, retaining the manifest
allowlist and hashes; see its [incremental API](https://docs.rs/sha2/0.10.9/sha2/).
Recording index/read/budget policy is shared. Linux retains directory flock,
FD-based free-space checks and private modes. Windows uses an unshared store lock
file, user-local inherited ACLs and caller-available bytes from
[GetDiskFreeSpaceExW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdiskfreespaceexw).
Windows mouse/calibration keeps recording disabled; storage fixtures authorize no
actual capture.

The full service fixture exercises HTTP, JPEG republication, worker leases,
raw-result/meaning receipts and Stop without browser/models, sensors, input or
dataset files. Windows CI also exercises existing manual Start/status/UI routes
and shared diagonal/all-four-corner Save/rollback. Emulation is not qualification
of actual browser/GPU, camera cadence or application input receipt.

Remaining: ordinary-user app/configuration/selection, browser lifetime, portable
licensed/checksummed package, then fresh human-directed Legion acceptance with
actual GPU/resources, detail drag and failure cleanup. Milestone 2 and #19–#22
remain open until their actual deliverables are verified.
