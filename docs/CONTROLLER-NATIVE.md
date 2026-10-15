# Native controller session and independent delivery

This opt-in source path connects native perception, the exclusive controller,
manual preview controls and an independent mouse/touch writer. The existing
fixed-mode and calibrated commands remain available. The2026-10-15 ARM64 candidate is staged separately and has partial scripted
receiving qualification below. Live perception/control and real-hand comfort remain
unqualified.

## Prepare and start

Build with `--features native-vision`. Use the receiving session configuration and
an explicit mode-settings file (see [the example](../tools/controller-modes.example.json)).
Their gesture settings and touch workspace must agree: preflight rejects divergent
values before initializing inference or opening a camera. Session `pinch_3d`
selects the shared detection conversion; mode settings do not override it.

Session `pair_separation_widths` selects the crowded-pair rejection guard for
camera sessions and offline sequences through the same perception factory. It
accepts0.8..2.0; omitted values retain the existing0.8 policy. The experimental
1.2 candidate prevents the observed early second contact in two rendered crossing
layouts, but cancels earlier and does not maintain input through occlusion.
It is an explicit candidate setting, not a new installed/default policy.
Both config-check commands report it without loading models or opening sensors.
Offline `--pair-separation` overrides the config only for that sequence check.

```sh
wigglecontrol config-check-controller SESSION.json SETTINGS.json
wigglecontrol camera-check-controller SECONDS SESSION.json SETTINGS.json PORT
wigglecontrol run-controller SECONDS SESSION.json SETTINGS.json PORT
wigglecontrol run-controller-until-stop SESSION.json SETTINGS.json PORT
```

Only preflight is camera-free. Camera-check acquires the selected camera and
provides the same semantic feedback without virtual devices. Run also creates
mouse/touch devices. Finite preview sessions use1..90 measured seconds plus the
existing bounded preparation. The measured clock starts after the first view is
painted; until-stop remains explicitly manual. Follow the existing human-ready
and recording rules before acquiring a camera. No command starts at boot.

This version uses relative mouse movement/precision and the explicit direct-touch
workspace. It does not adopt or rewrite a saved absolute pointer calibration.
Prayer/guided calibration retain their separate existing workflow. Visual macro
classification is not connected in this path (`visual_macros:false`); no gesture
can start an idle camera.

## Visible operation

The existing loopback preview shows current mode, held state and an explicit
primary-hand selector. Mode changes and hand claims release the old input and
require positive open evidence again. Two-contact mode uses both tracks and
disables the one-hand selector. Track IDs do not identify participants.

Crowding feedback asks the human to separate/open the hands. After tracking
returns, `awaiting_open_hands` comes from the actual per-hand gesture engines;
closed returning tracks remain blocked until fresh open evidence. The indicator
clears independently for each hand rather than guessing rearm from detection.

A transient frame/transport failure marks feedback delayed and retries, keeping
scoped Stop available. A failed Stop request remains unacknowledged rather than
closing the view. An acknowledged request ends this view and directs the human
to the independent capture panel; it is not a sensor-closure receipt. Cached
server metadata ages even in until-stop mode, and the browser includes transfer/
decode time before marking feedback fresh or acknowledging trial readiness.

Manual commands fence observations at their monotonic control time. A queued
open frame captured before the control cannot rearm a hand that is now closed.
The first post-control fresh open observation must be new. This prevents a
buffered camera frame from undoing the mode-transition rearm rule.

Pause stops input while capture/recording remain visible; Resume requires open
rearm. A pause during preparation remains paused when the measured clock starts. Stop ends this owned session. Feedback explicitly reports recording and
input state. A delayed view does not display cached hold state as current;
check the independent local panel for observed capture state after disconnection.

Controls use the existing exact loopback Host/same-origin contract, with one
pending controller command. HTTP acceptance means queued, followed by observed
producer state; it is not an OS delivery receipt. Legacy preview sessions reject
controller controls. Selected mode settings are retained privately beside the
session configuration and raw recordings. Preview control events retain their
exact monotonic time and resulting controller state as separate event rows; they
are not gesture-intent annotations.

## Delivery boundary

The writer reuses existing evdev mouse and direct-touch factories/state encoders.
It owns both file descriptors and consumes only one mode's input per frame.
There is no competing mouse/contact stream. It does not perform perception.

`controller-stream SECONDS GAIN WATCHDOG_MS` and
`controller-stream-until-stop GAIN WATCHDOG_MS` explicitly open devices.
The producer supplies bounded JSONL packets:

- `transition`: sequence, issued monotonic time, mode and enabled state. Release
  mouse/buttons and both touch slots before arming the selected mode.
- `frame`: sequence, issued/source monotonic times, mode, mouse events and the
  complete contact set. Reject stale/future/repeated source, repeated sequence,
  unmatched mode, oversized output or incompatible mixed streams.
- `stop`: release and terminate this writer.

