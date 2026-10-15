# Worker transport qualification

The experimental `provider-fixture` command connects a finite JPEG source, the
existing loopback feedback server, the pinned browser GPU worker and actual Rust
tracking/meaning. It opens no camera or virtual input device. It is not a live
camera backend or accepted receiving runtime.

```sh
wigglecontrol provider-fixture SEQUENCE.json SESSION.json SETTINGS.json ASSET_ROOT PORT
```

Build with `native-vision`; native ONNX inference is not initialized by this
command. Existing session/controller settings must match, select a single
estimate and explicit gesture confidence0.5, and omit visual macros. Retained
3D/2D pinch interpretation, workspace, manual modes, open rearm and Stop remain
the existing controller's responsibility. No gesture threshold/freshness is
silently changed by transport.

The sequence uses the existing version1 image manifest, with relative regular
640×480 JPEG files, at most2MiB each, and source offsets covering at most90seconds.
Prepare compression before the run and retain original/JPEG hashes plus independent
geometry. Compression changes pixels, so actual worker inference must run afresh.
After model-ready acknowledgment the finite producer publishes at those offsets
using the host's actual monotonic timestamp after reading each encoded frame.
This measures **encoded-frame publication→meaning**, not camera exposure→OS event.

Open `/provider/view` in the explicitly owned receiving browser. It reuses the
native feedback/mode/Stop page and labels the source synthetic. Model/SDK assets,
the classic worker and client share this loopback origin. Preparation is bounded
30seconds; model inference has the existing30second worker-request bound; the
source has a finite completion bound and ends with a release. Existing native
camera commands and their human-ready/recording requirements remain separate.

`ASSET_ROOT/provider-assets.json` contains `package_version:"1.1.1"` and a `files`
map of relative names to SHA256. Include the qualified `hand_landmarker.task`,
`package/vision_bundle.mjs` and selected files directly under `package/wasm/`.
Preflight verifies each bounded regular file with the existing Linux `sha256sum`
tool. Explicit receiving-local public-asset symlinks are supported; the server
serves only the verified allowlist. Model/SDK binaries stay outside Git with their
licenses and original manifest. The model digest also gets checked in the browser.

Rust owns acquisition timestamps, dimensions and a monotonically increasing lease
number. `/provider/frame` leases the latest published JPEG; only one is in flight.
The client returns `{sequence,hands}` to `/provider/result`, with raw pixel/world
points and a separate handedness score. It cannot submit a replacement source or
evaluation timestamp. Result acceptance consumes the exact lease once and waits
for the actual meaning receipt. Old leases expire; the source continues replacing
its latest slot, and skipped/expired/processed counts remain separate.

The main loop ticks tracking/meaning independently of worker execution. Mode,
hand selection and resume use existing control-time fences, so a pre-control
image cannot reactivate input. The optional HTTP extension requires the existing
Host/same-origin checks; headers stay bounded4KiB and provider bodies16KiB.
An absolute read deadline prevents partial bodies from indefinitely occupying
the listener. Existing preview routes remain available without this extension.

JSONL output separates source publication, accepted frames, intent transitions,
expired leases, manual controls and final release, with host-monotonic timestamps.
Presence0.5 stays explicitly a configured SDK threshold floor. The preview shows
encoded-frame→meaning duration separately from the aging last image; it does not
label that number as model-only inference. A worker-close acknowledgment is
recorded separately from finite server/process termination.

Source tests cover one-in-flight/latest-slot accounting, immutable host-time
association, wrong/repeated/expired results, actual receipt acknowledgment,
body/header/origin bounds, partial-body expiry and existing preview behavior.
Actual same-host ARM64 worker/transport timing, quality across skipped frames,
delayed-result held-release behavior and visible1080p receiving qualification
must be recorded after execution. Camera integration, broader negatives/crossings,
image-depth/macros, participants and human/application acceptance remain open.
