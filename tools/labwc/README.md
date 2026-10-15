# Local labwc pointer-focus candidate

This is an explicitly built compositor experiment for the controller roadmap,
tracked by [WiggleControl#5](https://github.com/AlterMundi/WiggleControl/issues/5).
The receiving desktop now selects the locally patched compositor after the
bounded qualification below. Other applications and physical control remain
separate acceptance gates.

The installed labwc0.20.2 clears pointer focus on touch down. In five finite
native-device cases, evdev received the subsequent stationary mouse press while
the Wayland client received none; moving first restored delivery. The tested
cases include one/two contacts and0/200/600ms delays. The same path exists in the
inspected upstream master9998771. See the maintained receiving research record.

The candidate enters the surface under the existing cursor on a press when no
touch points remain and pointer focus is missing. It emits no fake motion and
changes no calibration. The controlled nested and main-desktop comparisons below
qualify this handoff in the owned Wayland Chromium client. Compilation and
upstream unit tests alone do not establish that result.
Active touch, grabs, locks, pointer constraints and unrelated clients require
review/qualification before ordinary adoption. Wheel-only handoff is separate.

`source.json` pins labwc0.20.2's commit/archive, GCC14.2.0 and the public vendor
archive key. The dedicated ARM64 workflow builds baseline and patched binaries
in a Debian trixie container with the receiving distribution's public Raspberry Pi
package source. The public keyring is extracted from the pinned2025.1+rpt1
release, matching the receiving distribution. The older standalone key URL was
rejected by the build container's OpenPGP policy (SHA1 self-certification); package
and extracted key hashes are verified while repository signature checks remain
enabled. It does not change a host package or start a compositor. Actual
compiler/package versions, hashes and original source are retained with both
binaries for comparison and reversal. Artifact retention is14days.

```sh
tools/labwc/build_candidate.sh NEW_OUTPUT_DIRECTORY
```

This command needs matching native build dependencies; the workflow records them.
The tar artifact contains original full source, patch, build script, GPL license,
both binaries and provenance. Baseline/patched builds reuse one build tree. Existing
compositor source and desktop policy belong to labwc; this local experiment does
not claim upstream acceptance. No upstream issue or PR is submitted by these tools.

The patch and the included labwc source/license are GPL-2.0-only, preserving
[upstream provenance](https://github.com/labwc/labwc/tree/97f28877a343e062f3178d201f0248cd9c2610cf).
They are not relicensed under WiggleControl's original Rust license. The local
build helper is original project tooling; the resulting compositor is GPL.


## Candidate readiness —2026-10-15

[Build38716105142](https://github.com/AlterMundi/WiggleControl/actions/runs/38716105142)
passed on source691540e: native ARM64/GCC14.2.0, labwc0.20.2/wlroots0.20.2.
Baseline and patched builds each passed the three upstream suites for buffer,
string and XML helpers. Those suites do not exercise pointer focus. The artifact
contains source and GPL provenance plus both binaries. Baseline SHA256:
`12f13bed09db5f0c1b27bafa323dde551a348ad65c765d1bed96d02fa00bfce0`;
patched SHA256:
`6b9b09872f8b32e21bdef932461f83a664b28371396b671b1f0cd5644dbf17e0`.

Both were staged at a separate receiving user-local version and passed the dynamic
loader/`--version` check with matching xwayland/nls/rsvg/libsfdo support. No compositor
was started by preflight; the active compositor PID and system binary hash stayed
unchanged. Next qualify the focus behavior in a bounded nested desktop, retaining
base-versus-patch results before choosing ordinary receiving activation. No successful
stationary handoff or human camera acceptance is claimed from these build checks.

The external GPL experiment is excluded from the original Rust Cargo package.
Retirement removes only the staged version and archive; no desktop startup or
system binary has been changed by staging.


## Nested behavioral comparison —2026-10-15

The existing staged baseline and patch were each run in a bounded nested Wayland
session on the Pi. A private runtime/config/profile, explicit disabled activation-
environment updates and empty session hooks preserved the main desktop. Its owned
window was selected/fullscreened through foreign-toplevel requests and displayed
the existing LCARS input client at1920x1080. Touch traveled through the actual
uinput writer and parent Wayland backend; mouse clicks used the child compositor's
virtual-pointer protocol directly, avoiding the parent's own focus-loss defect.

All five direct stationary cases failed on baseline and passed on the patch:
one contact with0/600ms delay; two contacts with0/200/600ms delay. These are five
controlled protocol/client cases, not five independent human participants. The
extra two requested zero-displacement controls are **not a zero-motion test**:
[wlrctl0.2.2](https://sources.debian.org/src/wlrctl/0.2.2-2/pointer.c/) suppresses
zero motion before sending. They were no-operation controls, failing on baseline
and passing on the patch; the effect of an actually transmitted zero-motion
request remains unqualified. No such workaround was adopted.

Additional patched-session observations: no mouse press while one/two contacts
remained, including release of only one of a pair; one press after the final
contact; a DOM capture/acquire/release pair; and a click while the client held a
pointer lock followed by successful unlock. Session-lock security, every native
popup/grab and all application types are not proved by that pointer-lock test.
Both normal comparisons and guard input writers exited successfully. Temporary
nested sessions closed and the main owned workbench remained available.

Next receiving gate: build with distribution `/usr` and `/etc` resource prefixes,
retain/reverse the vendor package selection, then verify the complete native
uinput→libinput→main compositor→client handoff. The nested result does not claim
main-desktop activation or physical camera acceptance. The vendor wrapper and
its existing GPU/environment policy must be preserved.

## Main desktop adoption —2026-10-15

[Build38720964043](https://github.com/AlterMundi/WiggleControl/actions/runs/38720964043)
at source3d3121f retains the identical focus patch and uses distribution `/usr`
and `/etc` resource prefixes. Selected binary SHA256:
`0280557f83c6b86fc535f1612484fc2cc6bff6b039b682cedb3a7a775506fd7b`.
The vendor binary remains retained, SHA256:
`7bee5b51344f0ef4109def17e31244e54c6e920e670e5071fb0dd80d0b4a7667`.

The receiving-local selection helper first completed an actual file-selection
and reversal roundtrip, without restarting the active compositor. It then
selected the patch through a local `dpkg-divert`. Only the graphical session was
restarted; the vendor session wrapper and LightDM configuration were preserved.
The newly running compositor's executable hash matched the selected artifact.
The owned browser and independent compact panel were restored.

The complete native uinput→libinput→main labwc→Chromium route then delivered
one stationary mouse pointerdown in each of the five cases: one contact with
0/600ms delay and two contacts with0/200/600ms delay. All five fresh-motion
positive controls also delivered one. Before the patch, these same stationary
cases delivered zero despite kernel button events. The writer exited0, its
devices disappeared and the workbench returned at1920x1080 without page overflow.
Sensors stayed inactive; silent1.5GHz/fan0RPM remained selected.

This qualifies the observed Chromium handoff, not every client, session lock,
native popup/grab, wheel-only recovery or physical camera gesture. Issue#5 retains
those application/interaction follow-ups. Existing WiggleControl runtime binaries,
live thresholds and saved calibration were not replaced.

### Receiving reversal and maintenance

Resolve the selection helper and receipt through Shared Resources' receiving
overlay. Its `rollback` action verifies the retained vendor bytes, removes only
its own diversion and restores the original target. Restart the graphical session
after reversal to load that executable, coordinating any then-open applications.
Removing staged artifacts alone does **not** retire the selected compositor.

The local diversion deliberately preserves the patch across package updates;
vendor updates reach the diverted distribution path. If those vendor bytes change,
the helper refuses its old pinned rollback: inspect the new package and update
the maintained selection/reversal record before proceeding. This is a local GPL
experiment with ongoing maintenance, not an upstream package fix or acceptance.