Producer/consumer timestamps share the same host's Linux monotonic clock. The
independent watchdog allows50..1000ms. Expiry releases both devices and latches
updates off; another explicit transition is required. Replaying held contacts
cannot reactivate input after expiry. Source watermark remains across transitions.
EOF, malformed input, error and finite expiry release before destroying devices;
closing the sole uinput file descriptors also removes them on writer death.

The controller tick and fresh-observation paths publish neutral transitions on
loss/staleness. Idle ticks cannot keep held output fresh indefinitely. Old
pre-control frames cannot supply new-mode arming evidence. Pixel/model accuracy,
application gestures and compositor device classification require separate proof.

## Camera/device-free qualification

```sh
wigglecontrol mode-replay SETTINGS.json < controller.jsonl
wigglecontrol delivery-replay GAIN WATCHDOG_MS < packets.jsonl
wigglecontrol pipeline-replay SETTINGS.json GAIN WATCHDOG_MS < controller.jsonl
```

The full pipeline serializes actual writer packets, decodes them through its
consumer contract and emits actual Linux event codes using the existing state
encoders. Its clocks are explicitly supplied by replay commands. It opens no
sensor or uinput descriptor and reports `system_input:false`.

Tests cover button/two-slot release, transition-before-repress, watchdog latching,
source/sequence/mode integrity, malformed/EOF cleanup, pending HTTP controls and
pre-acquisition config agreement. The native UI was exercised on Cuevanico at
1920x1080 with a device-free Rust pipeline and retained synthetic images, explicitly
labelled UI TEST / sensors off / system input off. All five modes, hand selection,
held-mode change and pause/resume were observed. No page overflow or console
exceptions;390px layout had no horizontal overflow. A fixture proves this UI and
semantic path, not live acquisition or compositor effects. Its temporary server
was stopped and the owned browser returned to the research workbench.

## ARM64 candidate build

