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
package source. It does not change a host package or start a compositor. Actual
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
