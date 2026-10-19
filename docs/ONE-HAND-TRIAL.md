# Observed finite one-hand comparison

`provider-camera-manual[-preflight]` prepares a finite camera-worker session with
an explicit Start button. It combines the existing manual gate with the existing
producer-owned finite deadline. A receiving page may be inspected in standby
without a camera, input writer, recording or SDK worker. Obtain the human's fresh
ready acknowledgment before Start. After Start, the configured1..90seconds run
with the existing native countdown, private recording and Stop; elapsed time does
not reset when feedback is delayed. Existing automatic finite and until-Stop
commands retain their behavior.

```text
wigglecontrol provider-camera-manual-preflight SESSION SETTINGS ASSETS WARMUP_JPEG 45 PORT 1 --inject
wigglecontrol provider-camera-manual SESSION SETTINGS ASSETS WARMUP_JPEG 45 PORT 1 --inject
```

Preflight verifies configuration and selected public assets without opening
capture or input. The manual page states its trial duration. During cursor
reconstruction, a separate screen inset distinguishes the blue observed target
from the gold estimated target; these are screen coordinates, not camera pixel
positions. Fresh feedback explicitly labels a drag retained during a brief tracking
interruption. Delayed feedback keeps its unknown/last-state qualification.

## Receiving runner

`tools/one_hand_trial.py` verifies an exact executable and session/mode hashes,
requires the manual finite preflight contract, then runs that existing Rust source
and an independent event observer. Python performs operational composition only;
Rust still owns acquisition, perception, gesture interpretation, recording,
watchdog and uinput. Keep it beside `observe_absolute_input.py` and the existing
`trace_absolute_drag.py`. Reading or installing these files starts nothing.

A private receiving specification has exactly these fields; use its own paths and
hashes, and preserve the existing recording-enabled session and workspace:

```json
{
  "schema": "wigglecontrol.one-hand-trial/v1",
  "binary": "/selected/wigglecontrol",
  "binary_sha256": "EXPECTED_SHA256",
  "session": "/selected/session.json",
  "session_sha256": "EXPECTED_SHA256",
  "settings": "/selected/modes.json",
  "settings_sha256": "EXPECTED_SHA256",
  "assets": "/existing/public-provider-assets",
  "warmup": "/existing/synthetic-warmup.jpg",
  "port": 8876,
  "seconds": 45,
  "grant_observer_read": false
}
```

```sh
python3 -B tools/one_hand_trial.py --config PRIVATE_SPEC --preflight
python3 -B tools/one_hand_trial.py --config PRIVATE_SPEC --logs PRIVATE_LOG_PARENT
```

Use manually started, non-enabled receiving processes/units. Start the browser only
after `run.json` confirms the exact provider run and the observer's armed standby.
Avoid concurrent baseline/candidate sources or a shared browser-profile writer.
A native Stop or signal ends the owned provider; the runner retains/drains its
observer afterwards. An observer failure stops this trial instead of silently
continuing without evidence. A service wrapping this runner should send normal
Stop to the runner first (for example `KillMode=mixed` with a bounded stop timeout),
so the observer can outlive the provider's final release. This is diagnostic
lifecycle ordering, not a diagnosis of issue#8. Preserve the selected baseline and
rollback; do not overwrite a working beta or change physical-input settings.

Each new0700 run retains the specification/preflight, provider JSONL/stderr,
observer output and final run receipt. Existing Rust camera recording stays in the
session's private recording root. The runner writes no camera/input implementation
and does not create another conversation, model wakeup or background attention.

## Independent virtual-event observer

`tools/observe_absolute_input.py` accepts one exact owned producer PID or one
explicit writer PID, with expected native executable SHA. A producer observation
must be armed with no named virtual mouse already present; it selects that
producer's supported direct child writer and the uniquely named virtual node.
Multiple same-name devices, replaced bindings, different executables, process
restart/PID reuse and permission failures are reported rather than guessed.
Only the selected virtual node is opened read-only/nonblocking, without a device
grab. There is no physical event-stream discovery or read.

```sh
python3 -B tools/observe_absolute_input.py --producer-pid PID \
  --binary /selected/wigglecontrol --sha256 EXPECTED_SHA256 \
  --output NEW_PRIVATE_OUTPUT --wait-seconds 600 --seconds 60
```

If the actual new event node lacks read permission, explicit
`--grant-observer-read` reuses the installed `sudo -n setfacl` mechanism for only
that node. Its removal retires the grant. No global input group/rule is changed.
Native PID/executable plus unique new-node association assumes no concurrent
same-name writer; ambiguity invalidates the trace. This is operational evidence,
not a signed ownership claim or authority over another being's input.