The Rust workflow tests and builds this native path on an
[ARM64 Ubuntu24.04 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
with the existing Rust1.85.1 pin. This avoids compiling on the receiving Pi and
requires no new local cross toolchain. Its `wigglecontrol-arm64-candidate` artifact
contains a tar archive preserving executable mode, LICENSE/NOTICE and a manifest
with source commit, compiler, architecture, feature selection and binary SHA256.
It contains no model, inference runtime, camera recording or receiving config.
Artifacts expire after14days; the maintained workflow can build another commit.

A candidate build is not an installation or receiving qualification. Before use,
verify the run's source commit and manifest hash, compare the target's libc with
the binary's requirements, then stage at a separate user-local path and run help
and config-only preflight. Retain the selected prior binary and configurations.
Camera/input acceptance follows the existing human-ready and receiving rules.

## Receiving candidate checkpoint —2026-10-15

Candidate [2f63f29](https://github.com/AlterMundi/WiggleControl/commit/2f63f29)
passed [CI38711639096](https://github.com/AlterMundi/WiggleControl/actions/runs/38711639096),
including native ARM64 tests/build and existing x86/default/installer checks.
The native suite has98 tests. The binary SHA256 is
`6d424ef94ca62b362e69e4b0409059366262d46afee15991e72c3e073c25a31b`.
It requires glibc through2.39; the receiving Pi reports2.41. Help and camera-free
controller preflight succeeded there. The candidate, copied session/settings,
licenses and manifest are retained at a separate versioned user-local location
in the Shared Resources receiving overlay. Existing production/calibration
executables, thresholds and saved profile were preserved. Only the candidate
copy explicitly uses the saved diagonal rectangle as its touch workspace.

A finite scripted writer test used the owned Chromium Wayland qualification
page at1920x1080. CDP prepared/read that page; actual events came through the new
native writer, uinput, libinput and labwc. Observed:

- Mouse button down followed by up on a pointer→precision transition.
- Two simultaneous trusted contacts and movement interpreted by the client as
  scale1.50438 and rotation4.33389degrees; changing back to pointer ended both.
- With fresh pointer movement after touch, another mouse down/up pair and an
  independent watchdog release after the scripted producer stopped updating.
- Successful finite writer exit, no remaining WiggleControl input device and
  the owned browser restored to the research workbench. No console exceptions.

One attempt deliberately reported here as incomplete: a stationary first mouse
press after touch did not reach the client. Adding fresh pointer motion made the
repeat work; the responsible boundary is not localized. [#5](https://github.com/AlterMundi/WiggleControl/issues/5)
tracks stationary handoff acceptance. The first test generator also accidentally
exceeded the300ms watchdog before its next frame; input correctly remained
latched off and the writer terminated on rejected continuation. The corrected
scripted generator maintained explicit fresh test frames, then deliberately
stopped them for the watchdog check. This was not a camera-data replay.

The pre-existing named-operator uinput ACL was missing after boot despite its
installed udev rule. It was restored without widening access. Device registration
became visible after opening the node; startup/module bootstrap remains a
hypothesis and [#4](https://github.com/AlterMundi/WiggleControl/issues/4) owns
persistent install/upgrade/rollback qualification. No reboot was performed.

The independent panel observed camera/microphone inactive after this test,
silent mode, fan0RPM and1.5GHz. No camera/microphone session ran. Combined-device
scripted delivery does not qualify live native session switching, hand accuracy,
stationary handoff, absolute precision, participant routing, visual macros or
comfort. The candidate is opt-in and incomplete, with no startup service.
To retire it, remove its versioned candidate directory/archive and test artifacts;
there is no active process to disable and the selected original runtime remains.


## Input preparation and handoff investigation —2026-10-15

[8ffe587](https://github.com/AlterMundi/WiggleControl/commit/8ffe587) adds the
maintained `tools/prepare_input.sh` helper and installer integration. Isolated
install/upgrade/interruption/rollback passed, including a pre-existing module file
and an older manifest without that path. [CI38713826229](https://github.com/AlterMundi/WiggleControl/actions/runs/38713826229)
passed all jobs. The helper was adopted independently of runtime binaries.

A controlled receiving test verified no existing uinput users before unloading
only that module. The kernel's own static-node description recreated a0600 node:
operator open failed with EACCES and no driver registered. Running the helper
loaded the module, retained its boot dependency in the original rollback and
made normal-user open succeed. No virtual device or sensor session was created.
Production/calibration executables and the original configuration/profile retained
identical hashes. This reproduces the bootstrap gap and verifies preparation;
an actual reboot remains unobserved, so#4 retains that acceptance item. See
[INSTALL.md](INSTALL.md#permissions-and-original-baseline) for the supported route.

The stationary handoff was then isolated with read-only observation of only the
finite test writer's two evdev devices. All five stationary presses reached the
kernel but produced no mouse pointerdown in Chromium. Fresh pointer movement
before the repeated press produced one in all five. Cases: two contacts with
0/200/600ms pause; one contact with0/600ms pause. Native and client traces are
retained separately. The generator uses monotonic time and full explicit synthetic
contact frames; it captures no camera, microphone or unrelated input device.

Installed versions: labwc0.20.2/wlroots0.20.2/libinput1.28.1. The pinned labwc
[handling of touch](https://github.com/labwc/labwc/blob/97f28877a343e062f3178d201f0248cd9c2610cf/src/input/touch.c)
clears pointer focus; its button handler makes the cursor visible without an
explicit missing-focus re-entry. Inspected upstream master9998771 retains that
path. This is a source-grounded explanation to qualify further, not proof that
an untested compositor patch works. The [local candidate](../tools/labwc/README.md)
adds re-entry before a stationary press when contacts have ended and focus is
missing. A separate off-host workflow builds baseline and patched ARM64 binaries;
there is no automatic desktop activation or compositor restart. Physical camera
control and the full roadmap remain unfinished.


The local candidate build subsequently passed as CI38716105142/source691540e.
Baseline and patched ARM64 binaries each passed three upstream helper suites and
receiving loader/version preflight. Exact binary hashes and scope are retained in
[the candidate guide](../tools/labwc/README.md#candidate-readiness-2026-10-15).
The active compositor PID and system binary hash stayed unchanged; no patched
desktop was started. Nested behavioral comparison remains the next gate for#5.


The subsequent nested comparison delivered the stationary press in5/5 patched
cases versus0/5 baseline. One/two contacts and0/200/600ms gaps were controlled;
touch used the real writer/parent backend and mouse used the child virtual-pointer
protocol. Active-contact exclusion, final-contact re-entry, DOM capture and client
pointer-lock lifecycle passed. An attempted zero-motion control was discovered to
be omitted by wlrctl, so it does not qualify that alternative. See the candidate
guide for method/limits. Main desktop activation and actual native uinput handoff
remain the next receiving gate; no camera acceptance follows from these tests.

### Main desktop follow-up —2026-10-15

The distribution-prefix labwc candidate from source3d3121f/build38720964043 is
now selected through a receiving-local reversible package diversion. An actual
file-selection/rollback roundtrip passed before activation; after the targeted
graphical-session restart, the new running executable matched the artifact hash.
The vendor binary, wrapper and graphical-session configuration remain retained.
See [the compositor guide](../tools/labwc/README.md#main-desktop-adoption-2026-10-15)
for hashes, reversal and package-update maintenance.

The same finite native writer diagnostic now reports one stationary pointerdown
and one fresh-motion control pointerdown in each of all five cases. One contact
used0/600ms gaps; two contacts used0/200/600ms. This is the complete
uinput→libinput→main compositor→owned Wayland Chromium path, unlike the nested
virtual-pointer test. Writer exit0, removal of both virtual devices and return to
the owned1920x1080 workbench were observed. The independent panel was active and
reported camera/microphone inactive, silent1.5GHz and fan0RPM. No physical trial ran.

The observed stationary Chromium failure is repaired in this selected local
compositor. Other supported clients, session locks, native grabs/popups and
wheel-only handoff remain unqualified in#5. The selected WiggleControl runtime,
45/65mm thresholds and saved calibration were preserved; this does not qualify
live camera mode switching or the full controller roadmap.
