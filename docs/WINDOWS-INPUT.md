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

Independent process lifecycle, parent/child supervision and process-death cleanup,
actual writer launch/transport, camera/runtime integration and application receipts
remain implementation/qualification work in #19–#22. Win32 injected button state
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
