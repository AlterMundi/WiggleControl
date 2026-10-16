# Camera-free absolute drag trace

`tools/trace_absolute_drag.py` prepares issue#8 qualification using the existing
finite `controller-absolute-stream` writer. It requires no writer changes, model,
camera, browser, window automation or compositor restart. Python handles fixture
files, packet scheduling and read-only Linux events; Rust still owns uinput and
release/watchdog behavior. The installed Beta1 source1cf0aed exposes the required
command, so its writer can be tested before adopting a newer candidate.

Status: locally prepared and device-free tested, **not receiving-qualified**.
The human is watching Farscape. Do not deploy or run it there until the desktop
is available and the finite input experiment has been coordinated. No physical
input devices were opened during development; no Pi operations occurred.

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
