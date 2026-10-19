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

Import an already adopted calibration with
[`prepare-provider-config`](PROVIDER-CONFIG.md#import-a-selected-calibration-without-replacing-it)
when preparing a new configuration. Select its resulting session/controller
files below; this prevents independently edited workspace maps from drifting.

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

For [camera-only guided calibration](GPU-CALIBRATION.md), select
`--calibration-profile /absolute/private/calibration.json` instead of `--seconds`.
That bundle opens the calibration controls and never passes `--inject`; the
profile changes only after four-corner verification, camera closure and Save.

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
The generated wrappers require the endpoint's native run ID to match the selected
source unit's current MainPID before opening either browser. A different provider
on an occupied port is refused. Existing wrapper-service launch routes remain
compatible unless they explicitly opt into this direct-process requirement.
Only the human's Start requests detector preparation/acquisition. Closing controls
or feedback does not close the detector. Reopening controls rebinds to the current
source; the existing helper does not restart an already running source/worker.
`stop.sh` stops only the selected three units. Each has `Restart=no` and
`KillMode=control-group`; the detector binds to the source's lifetime. Controls
remain open after source completion to retain its final result, without inference
or images; reopening through `open.sh` binds a fresh page to the current source.
There is no boot target, automatic restart, remote-debugging port or disabled browser
sandbox. No Matrix or daimon-integrators service/configuration is used.

The control page requests normal Stop with `graceful:true` on the existing
run-bound `/provider/stop` endpoint. Worker error cancellation remains distinct;
cleanup/join/writer errors still fail the session. Cheap configuration exposes
`terminal_ready` only after final publication. Controls wait for observed camera/
input closure and that publication, paint the final result, then acknowledge the
existing terminal-display endpoint. A failed receipt or later network failure
does not overwrite already observed closure. Old sources without `terminal_ready`
retain their prior compatibility behavior; new sources do not equate an early
EOF sample with completed final publication.

Before changing/removing linked units, end only this installation with `stop.sh`
and confirm its source/worker are terminal. Remove the three selected links with
`systemctl --user disable` using their exact names from `bundle.json`, then reload
the user manager. Keep artifacts, accepted configuration/calibration, browser
profiles and private recordings for intentional retention/rollback; this tool
does not delete them or claim complete uninstall/rollback support.

## Evidence and limits —2026-10-19

Six offline tests cover finite/until-Stop argument selection, rejection of
nonmanual/active preflight, preservation of existing output, private preparation,
service dependency/cleanup/no-start contracts, systemd/shell argument escaping
and actual launcher refusal before browser launch on a different source PID.
Actual pinned assets and the Rust GPU configuration without ONNX paths pass real
preflight. The generated units pass the installed `systemd-analyze --user verify`,
including spaces, dollar and percent characters in the bundle path; wrapper
scripts pass `sh -n`. That initial preparation linked/started no unit and ran no
browser, camera, inference or input. It used a synthetic blank JPEG only for
dimensional/preflight validation; it does not qualify hand-model warmup.

Subsequent isolated VPS qualification linked uniquely named test units and used
an owned Chromium/Xvfb display under the ordinary user. Actual `open.sh` reached
idle controls with camera/input off and no preparation. Closing/reopening controls
preserved the running source/worker and run ID. The first fixture reproduced a
normal Stop reported as worker cancellation and controls disappearing with the
source; these findings motivated the graceful Stop and dependency correction.

The corrected fixture's actual Stop leaves source/worker inactive with exit0,
zero capture and no input start. Controls remain open with the final camera-off/
mouse-off result and a successful terminal-display receipt after the listener
exits.239 native-feature tests, all-target Clippy, the control JS regressions and
six launcher tests pass. Fixtures, selected links and displays were removed.
The retained control browser eventually reached the fixture's300-second resource
limit; this test-only limit is absent from generated product units. No Start was
clicked, model initialized, camera opened or input emitted in either fixture.

This is a launcher-preparation component, not the completed Linux preview.
Idle graphical lifecycle is now qualified on the VPS, not on the ThinkPad or Pi.
Live detector/graphics performance, setup/guided calibration, pinned release
assembly, input permissions, clean-host installation,
update/removal/rollback and human acceptance remain in issue12/#13 and
[LINUX-PREVIEW.md](LINUX-PREVIEW.md). Existing Pi services/calibration/trials are
unchanged. See [PROVIDER-CONFIG.md](PROVIDER-CONFIG.md) and
[PROVIDER-ASSETS.md](PROVIDER-ASSETS.md) for the other completed source components.