`state.json` records armed/attached/ended times and periodic event/button counts.
`events.jsonl` preserves the kernel timestamp's unverified clock separately from
monotonic read timestamps, synchronization/loss markers and actual key edges.
Startup before attachment and initial button state remain **unknown**, even if
the attachment is fast. A complete after-attachment trace is not a complete
camera-session trace. Compare its boundary with the semantic journal before
claiming coverage of a particular press/release. Event loss, partial records,
truncation or an unreaped writer must not become proof of a successful release.
Filesystem/drop or application evidence remains separate from raw button events.

On a receiver Frame rejection, current native stderr includes a JSON
`wigglecontrol.delivery-rejection/v1` diagnostic after the existing error prefix.
It distinguishes failed arm/mode/source-age/source-order/count predicates and
retains receiver time, source/issued age, receive gap, previous source and arm
state before/after timing checks. Original trial d8f9bb9 has only the broad error;
do not retroactively attribute a specific predicate to it. This extra diagnostic
does not relax rejection or cleanup and does not prove the interruption fixed.
Keep the private stderr with the exact artifact and independent trace.

No new live human recording or input is authorized merely by these prepared
files. Describe one short owned task, verify receiving preparation and Stop, then
obtain fresh readiness. Preserve video, kernel/semantic evidence and human intent
labels separately. Source tests or appealing motion estimates do not establish
camera throughput, gesture accuracy, real cursor cadence or seated-use acceptance.

Validation: `python3 -B tools/test_one_hand_trial.py` covers actual process cookies,
wrong artifact/command rejection, changed specs, automatic/unbounded preflight
refusal, ended producers, permission failure and replaced-node cleanup without
opening physical input. Actual receiving lifecycle and kernel tests are separate.

## Guided calibration display lifecycle

The existing `calibrate-guided CONFIG PORT PROFILE` workflow remains camera-only
and saves a verified candidate before adoption. A receiving launcher must give
each attempt a fresh native hand overlay and close that overlay when its producer
ends, including retry/failure. Reusing an already running overlay can retain its
old display, timer and pending controls after the calibration server has exited.

For separate manually started user units, the calibration source can use
`ExecStartPre=/usr/bin/systemctl --user restart OVERLAY_UNIT` and
`ExecStopPost=/usr/bin/systemctl --user stop OVERLAY_UNIT`; its desktop launcher
restarts the source unit. A plain overlay `start` does not reset an existing
process. Keep the task-local Stop launcher able to stop both exact owned units.
Starting/retrying camera calibration remains a fresh human desktop action.

Receiving qualification reproduced the orphaned display with the calibration
source inactive and overlay still active. After the unit correction, two actual
desktop-launcher cycles, substituting a camera-free sleep for the producer,
observed different live source/overlay PIDs and both units inactive/success with
empty cgroups after each exit. The real calibration command was restored and
left inactive. This proves that launcher lifecycle, not a successful new hand
calibration. Previous accepted profile and failed-attempt evidence are preserved.

Corner verification now reports `move` until the existing review state has fresh
in-target evidence starting the corner dwell, then `hold`. Moving away, stale
input or tracking loss returns the guidance to `move`; finishing a corner waits
for the next target's painted acknowledgment. Previously every verification step
reported `hold` immediately, including while still at the opposite corner. This
contradicted the instruction to move to the new target. A regression covers the
second-pose→first-corner transition, entry, tracking loss and advancement, while
preserving all freshness, dwell and adoption requirements.

In the retained human retry both measurement poses reached24 usable samples.
The first verification target was upper-left, while all188 available mapped
positions stayed in the lower-right quadrant and none reached its tolerance.
It then timed out. This demonstrates the different failure boundary from the
earlier out-of-frame measurement; it does not establish what the human noticed
or intended. Keep the four-corner check and its real hold evidence intact.

Tracking rejection is also promoted into the guided instruction itself. A partly
out-of-frame hand requests inward movement, an absent/unclear hand requests
repositioning, and delayed tracking is labelled. Those states must not display
HOLD merely because the timed hold interval has begun. These are presentation
hints from the existing tracking assessment; they neither generate samples nor
relax freshness, geometry, count or adoption requirements. A regression clears
the hint and still refuses advancement when no real samples exist.


## Manual standby duration —2026-10-19

The observer originally expired after600seconds while waiting for manual Start,
which ended a prepared scroll trial before capture. Retained receiving evidence
confirms zero captured frames/no input and normal child cleanup; this was idle
expiration, not a hand-recognition failure.

An optional trial-specification `standby_seconds` now selects30..3600seconds,
default3600. This bounds idle waiting only. The finite90-second acquisition starts
on actual acquisition after human Start, and attached tracing still uses the
existing acquisition duration plus15seconds. No camera, model preparation or input
starts from increasing standby, and no completed trial is restarted automatically.
Existing specifications remain accepted; explicit600 retains their old idle bound.
