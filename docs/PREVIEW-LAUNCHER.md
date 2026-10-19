# Independent manual launcher preparation

`tools/prepare_preview_launcher.py` prepares a normal-user systemd bundle for
the existing Rust source, separate headless Chromium detector and optional mouse
controls. It reuses `mouse_launcher.py`; it does not implement a second detector
or session protocol. Python handles preparation/launch only, outside perception
and input delivery.

Preparation runs the selected binary's real no-acquisition preflight, then
publishes a new private directory. It does not install/link/enable/start units,
launch Chromium, initialize inference, open sensors or create input devices.
An existing output is preserved and rejected. Choose a stable final location;
generated launch paths refer to it and to the explicitly selected artifacts.

From the checkout, with existing compatible artifacts/configuration:

```sh
python3 tools/prepare_preview_launcher.py \
  --binary /absolute/path/wigglecontrol \
  --browser /usr/bin/chromium \
  --session /absolute/private/session.json \
  --settings /absolute/private/settings.json \
  --assets /absolute/path/prepared-provider-assets \
  --warmup /absolute/path/public-synthetic-hand.jpg \
  --output /absolute/private/preview-launcher \
  --port 18876 --seconds 90 --name mouse
```

Select `--seconds 1..90` for a finite trial, or explicit `--seconds 0` for use
until Stop. Both wait for manual Start; idle waiting does not consume the finite
acquisition clock. New physical trials still require coordinated human readiness
and the private recording-enabled configuration. A launcher is not camera/input
permission or acceptance of its selected calibration.

The bundle includes three uniquely named user units, `open.sh`/`stop.sh`, the
existing launch helper and `bundle.json` with checksums, selected bindings and
preflight. Artifacts/configuration/assets remain at their explicit paths; they
are not copied into a distributable release by this tool.

## Explicit installation and operation

On the receiving user's graphical session, after checking that the selected name
and loopback port do not conflict with another installation:

```sh
preview_bundle=/absolute/private/preview-launcher
systemctl --user link "$preview_bundle"/units/wigglecontrol-preview-mouse-*.service
systemctl --user daemon-reload
"$preview_bundle/open.sh"
```

These are explicit installation/launch actions, separate from preparation.
Do not use `--force` to replace another installation. The user's existing systemd
manager must have the receiving desktop's display environment; do not copy another
host's Wayland socket name. Browser/graphics, normal-user camera/uinput permissions
and desktop effects require receiving qualification before actual use.

Opening starts an idle source and independent detector, then opens controls.
Only the human's Start requests detector preparation/acquisition. Closing controls
or feedback does not close the detector. Reopening controls rebinds to the current
source; the existing helper does not restart an already running source/worker.
`stop.sh` stops only the selected three units. Each has `Restart=no` and
`KillMode=control-group`; worker/control bind to the source's lifetime. There is
no boot target, automatic restart, remote-debugging port or disabled browser
sandbox. No Matrix or daimon-integrators service/configuration is used.

Before changing/removing linked units, end only this installation with `stop.sh`
and confirm its source/worker are terminal. Remove the three selected links with
`systemctl --user disable` using their exact names from `bundle.json`, then reload
the user manager. Keep artifacts, accepted configuration/calibration, browser
profiles and private recordings for intentional retention/rollback; this tool
does not delete them or claim complete uninstall/rollback support.

## Evidence and limits —2026-10-19

Four offline tests cover finite/until-Stop argument selection, rejection of
nonmanual/active preflight, preservation of existing output, private preparation,
service dependency/cleanup/no-start contracts and systemd/shell argument escaping.
Actual pinned assets and the Rust GPU configuration without ONNX paths pass real
preflight. The generated units pass the installed `systemd-analyze --user verify`,
including spaces, dollar and percent characters in the bundle path; wrapper
scripts pass `sh -n`. No unit was linked/started and no browser, camera, inference
or input ran. The test used a synthetic blank JPEG only for dimensional/preflight
validation; it does not qualify hand-model warmup.

This is a launcher-preparation component, not the completed Linux preview.
End-to-end graphical/browser behavior of this independent bundle, setup/guided
calibration, pinned release assembly, input permissions, clean-host installation,
update/removal/rollback and human acceptance remain in issue12/#13 and
[LINUX-PREVIEW.md](LINUX-PREVIEW.md). Existing Pi services/calibration/trials are
unchanged. See [PROVIDER-CONFIG.md](PROVIDER-CONFIG.md) and
[PROVIDER-ASSETS.md](PROVIDER-ASSETS.md) for the other completed source components.
