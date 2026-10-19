# Reproducible assets for the GPU preview

The selected browser GPU detector needs the Tasks Vision SDK/WASM and Hand
Landmarker task bundle. Their exact archive/files are pinned in
[provider-assets.lock.json](../packaging/provider-assets.lock.json).
The [preparation tool](../tools/fetch_provider_assets.py) downloads/validates data
without npm installation/scripts, inference, sensors, system input or root.
This is an asset-packaging component, not the complete standalone installer.

From the checkout, select explicit output and cache locations with sufficient
space on their owning filesystem:

```sh
python3 tools/fetch_provider_assets.py \
  --output target/provider-assets \
  --cache target/provider-asset-cache
```

The prepared files take about47MiB; the reusable download cache about20MiB.
The SDK1.1.1 archive is checked with its pinned SHA512 and each selected member
with its own SHA256/size. The Hand Landmarker v1 bundle is checked independently.
Only selected regular members are copied; archive paths/symlinks cannot write
elsewhere. The source does not run package lifecycle code or accept a latest URL.

The output includes:

- `provider-assets.json`: the existing Rust transport's eight runtime assets.
- Unchanged SDK module and six WASM/loader variants, plus the task bundle.
- Original package metadata and README, Apache2 license text and source pointers.
- `asset-bundle.json`: full output checksums and the exact source-lock digest.

The [SDK metadata](https://registry.npmjs.org/@mediapipe/tasks-vision/1.1.1)
declares Apache2; the [official hand-tracking model card](https://storage.googleapis.com/mediapipe-assets/Model%20Card%20Hand%20Tracking%20(Lite_Full)%20with%20Fairness%20Oct%202021.pdf)
identifies the same license. These artifacts do not include experimental
reconstruction rigs or personal recordings/calibration.

After preparation, verify/reuse the bundle without network or even a retained
cache by adding `--offline` to the command. A new offline destination requires
verified cached archive/model bytes. Download/validation/extraction finish before
publishing the destination. Repeating against the same complete verified bundle
is idempotent. A differing existing directory is preserved and rejected; choose
a new destination rather than replacing a live or unfinished bundle.

## Qualification —2026-10-19

The pinned npm archive was independently verified against registry SHA512; all
runtime SDK hashes match the existing qualified provider cache. Actual online
preparation, separate offline preparation and repeated cache-free verified reuse
passed. The real Rust manual-camera preflight accepted the new output without
acquisition, model initialization or virtual input. This validates packaging and
pre-acquisition compatibility, not inference speed or a new receiving installation.

Device-free fictional-archive tests cover offline/no-network operation, exact
runtime allowlist and license/metadata retention, idempotent reuse, corrupt output
preservation, missing/corrupt cache rejection and unsafe selected archive members.
No partial destination is published on those failures. Run:

```sh
python3 tools/test_fetch_provider_assets.py
```

The complete preview still needs independent configuration/guided calibration,
normal-user launch/permissions, runtime browser/environment qualification,
installation/update/removal/rollback and physical acceptance/prerelease. These
remain in [issue12](https://github.com/AlterMundi/WiggleControl/issues/12) and
[LINUX-PREVIEW.md](LINUX-PREVIEW.md). No Pi configuration changes, package install,
model upgrade or new capture are implied by preparing these pinned assets.
