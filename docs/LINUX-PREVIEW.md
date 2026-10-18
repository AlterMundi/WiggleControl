# One-hand Linux public preview — screen and detail

Agreed with Nicolás on 2026-10-18. This is the next delivery scope, not a claim
that the listed features are implemented, installed or physically accepted.
[GitHub milestone 1](https://github.com/AlterMundi/WiggleControl/milestone/1)
owns this delivery; the broader [roadmap](ROADMAP.md) remains preserved.

## Product and platform scope

Deliver an independently installable experimental Linux mouse for ordinary
applications, with manual Start/Stop, saved guided calibration and no required
Daimon Matrix, CompAII service or daimon-integrators configuration. Optional
feedback must remain separate from the detector; video/skeleton rendering is
not required for mouse use.

Initial physical qualification uses the existing Raspberry Pi 5/labwc/Wayland
single 1080p environment. Nicolás offers the ThinkPad for the Linux x86-64
qualification: prepare a matching artifact/install path, then record its actual
hardware, distribution, desktop, camera and GPU/backend behavior. Do not infer
camera access, immediate desktop availability or the exact ThinkPad model from
this offer. Qualify a clean standalone installation, not only a prepared host.
Only list platform combinations supported by observed results in the release.

The existing selected beta uses a headless Chromium GPU detector with Rust
gesture interpretation and Linux input delivery. Reuse that working path and
pinned dependencies; a pure-native replacement is not a prerequisite. The current
installer still prepares the older native path and reads camera settings from
daimon-integrators: it is not a complete installer for the selected beta.

Windows and macOS are later ports. Perception/gesture/filtering concepts can be
reused, but Linux evdev/uinput output, device permissions, V4L camera paths and
systemd/labwc operation are platform-specific. Each port requires its own input,
capture/lifecycle integration, permissions and physical qualification. An x86
Linux build does not establish Windows or macOS support.

## Required interactions

- Screen mode preserves comfortable full-screen calibrated pointing.
- Left click, natural double click, right click, intentional drag/release and
  gesture-controlled scroll work in ordinary applications.
- A deliberate Escape gesture releases controller-owned held input and emits
  one Esc; no repeat until positive rearm. Its application-dependent effect is
  distinct from controller Stop and never invokes a coordinated global Stop.
- Detail is a held precision modifier; selecting it before an action is the
  acceptable baseline, and changing it during an action is optional. It reduces
  pointer travel for the same hand travel and stabilizes rest while preserving
  very small deliberate movement. More smoothing alone or a large fixed dead
  zone is not sufficient. It cannot create additional sensor resolution.
- The initial detail gesture candidate is ring/little fingers flexed, leaving
  thumb/index available for pinch. Actual recognition and comfort must decide
  the final binding; right-click/scroll/Escape gestures must not conflict.
- Detail entry and exit must not jump the cursor or trigger clicks. Specify
  anchoring, return-to-screen mapping, edge access and recenter/reposition behavior
  before acceptance; restoring the original absolute map immediately can jump.
- Prefer detail independent of click/drag state if that is the natural design.
  Do not add action locks solely to enforce this plan. If changing detail during
  a held action is separate extra work, defer it: selecting mode before the action
  is acceptable. Neither design may jump the cursor, invent clicks or
  unintentionally release a drag. Qualify the chosen behavior, retaining explicit
  Escape/Stop and established tracking/process failure handling. Missing modifier
  evidence is not an open hand.
- Retain lightweight mode/activity feedback, physical input fallback, reliable
  scoped Stop and no startup acquisition or automatic restart.

## Work ownership and order

| Order | Work | Owner |
| --- | --- | --- |
| 1 | Complete button selection, natural double click, gesture scroll and one-shot Escape; select nonconflicting bindings | [#10](https://github.com/AlterMundi/WiggleControl/issues/10) |
| 2 | Integrate held detail into the actual calibrated absolute path; stable rest, small motion and action-boundary transitions | [#11](https://github.com/AlterMundi/WiggleControl/issues/11) |
| 3 | Standalone Linux installation/launch/rollback, selected GPU assets and independent configuration; ARM64 and offered ThinkPad x86-64 qualification | [#12](https://github.com/AlterMundi/WiggleControl/issues/12) |
| 4 | Integrated application/human qualification, measured software performance and honest current documentation/prerelease | [#13](https://github.com/AlterMundi/WiggleControl/issues/13) |
| Dependency | Actual normal-user input readiness after a coordinated boot, without capture | [#4](https://github.com/AlterMundi/WiggleControl/issues/4) |
| Dependency | Resolve or explicitly disposition the finite-trial Frame rejection from observed evidence | [#9](https://github.com/AlterMundi/WiggleControl/issues/9) |

Packaging preparation can proceed alongside gesture work. Reuse existing APIs,
helpers, regressions, recordings and accepted calibration. Do not require an
unrelated backend rewrite or new hardware to deliver a bounded public preview.

## Release acceptance

Relevant regressions cover event order, button cleanup, no unintended press on
reacquisition, transition stability, loss/ambiguity and process failure. Existing
source/receiving evidence can be reused where the implementation is unchanged.

Application/human checks cover left/right/double click, scroll, Esc, drag/drop,
loss/recovery and Stop; screen/detail checks include small targets, text selection,
slow drawing, rest stability, edges and return to screen without a jump. Reuse
the existing working targets: at least 18/20 intended clicks, ten completed drags,
60 relaxed seconds without unintended actions and at least five minutes of seated
use. Record added task counts and failures separately, alongside Nicolás's
acceptance of comfort. These targets are not achieved results.

Compare the prepared software-only increment in actual use; record camera cadence,
what latency includes, jitter/lag, CPU/memory and sustained thermal behavior under
the existing power policy. The camera/lighting investigation remains a separately
coordinated comparison, not a requirement to buy hardware or hide current limits.

Verify a clean standalone installation, update/config preservation, input/key
permissions and uninstall/rollback. Distribute pinned source-bound artifacts,
checksums and third-party licenses. Explain sensor and recording behavior; retain
private recording-enabled research trials without bundling personal footage or
configuration. Publish a tagged GitHub prerelease after scoped qualification;
select its exact version at delivery. CI artifacts are not a published release.

## Deferred work and historical incidents

Changing precision during a held drag/stroke is not a release requirement.
Nicolás clarified that it should be included if naturally independent of button
state; do not artificially isolate it to enforce deferral. Defer only separate
added work; choosing mode before the action remains acceptable. A second hand
as a modifier, bimanual tools, prayer entry, general macros, two-contact touchscreen,
multiple participants, four hands and Windows/macOS adapters remain future work.
Issues #1/#3 retain broad implementation/exploration scope; #2 retains calibration
and optional prayer history; #5/#6/#7 retain direct-touch/multi-hand research.
Those whole umbrella issues are not prerequisites for the one-hand preview.

Sticky-folder incident [#8](https://github.com/AlterMundi/WiggleControl/issues/8)
is closed on explicit human acceptance: it has not recurred. Its cause remains
unknown and it is not a release blocker unless it recurs. It is separate from
premature drag release and #9.

## Native goal handoff

The reconciled [goal specification](ONE-HAND-GOAL.md) carries the previous goal's
reliability/temporal-continuity/comfort work into this bounded delivery. Nicolás
cleared the old native goal and explicitly requested the new one on2026-10-18.
Native creation/state is recorded by the harness, not inferred from this document.
No camera session, desktop input test or reboot is started by recording this plan.
