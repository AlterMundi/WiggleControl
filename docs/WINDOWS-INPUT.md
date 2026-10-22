# Windows native output adapter

Source implementation for [#19](https://github.com/AlterMundi/WiggleControl/issues/19)
and [milestone2](WINDOWS-PREVIEW.md). This is an adapter-library increment, not a
user-facing Windows mouse application or receiving acceptance.

`platform/windows/display` enumerates actual monitors in physical DPI coordinates,
using a temporary per-thread DPI context restored before return. It never changes
layout, resolution or user scaling. A selected-monitor snapshot maps the common
normalized calibrated pointer to the Win32 virtual desktop, including negative
origins. Invalid/outside targets and nonfinite points are rejected. Actual Legion
monitor/DPI configurations still require physical qualification.

`platform/windows/input::Mouse` implements the common output conversion interface.
It emits calibrated absolute motion, left/right/middle button edges and signed
fine wheel events for both axes. The preview uses the common absolute publisher;
relative movement and direct touch are explicitly unsupported here. Gestures,
detail mapping, click stabilization, scroll/inertia and freshness/Hold/watchdog
policy remain in the shared core. `output::State::with_backends` binds a configured
converter without arming or emitting any input.

`Injector` sends prepared events individually through Win32 `SendInput`, checking
each result. It tracks API-accepted held buttons separately from logical batches;
a rejected native release retains ownership and reports failure. Batch errors
attempt cleanup, repeated logical cleanup does not release an unowned button,
and Escape releases tracked input before its one key tap. Escape key-up is
attempted even if pressing fails. Normal destruction attempts any outstanding
owned release; consumers must also check explicit cleanup results. API acceptance
is not evidence that an application consumed an event.

The sender seam allows native Windows compilation and device-free tests of partial
API failure, failed key-up, held ownership, duplicate cleanup, both wheel axes and
the common writer watchdog through this actual conversion layer. CI fixtures use
the fake sender exclusively and never invoke `SendInput` or camera capture.

The independent writer/supervisor source is described below. Camera/runtime
integration and application receipts remain implementation/qualification work
in #19–#22. Win32 injected button state
must not be assumed to disappear like a Linux uinput device. In particular, a
Rust destructor is not a hard-process-death cleanup mechanism. Do not advertise a
ready Windows controller based on codec tests or this library.

Windows input integrity-level restrictions remain part of the receiving scope;
secure-desktop/UAC and game/anti-cheat support are outside this preview. Native
rejection reports the accepted count and available error code, without inventing
a specific cause or silently elevating privileges.

Official API references:

- [SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
- [MOUSEINPUT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-mouseinput)
- [GetMonitorInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmonitorinfow)
- [SetThreadDpiAwarenessContext](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddpiawarenesscontext)

## Source checkpoint —2026-10-22

Source9395d28 passes all five jobs in
[CI39722430502](https://github.com/AlterMundi/WiggleControl/actions/runs/39722430502),
including native Windows library/adapter tests and Clippy, the default binaries,
and existing Linux native/package checks. Mapping/cleanup tests use fictional
geometry and fake API emission; the code is not physically qualified on Legion.
The follow-up monitor-info pointer passes the entire extended SDK buffer before
casting to its base type; live display/DPI and actual input remain receiving checks.

## Independent writer and failure recovery

`platform/windows/writer` consumes the existing bounded JSONL `output::Packet`
stream in a separate process. It runs the common watchdog every20ms, including
under a busy producer, emits accepted held ownership/readiness receipts, and
cleans up before reporting terminal status. Stop, EOF, rejected packets, broken
receipt pipes and native failures all take the explicit release path. Native
operation checks the selected physical desktop snapshot at Start and every250ms;
a layout change terminates input rather than silently remapping it. No constructor,
spawn or calibration command arms input; fresh explicit transitions remain required.

The owning application uses `Writer::spawn` on manual Start, then `send` and
`stop`. Its separate monitor checks child exit and receipt silence independently
of the producer thread. A blocked writer is terminated and reaped before recovery
can emit any button-up. The parent retains both acknowledged ownership and possible
presses registered before pipe writes: a child can die after an API press but
before acknowledging it. At most64 packets may await acknowledgement. Recovery
emits only releases for that session's acknowledged/pending button or Escape bits,
never a new press, automatic restart or blanket release. A failed cleanup remains
unconfirmed in the outcome. Normal EOF/Stop returns a separate terminal outcome.

The hidden-console child uses Rust's maintained Windows `CommandExt` process
creation flags and ordinary bounded pipes, with a five-second startup allowance.
The active receipt watchdog uses the configured50–1000ms delivery budget; preview
runtime integration will select its normal150ms value. App hard exit closes the
pipe and the surviving writer releases. Writer crash/hang is handled by the
surviving parent. Simultaneously killing the entire process tree, OS termination
or failure of both recovery paths is not covered by a Rust destructor or this
protocol. Win32 injection shares desktop button state: recovery of an uncertain
press can also end a coincident physical hold of that same button. There is no
claim of independently classified Linux-style devices on Windows.

`windows-writer CONFIG_JSON` is an internal native input command, not the preview
launcher. `windows-writer-fixture` and `windows-writer-parent-fixture` route all
emission to fictional traces; they never call SendInput or display discovery.
Native-Windows process tests cover neutral startup, explicit Stop, EOF cleanup,
source-watchdog disarm/no re-press, malformed/oversized/stale input, a crash before
acknowledging a press, a stalled child, parent exit without destructors and rejected child/parent
releases that must remain explicitly unconfirmed. They
exercise real processes/pipes with a fake final emitter. These are source tests,
not actual input or Legion acceptance; #19 stays open for runtime/receiving work.

Process references: [Rust CommandExt](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html#tymethod.creation_flags)
and [Win32 process flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags).
