# Camera-free absolute drag trace

`tools/trace_absolute_drag.py` prepares issue#8 qualification using the existing
finite `controller-absolute-stream` writer. It requires no writer changes, model,
camera, browser, window automation or compositor restart. Python handles fixture
files, packet scheduling and read-only Linux events; Rust still owns uinput and
release/watchdog behavior. The installed Beta1 source1cf0aed exposes the required
command, so its writer can be tested before adopting a newer candidate.

Status: the human freed the receiving desktop and resumed the one-hand goal on
2026-10-16. Five scripted desktop cases below have now executed on the installed
Beta1 writer. The same five cases also ran in a PCManFM window; issue#8
reproduction and human gesture acceptance remain pending. No camera acquisition or controller-setting change
was part of these tests.

## Preparation and execution

Create a new private workspace; its parent must exist. The helper creates two
owned empty directories and records their filesystem identities. It never removes
or resets them automatically. By default they live under `WORKSPACE/surface` for
a PCManFM window test. For the desktop case, pass `--surface-directory` with the
actual existing desktop directory: only two new uniquely named folders are added.
Each case needs a fresh fixture, so previous results stay intact.

```sh
python3 tools/trace_absolute_drag.py prepare /PRIVATE/NEW-WORKSPACE
python3 tools/trace_absolute_drag.py plan /PRIVATE/NEW-WORKSPACE \
  --case release --source 0.2 0.3 --target 0.7 0.8
```

`prepare`, `plan` and `inspect` open no input device and start no writer. Coordinates
above are illustrative normalized screen positions. Before a real run, arrange
only the owned fixture, locate its source/target icons and measure their actual
positions on the selected output. Preserve existing applications and physical input.
Test a PCManFM window and its desktop surface separately; one does not qualify the
other. The helper itself does not focus/open/close windows or take screenshots.

**`run` injects actual pointer movement and button input:**

```sh
python3 tools/trace_absolute_drag.py run /PRIVATE/NEW-WORKSPACE \
  --case release --source X Y --target X Y --binary /PATH/TO/SELECTED/wigglecontrol
```

The named-operator uinput grant must already work. The helper refuses a preexisting
named virtual mouse, starts one writer with a12 second limit, and attaches to the unique newly
appeared virtual `WiggleControl Absolute Mouse` event node before sending any input.
This binding requires no concurrent writer. It never opens physical event streams
or grabs an input device. If event-node read permission is absent, default behavior
ends the owned writer without injecting. Explicit `--grant-observer-read` can reuse
`sudo -n setfacl` to grant only the operator read access to that new virtual node;
its removal retires this temporary ACL. It changes no global input group/rule.
Actual sudo/read success must be observed, not inferred from historical grants.

| Case | Termination being compared |
| --- | --- |
|`release` | Button up at target, then1s of fresh neutral frames before Stop. |
|`stop-held` | Stop while held at target; writer cleanup releases and removes device. |
|`watchdog` |800ms without producer packets while stdin/device stay open; then Stop. |
|`producer-eof` | Close only this producer's stdin while held; observe writer cleanup. |
|`writer-death` | SIGKILL only this child writer while held; observe removal/client recovery. |

All cases first hover, press, hold and move to the target with50ms fresh packets.
The helper requires a synchronized kernel press before drag motion and refuses
catch-up bursts after100ms scheduling lateness. Errors trigger cleanup of only its
own child. It never restarts PCManFM, labwc or another service. If cleanup fails,
the report preserves unknown/unreaped state rather than claiming closure.

## Evidence and limits

Private `trial/` retains the plan, actual sent packets with monotonic times,
selected binary hash, writer stderr, raw events and a terminal result. Events
include synchronization and loss markers. Kernel timestamps retain their original
unverified clock; `read_monotonic_us` is when bytes were read, not hardware event
time. Do not subtract those clocks to invent latency. The parser currently targets
the64-bit Linux ABI used by the development and ARM64 receiving hosts.

`packets_sent` proves writes to stdin, not kernel/application delivery. Synchronized
`kernel_buttons` separately records button edges. `SYN_DROPPED`, partial records or
unsynchronized final buttons make the trace incomplete. Writer death may remove
the device without an observable explicit kernel button-up; compositor/client
recovery is separate evidence, not a parser assumption.

`filesystem_drop:moved` requires the original empty directory's device/inode at
the target and absence at its old location. A copy does not pass. A failed/unfinished
move remains `not_proven`; inspect again after delayed application work if needed:

```sh
python3 tools/trace_absolute_drag.py inspect /PRIVATE/NEW-WORKSPACE
```

Even a proven move does not establish that the floating icon/grab disappeared.
`desktop_grab_released` stays unknown. After each coordinated run, observe actual
pointer movement and the owning application's remaining drag state, retaining
that separate evidence. Do not repeat a case while its desktop drag is unresolved.
Keep failed fixtures/evidence; remove only verified owned empty test paths when
they are no longer needed. Never use a human folder as the fixture.

