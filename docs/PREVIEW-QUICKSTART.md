# Connected preview setup from source

This is the connected preparation flow, not a published release or a clean-host
qualification claim. A [full candidate archive](PREVIEW-PACKAGE.md) supplies the
binary/tools/pinned assets and original synthetic warmup; start with its
START-HERE and verifier. From its extracted root, select:

```sh
preview_binary="$PWD/bin/wigglecontrol"
preview_assets="$PWD/provider-assets"
preview_warmup="$PWD/packaging/warmup.jpg"
```

Alternatively, use a source checkout with an existing compatible Linux build,
Chromium, prepared pinned assets and an explicitly selected synthetic warmup JPEG.
The [asset guide](PROVIDER-ASSETS.md) prepares/verifies the SDK/model without npm
scripts. Tagged delivery and clean-host/ThinkPad qualification remain open.
Preserve an existing installation/profile; use new private directories.

Run helpers from the extracted package or source checkout. The receiving desktop needs its ordinary
user systemd manager, Chromium/GPU support, Python3.12+, normal camera permissions
and a physical input fallback. [Input preparation](INSTALL.md#independent-preview-input-preparation)
also uses ACL utilities, kmod, udev and `flock`. None of the preparation commands
below acquires a camera. Opening controls remains idle until a human Start.

## Choose resources and prepare calibration

Select the real camera and dimensions of one display. The numbers and paths here
are examples; they are not another user's calibration or hardware discovery.
Use stable absolute artifact locations and unused names/loopback ports.

```sh
# Source-checkout users choose equivalent absolute artifact paths first.
preview_browser=/usr/bin/chromium
preview_root="$HOME/.local/share/wigglecontrol/preview-01"
mkdir -m 700 -p "$preview_root"

"$preview_binary" prepare-preview-config \
  /dev/video0 1920 1080 "$preview_root/recordings" "$preview_root/uncalibrated"

python3 tools/prepare_preview_launcher.py \
  --binary "$preview_binary" --browser "$preview_browser" \
  --session "$preview_root/uncalibrated/session.json" \
  --settings "$preview_root/uncalibrated/settings.json" \
  --assets "$preview_assets" --warmup "$preview_warmup" \
  --calibration-profile "$preview_root/calibration.json" \
  --output "$preview_root/calibration-launcher" --port 18877 --name calibration

python3 tools/manage_preview_launcher.py install "$preview_root/calibration-launcher"
python3 tools/manage_preview_launcher.py open calibration
```

Choose Start only when the camera is available and you are ready. Calibration
cannot emit system input. Follow both diagonal instructions and all four corners.
Save becomes available only after verification and observed camera closure;
Retry requires another Start, and Cancel preserves the saved profile. See
[calibration controls and limits](GPU-CALIBRATION.md). After Save:

```sh
python3 tools/manage_preview_launcher.py stop calibration
"$preview_binary" prepare-provider-config \
  "$preview_root/uncalibrated/session.json" "$preview_root/uncalibrated/settings.json" \
  "$preview_root/calibration.json" "$preview_root/calibrated"
```

The copied calibration maps both settings files consistently. Original templates
and the saved profile remain intact. Import validates the selected profile; it
does not perform a second physical verification or certify another display.

## Prepare and use the mouse

The receiving administrator prepares the independently scoped input grant once,
using the actual existing account rather than a copied host username:

```sh
sudo tools/prepare_input.sh --standalone RECEIVING_OPERATOR
```

Then return to the ordinary desktop user:

```sh
python3 tools/prepare_preview_launcher.py \
  --binary "$preview_binary" --browser "$preview_browser" \
  --session "$preview_root/calibrated/session.json" \
  --settings "$preview_root/calibrated/settings.json" \
  --assets "$preview_assets" --warmup "$preview_warmup" \
  --seconds 0 --output "$preview_root/mouse-launcher" --port 18876 --name mouse

python3 tools/manage_preview_launcher.py install "$preview_root/mouse-launcher"
python3 tools/manage_preview_launcher.py open mouse
```

`--seconds 0` explicitly selects use until Stop. A new finite trial can instead
select1..90 seconds; idle preparation does not consume its acquisition clock.
Only Start prepares the detector and opens camera/recording/input. Closing the
control window does not stop the independent detector; reopen with `open mouse`.
Use the page's Stop or the following scoped command to end it:

```sh
python3 tools/manage_preview_launcher.py stop mouse
```

The generated defaults select thumb/index left click and thumb/middle right click,
held ring/little-finger [detail](DETAIL.md), deliberate [Escape](ESCAPE.md), and
closed-fist two-axis [grab scroll](SCROLL.md). Positive moving-open release can
retain bounded scroll inertia; opening at rest does not. These are experimental
bindings with first-use evidence, not completed accuracy/comfort acceptance.
No camera/skeleton view is needed for mouse operation. Physical input and Stop
remain available independently.

## Recording, updates and removal

The current experimental camera path requires explicit private recording, for
both calibration and mouse use. Start records camera JPEGs/timestamps and retained
controller evidence under the chosen directory; no microphone or upload is used.
Use a private storage location with capacity for the selected session. Continuous
sessions rotate512MiB segments; rotation is not a total-retention limit. Stop
closes capture, but recordings remain until intentionally removed. Predictions
in journals are not human intent labels. Config preparation itself records nothing.

Prepare every update as a new bundle with the same installation name and retained
old artifact/configuration paths. Do not edit hash-bound version inputs in place.
The [user manager](PREVIEW-LAUNCHER.md#user-installation-update-and-rollback) installs
the next bundle or rolls back to the previous one, stopping only that installation
and leaving it off until explicit open. Uninstall both selected user bundles:

```sh
python3 tools/manage_preview_launcher.py uninstall mouse
python3 tools/manage_preview_launcher.py uninstall calibration
sudo tools/rollback.sh --standalone
```

Confirm their camera/input processes are terminal before reversing the grant.
Permission rollback preserves independently changed files/ACL for reconciliation;
an error is not completed removal. Units and input registration are separate from
retained versions, profiles, browser state and private recordings. No command
here purges that data, starts capture at boot or requires Matrix/integrators.

The pending [release acceptance](LINUX-PREVIEW.md#release-acceptance) includes real
calibration/desktop tasks, post-boot input readiness, clean standalone installation,
ThinkPad, measured resources/comfort and source-bound tagged artifacts. A passed
camera-free preparation chain does not establish those outcomes.
