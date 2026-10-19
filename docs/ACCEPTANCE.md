# End-to-end acceptance checkpoint

Current delivery acceptance is scoped by [LINUX-PREVIEW.md](LINUX-PREVIEW.md)
and [#13](https://github.com/AlterMundi/WiggleControl/issues/13). Its planned
screen/detail and basic-action requirements are not yet accepted. Latest physical
feedback and exact receiving increments are maintained in [BETA-1.md](BETA-1.md)
and [CONTEXT.md](CONTEXT.md).

## First left/right human feedback — 2026-10-18

After the separate right-button trial, Nicolás reports that left and right clicks
seem to work well. This is positive first-use feedback for the two gesture
bindings, alongside the actual delivery evidence in BETA-1.md. It does not label
each recorded press as intended or establish a measured success/false-activation
rate. Natural double click, held detail, longer drag/loss tasks, relaxed no-action
and sustained comfort acceptance remain open. Gesture scroll/Escape still require
implementation before their human trials. The resolved sticky-folder incident
is not reopened by these ordinary drag-qualification tasks.

## Positive held-detail application use — 2026-10-18

Nicolás reports that detail worked very well, including deliberate click/detail
use in the X menu. The separate90s retry completed successfully with four observed
mode cycles, three left and two right press/release pairs, all confirmed
intentional by the human. Camera/input closure and child/device cleanup are
retained. See [DETAIL.md](DETAIL.md) for scope and source-bound evidence.

This accepts a first real application use of the modifier and clicking. Small
pointing/drawing/rest tasks, mode changes during held actions, longer comfort and
measured unintended-action rates remain open; gesture scroll/Escape and standalone
release work remain incomplete. The accidental task-window closure is not reported
as a controller fault.

## Positive Escape gesture/application use —2026-10-19

The d625bbc candidate completed a manual90-second recorded trial/error-free.
Twelve Escape outputs produced12 trusted keydown/keyup/dialog cancellations in
the owned Wayland application. Nicolás reports several successful attempts and
good operation; the exact intended attempt count was not separately annotated.
Camera/input closure, child reaping, device removal, private video/evidence and
unchanged normal configuration/calibration are retained. [ESCAPE.md](ESCAPE.md)
owns the matching source, adapter and physical evidence.

This accepts first physical Escape gesture/application use. Held-drag cancellation,
labelled prolonged-held no-repeat/rearm and ordinary-use false actions remain
focused follow-up work. Natural double click, gesture scroll, detail precision/
rest/drawing and sustained comfort/packaging remain milestone work.

## Positive first fist-scroll use —2026-10-19

The separate53caf00 candidate completed the manual90-second recorded trial
without error. The owned Wayland Chromium task received133 trusted wheel events
and688 scroll updates. Nicolás reports that it worked and looked quite good;
he expects a full page to be easier to use than the small exercise pane and
explicitly defers further polish. This accepts basic first physical scroll use,
not individually labelled direction/release/inertia cases or a false-action rate.

The source captured1213 frames and interpreted1206 observations. Camera/input
closed, provider/observer exited0 and were reaped, and the selected virtual device
was removed without an incomplete kernel trace. Private video and source/config/
application evidence are retained. The next scroll usability comparison should
use a full page or ordinary application. See [SCROLL.md](SCROLL.md).

Natural double click, broader action/drag/loss qualification, detail precision/
rest/drawing, sustained comfort and standalone packaging remain milestone work.
This trial does not deploy the later source-only transition or launcher fixes.

## Historical six-milestone baseline

The table and finite trial protocol below retain the original broader
DEVELOPMENT.md checkpoint. They do not select the current beta/runtime, describe
current test counts or require multitouch before the one-hand Linux preview.
QUALIFICATION.md preserves dated observations.

| Plan requirement | Current evidence | Remaining acceptance |
| --- | --- | --- |
| Versioned observations, bounded replay, pinch/drag/scroll, neutral loss/stale/Stop/reacquisition | Rust contracts/fixtures, 31 native-feature tests; fictional jitter, malformed time/input and hand replacement covered | Real noise/lag tradeoff and comfort of the selected smoothing/cursor/scroll mapping |
| Linux relative mouse, libinput recognition, narrow grant, reversible setup | Real Chromium Wayland and VLC XWayland click/drag/wheel/fullscreen; writer stall/death release; original ACL reversal/reinstall | Human workspace/recentering, physical fallback while using camera input |
| Native backend/model/compiler/license pins and comparison | ort/CPU runtime qualified on ARM64; exact tract failure retained; OpenCV positive coordinate comparison; static initial/tracked cost and local session CPU/RSS/FPS | Same physical scene comparison with browser baseline; tracked-hand resources/jitter/thermal context and model coverage decision |
| Camera-to-mouse, existing visible panel, local operation, failure/recovery | Finite mouse/touch sessions and panel Start/Stop; offline network namespace; ENOENT cleanup and actual UVC loss/rebind; no startup capture | Intentional live clicks/drags in desktop/Chromium/VLC, relaxed false actions, seated comfort, actual hand loss/reacquisition and held release |
| Direct touch, stable type-B slots, explicit single-output mapping, independent mode | Trusted one/two contacts in touch-aware Chromium; scale/rotation interpretation; producer-array/identity ambiguity replay; finite watchdog removal | Camera-driven one/two contact manipulation; real crossing/occlusion/loss identity; mouse-only fallback behavior with real hand input |
| Packaging, bounded resources/retention, rollback, permissions, shared maintenance | Local pinned artifacts/licenses; 20 owned input summaries; actual original-grant/file rollback/reinstall; maintained Shared Resources | Qualified final calibration/configuration, hardware/human limits and portable shared skill after acceptance |

## Receiving human trial

Coordinate the camera/desktop before starting. The current finite control is
`daimon-integrators input SECONDS [mouse|touch]`, maximum 120 seconds, with scoped
Stop on the floating panel. Keep the physical mouse available. No session is
started solely because this checkpoint is read.

1. Confirm visible preparation, camera-active indication and microphone idle.
   Use the local qualification page; establish comfortable seated hand range.
2. At the intended distance (record actual distance, initially about 1 m and
   later the existing 2 m target), attempt twenty intended clicks and ten drags.
   Record successes, misses, unintended actions and jitter separately. The proposed
   click criterion is at least 18/20; it is not an achieved result.
3. Keep relaxed hands visible for sixty seconds; count unintended actions.
   While deliberately holding a drag, remove/occlude the hand and use Stop;
   verify neutral input, then open-before-repress on reacquisition.
4. Operate actual desktop items, Chromium and VLC, including fullscreen and
   simultaneous video playback. Compare the same scene/settings to the existing
   browser tracker; resource timing does not substitute for success counts.
5. In touch mode, manipulate the known touch-aware target with one and two
   contacts; cross/occlude hands and observe ownership/release. Check a mouse-only
   client and record its real fallback rather than assuming native zoom support.
6. Record five minutes of seated comfort across explicitly started finite sessions,
   and whole-pipeline latency/FPS/CPU/RSS/temperature/throttling during accepted-hand
   tracking. Confirm the final chosen calibration and limits with the human.

Physical electrical cable hotplug remains a separately unobserved hardware case;
driver-visible camera loss/recovery is already qualified. Browser static-reference
coverage differs from the chosen ONNX pipeline, whose single-palm result is also
reproduced in original OpenCV. Neither case proves live user accuracy. Final shared
guidance/release must preserve these limits and the actual acceptance results.
