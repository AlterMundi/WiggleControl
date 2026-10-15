# Local labwc pointer-focus candidate

This is an explicitly built compositor experiment for the controller roadmap,
tracked by [WiggleControl#5](https://github.com/AlterMundi/WiggleControl/issues/5).
The existing desktop compositor remains selected until separately qualified.

The installed labwc0.20.2 clears pointer focus on touch down. In five finite
native-device cases, evdev received the subsequent stationary mouse press while
the Wayland client received none; moving first restored delivery. The tested
cases include one/two contacts and0/200/600ms delays. The same path exists in the
inspected upstream master9998771. See the maintained receiving research record.

The candidate enters the surface under the existing cursor on a press when no
touch points remain and pointer focus is missing. It emits no fake motion and
changes no calibration. This is an inference-driven candidate patch, pending
actual compositor/client regression and receiving activation. Compilation and
upstream unit tests alone cannot prove that it fixes the observed handoff.
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