Six local tests cover identity-based movement versus copying, nonempty-fixture
refusal, no-process/no-device planning, overlapping-writer refusal, permission
failure before injection, fragmented/synchronized event decoding and loss flags.
The five plans also pass through the actual Rust device-free delivery consumer:
normal/Stop release, independent300ms watchdog release, and EOF cleanup. EOF is
explicitly not a real SIGKILL qualification. CI is configured to repeat these checks on default
x86 and native ARM64 builds. No result here diagnoses issue#8 or closes human
acceptance.

## First receiving desktop comparison —2026-10-16

The unchanged installed1cf0aed writer (SHA256
`f4ad91eaf5f8b1e5f44c0e87d68e57ec6ea71666d23094c4025e068234a40eb9`)
ran one fresh owned desktop fixture per case after the human freed the Pi. Each
trace retained synchronized down/up,47 observed kernel events, no SYN_DROPPED or
partial/unsynchronized records, original directory identity at the target, reaped
writer and removed virtual device. No physical event stream was opened. The
optional ACL was needed and verified only on each newly created virtual node.

| Case | Writer exit | Kernel button edges | Original empty folder moved |
| --- | --- | --- | --- |
| Explicit release,1s neutral before Stop |0 | Down, up | Yes |
| Stop while held |0 | Down, up | Yes |
| Producer silence with stdin open |0 | Down, up | Yes |
| Producer EOF |0 | Down, up | Yes |
| SIGKILL of only the owned writer |-9 | Down, up | Yes |

For the watchdog case, observer-read button-up occurred300.933ms after the last
sent fresh frame. Both are receiving-host monotonic measurements; reader delay is
included, and this is one observation rather than a guaranteed latency bound.
The SIGKILL case also produced observable up on this receiving system; this does
not mean the killed writer executed cleanup or guarantee another kernel's result.

After each trace, a separate finite no-click absolute writer moved between two
hover points while remaining alive for cursor-inclusive screenshots. The inspected
right-hand screenshot showed the cursor without a floating folder. This is a
separate CompAII visual observation, not an authoritative compositor grab query or
human acceptance; the raw trace correctly retains `desktop_grab_released:null`.
All fixture directories were removed only after their original identities, empty
contents and observed completed move were verified; raw plans/events/results and
screenshots remain private in the receiving runbook's evidence root.

The first hover-only screenshot attempt failed because the installed grim reports
`jpeg support disabled`; the subsequent PNG probe completed and removed its owned
writer. No application/compositor/service restart or setting change was used to
obtain the successful results. The old workbench and camera remain stopped.

These five single-case results **do not reproduce or diagnose issue#8**. They show
that the installed writer and current desktop can complete this controlled
sequence, including the tested terminations. The separate window comparison below
extends this checkpoint; no unobserved cause or general fault tolerance follows
from these runs.

## First receiving window comparison —2026-10-16

The same installed1cf0aed executable repeated all five cases in the measured front
PCManFM window, with two owned views of the same private fixture surface open.
Each fresh empty source directory moved by original identity into its target.
All five retained47 synchronized kernel records with one down/up, no loss or
incomplete record, reaped writer and removed device. The inspected cursor-alive
hover image showed no floating folder after each case. Watchdog observer-up was
301.012ms after the last sent frame, including observer delay. SIGKILL returned-9
with observable kernel up here; the other cases exited0. No app restart or camera
was used. The ten window-fixture directories were retired only after the scoped
move/identity/visual checks; raw events and screenshots remain private.

These are five single cases on a second application surface, not reproduction
or a fix of issue#8. The original failure remains open. A subsequent comparison
must retain the kernel stream during closer-to-usage Beta interaction rather than
assuming this controlled packet plan covers the unknown original sequence.

### Application double-click check

A fresh receiving GTK3 settings instance reported400ms double-click time,
25px double-click distance and8px drag threshold. PCManFM's process maps GTK3.
Two exact-position click pairs then used the same traced installed writer over an
owned empty folder in the window:

| Requested press interval | Observed kernel press interval | Button edges | PCManFM result |
| --- | --- | --- | --- |
|500ms |499.968ms | Down/up/down/up | Folder selected, stayed in parent |
|300ms |299.986ms | Down/up/down/up | Entered that exact source folder |

Native toplevel titles and retained screenshots independently establish those
application results. This brackets actual temporal behavior around the reported
400ms setting; it does not measure its exact boundary or spatial tolerance.
No hand source or pinch recognition participated. The result rules out a universal
inability of the installed mouse writer to express an application double-click;
it does not diagnose the human's failed double pinch. Both owned test views were
closed with individually guarded finite native clicks, and remaining empty click
fixtures were identity-checked before removal. The original desktop process stayed
in place. No human folder, desktop setting or physical input configuration changed.
