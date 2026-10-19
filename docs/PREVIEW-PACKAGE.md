# Source-bound Linux preview candidate assembly

`tools/package_preview.py` assembles an independently usable candidate archive
from a native CI binary manifest and prepared provider assets. It runs no binary,
model, camera or system input. It requires the matching Git checkout and Cargo's
existing dependency cache for offline license metadata. Python packages are not
installed and npm scripts are not executed.

```sh
python3 tools/package_preview.py \
  --candidate /absolute/native-candidate \
  --assets /absolute/prepared-provider-assets \
  --output /absolute/new-preview-candidate.tar.gz
```

The candidate's exact source SHA, native-vision feature, binary hash and ELF
architecture must agree. Git blobs from that revision supply the selected tools,
documentation, dependency declarations and packaging files; arbitrary working
files are excluded. The working Cargo declarations must match that revision.
Provider bytes are validated with the existing offline helper, only pinned
members are copied, and the actual copy is validated again. Binary copy/hash
is also rechecked. Existing/racing output paths are preserved through exclusive
single-archive publication. The same inputs produce byte-identical archives
with normalized ownership/timestamps/modes, including under a private umask.

The archive includes:

- `bin/wigglecontrol`, `binary-manifest.json` and source/architecture metadata.
- Existing manual configuration/calibration/launch/permission/removal helpers.
- Pinned SDK/WASM/model, metadata, Apache license and source lock.
- Original constant-color `packaging/warmup.jpg` and its hash/provenance/limits.
- Source documentation, START-HERE, MIT/Apache notices and locked Rust dependency notices.
- `preview.json` with the exact regular-file inventory, permissions and SHA256s.

No browser profile, personal calibration/configuration, recording, native ONNX
runtime/model or experimental reconstruction dependency is copied. Chromium,
systemd user services, camera/input access and compatible native system libraries
remain receiving dependencies. The build OS/libc is recorded, not a proved minimum
libc requirement or a claim that an arbitrary Linux installation is compatible.

Rust notices follow the selected target's reachable locked dependency graph,
including build dependencies conservatively. Original crate license/notice files
are retained. Three published crates omit standalone license texts; pinned
[supplements](../packaging/dependency-licenses.json) retain upstream originals at
each crate's own `.cargo_vcs_info.json` revision. The collector requires revision
and hash agreement and refuses undeclared/missing/unreviewed notices. This records
declared upstream terms and selected texts, not a new license grant.

The synthetic JPEG decodes to640×480 RGB(18,22,33), with no person, camera frame
or private metadata. It provides an explicit initial inference reference. It
does not establish detected-hand/finger-graph warmup or physical tracking. No MHR,
WiLoR or MANO dependency is introduced. The existing SDK/model licenses remain
separate from the original image's MIT terms.

## Verification and CI

After extraction into a new stable directory:

```sh
python3 tools/verify_preview.py
```

This validates every regular file/mode, rejects symlinks/special/unlisted files and
never executes the binary. Integrity is separate from download authenticity;
compare the archive's external checksum with the selected trusted delivery source.
Runtime mutable configuration/browser/recording data belongs outside this version
directory so later verification remains meaningful.

The native ARM64 and x86-64 CI builders retain the existing bare binary candidates
(including the unchanged ARM artifact name) and add full preview candidates with
external SHA256 sidecars. They assemble using real pinned assets and notices,
extract/verify, execute the native packaged configuration generator and prepare
a camera-only calibration launcher against packaged assets. That CI preparation
uses `/usr/bin/false` as an explicit never-invoked browser placeholder: no actual
browser/GPU, camera, inference or system input is tested by it. The full candidates
retain `receiving_qualified:false` and `release:false` with14-day CI retention.
They are not a tagged public prerelease or a substitute for physical acceptance.

Device-free fictional tests cover reproducible complete inventory, private-extra
exclusion, source/hash/architecture refusal, existing-output preservation,
corrupt assets/missing licenses, supplement revision binding and extracted
digest/mode/unlisted-file refusal. Actual current x86 metadata retains notices
for80 dependencies; this is a license-collection check, not a runtime benchmark.
See [LINUX-PREVIEW.md](LINUX-PREVIEW.md) for the remaining release requirements.
