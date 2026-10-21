# Explicit small dataset recordings

Nicolas changed the recording policy on 2026-10-21 after a continuous mouse run
was measured writing about 6 MiB/s on Legion (about 21 GiB/hour if sustained). The 512 MiB limit applied per segment;
rotation did not cap the take. Ordinary operation must not accumulate video.

## Selected policy

Ordinary mouse use, until-Stop operation and calibration have recording disabled
by default. Existing `recording_directory` settings alone no longer opt in.
Fresh prepared configurations explicitly set `recording_enabled: false`.
Capture and input can operate without a recording directory.

For an explicitly requested dataset sample, set `recording_enabled: true` in a
selected private session configuration and choose a finite 1..30-second trial.
Until-Stop and longer trials reject recording before loading assets or opening
the camera/input. There is no periodic sample capture, background restart,
automatic retention deletion or implicit extension to another sample.

The recorder additionally limits streamed take bytes to 256 MiB, the selected
recording directory to 2 GiB, and preserves at least 2 GiB of available filesystem
space before each write. Manifest, configuration, video, frame-index and observation
writes share one budget. The directory budget includes already retained files;
a directory-scoped advisory lock excludes concurrent supported recorder takes.
The timeout also bounds writes to 30 wall-clock seconds after recording creation.
External writers can consume space concurrently: this is an observed reserve,
not a filesystem quota or a guarantee against unrelated disk exhaustion.

A reached limit or storage/write failure rejects further recording writes and
uses the existing session-error cleanup, closing capture and releasing owned
input. Partial retained samples remain available; nothing is silently deleted
or overwritten. The reserve check uses the actual directory file descriptor and
unprivileged available blocks. It fails visibly when filesystem observation fails.

## Existing footage and evidence

Nicolas authorized deleting accumulated footage unless linked to a concrete code
test. The receiving cleanup removed 6 unreferenced Legion takes (~4.50 GiB) and 22
unreferenced Pi takes (~4.13 GiB). The small synthetic review fixture, accepted
calibration profiles, models, package/runtime files and other media were preserved.
Private receipts retain exact paths/counts; no raw footage enters this repository.

Source tests, installed default behavior and actual manual receiving acceptance
remain separate. A source-only change does not disable recording in an older
installed binary. Do not restart a stopped camera to validate storage policy:
use camera-free preflight and fictional recorder fixtures, then deploy into idle.
