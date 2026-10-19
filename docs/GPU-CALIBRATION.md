# Independent guided GPU calibration

The browser GPU worker feeds the existing Rust diagonal sampling, four-corner
review and profile store. Calibration is camera-only: it cannot create a virtual
mouse or keyboard. Its separate controls show instructions and target/observed
points without camera images or a skeleton. Receiving/human qualification is
still pending; preserve the accepted Pi profile when selecting a trial candidate.

## Preparation

Generate a fresh pair with [`prepare-preview-config`](PROVIDER-CONFIG.md#configuration-example)
or use a valid existing standalone provider configuration, pinned assets
and explicit synthetic warmup. Set session `pointer_output` to the selected single
display's dimensions. Select an absolute profile path with an existing private
parent. Existing profiles must validate, belong to the current user and match
those dimensions; preparation reads them without replacement.

```sh
wigglecontrol provider-calibrate-preflight \
  /absolute/private/session.json /absolute/private/settings.json \
  /absolute/provider-assets /absolute/synthetic-warmup.jpg \
  /absolute/private/calibration.json 18877

python3 tools/prepare_preview_launcher.py \
  --binary /absolute/wigglecontrol --browser /usr/bin/chromium \
  --session /absolute/private/session.json \
  --settings /absolute/private/settings.json \
  --assets /absolute/provider-assets --warmup /absolute/synthetic-warmup.jpg \
  --calibration-profile /absolute/private/calibration.json \
  --output /absolute/private/calibration-launcher \
  --port 18877 --name calibration
```

Calibration and `--seconds` are mutually exclusive. Follow the existing
[explicit user-unit operation](PREVIEW-LAUNCHER.md#user-installation-update-and-rollback)
using this bundle. Its source is `provider-calibrate` with the same arguments as
preflight; detector/controls use `/provider/worker` and `/provider/calibration`.
Preparation starts no units, browser, model, listener, sensor or input device.
No Matrix/integrators configuration is required. Before Start, coordinate camera
ownership and stop the selected mouse session if it is using that camera;
opening idle calibration does not interrupt another application.

## Human flow and lifecycle

Start is explicit. The worker prepares on the synthetic image, then the source
opens camera/private recording. Warmup points never calibrate. Each stage clock
waits for a source frame and acknowledgment that its instruction painted.

Each diagonal stage gives15 seconds to move and5 seconds to hold, reusing the
existing stable-sample/freshness rules and workspace margin. All four corners
then require fresh continuous dwell in order, each within20 seconds. Two diagonal
inputs alone cannot finish. Missing/out-of-frame/ambiguous/unclear observations
do not supply valid samples, and tracking loss/staleness interrupts corner dwell.

Completion or failure closes capture before the next choice. Save requires all
four corners, saved verification and confirmed camera closure; it adopts through
the existing profile store with a previous-profile backup. Retry discards this
attempt's draft/verification and returns to idle; a new Start is required. Cancel
closes capture and preserves the adopted profile. No action automatically starts
another acquisition.

Capture is bounded to150 seconds per attempt. With camera off, results can wait
up to ten minutes for Save/Retry/Cancel; expiry never adopts them. Closing controls
does not stop capture: reopen through `open.sh`, choose Cancel or use the scoped
`stop.sh`. Save/Cancel end the source; controls retain its final state.

Private recording is currently required for this experimental camera path; each
attempt creates its own recording. No microphone is used. After explicit Save,
use [`prepare-provider-config`](PROVIDER-CONFIG.md#import-a-selected-calibration-without-replacing-it)
for a new matched mouse configuration. Calibration never rewrites/starts an
existing mouse bundle. Native `calibrate-guided` remains a separate supported path.

## Qualification

Camera-free tests reuse actual sampling/review/store code: two diagonals continue
into four corners; Save waits for closure; previous profiles survive; no-hand
failure, manual Retry, capture limit, Cancel and lost-frame dwell are covered.
Run-bound HTTP actions, painted-step controls, retained terminal state and the
manual camera-only launcher have matching regressions.

The full native-feature suite passes246 tests, zero failures, with one existing
ignored benchmark; all-target Clippy, the control JS and seven launcher tests
pass. Actual VPS user-unit preparation/open reaches idle calibration with no
model preparation, camera/input, profile or recording creation. Cancel through
the real browser ends source/worker with exit0, terminal-display acknowledgment
and retained controls. Selected units/displays were stopped and installed test
links removed; private evidence remains retained.

The actual controls fit1920×1080 and1600×950 without page scrolling;375px reflows
vertically without horizontal overflow. SVG target/point rendering was inspected
with an explicitly labelled presentation fixture after the source ended. It is
not physical tracking evidence. The browser check used existing CDP after the
Playwright attachment failed; controls used software rendering in the second
fixture. GPU/model performance was not measured. An earlier bounded fixture
expired in idle with zero capture/input; its timeout is retained separately.

These checks do not establish physical calibration accuracy, live GPU performance
or human comfort. Receiving trials, complete install/update/removal and offered
ThinkPad qualification remain in [LINUX-PREVIEW.md](LINUX-PREVIEW.md), issue12/#13.
