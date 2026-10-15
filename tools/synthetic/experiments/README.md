# Retained rejected experiments

These are reproduction artifacts, not installed runtime selections.

`palm-refresh-b58c975.patch` applies to WiggleControlb58c975. It adds a temporary
`tracking_roi:palm_refresh` session policy: fresh palm detection each frame,
mutually unique corresponding-palm matches below0.25 normalized RMS, otherwise
the prior tracking crop. Existing IDs and identity/overlap rejection remain.
It reuses fresh proposals rather than running the palm model twice, and records
actual refreshes separately from accepted hand observations. The patch also
retains its ambiguity/proposal-order regression and explicit policy metadata.

The three-scene experiment is rejected: the adverse-yaw pinch receives no refresh
during its near hold and loses its remaining native contact; the negative V
retains27 contact frames versus26, while central near coverage stays9/9. No
threshold is enlarged afterward. The patch is retained to reproduce that result,
not as a pending production feature or a new option in the current runtime.

At the exact base, use `git apply --check PATH_TO_PATCH`, then `git apply
PATH_TO_PATCH` and build with the existing native-vision feature. Use only the
camera/device-free `controller-sequence` experiment with the declared session
policy and retained synthetic inputs. `git apply -R PATH_TO_PATCH` reverses that
source experiment. Applying/building it alone does not install or activate input.
See [the investigation](../../../docs/research/roi-stability-and-tasks-2026-10.md).
