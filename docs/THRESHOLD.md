# Threshold usage, building and limitations

Threshold extends the [Hew Edge Treatment development branch](https://github.com/EZOGuitars/Hew3dEdging/tree/feature/edge-treatment). Its private release destination is [EZOGuitars/HewEdgingThreshold](https://github.com/EZOGuitars/HewEdgingThreshold). The release history descends from upstream [hew3d/hew](https://github.com/hew3d/hew); it is not an upstream-endorsed release.

## Supported operations

Select Edge Treatment. For a complete polygonal or circular cap, choose **Select face perimeter**, then click its planar cap. Radius and equal-distance chamfer input accept explicit metric/imperial units; otherwise Hew's current unit preference applies. Numeric input remains independent of slider steps and display rounding. The slider uses a conservative native geometric bound with a validated endpoint; it is not an exact maximal radius. Typed values beyond it still receive normal validation.

Supported carriers are connected, outward-oriented, closed convex solids with a complete eligible cap and perpendicular adjacent walls. Polygon corners may have different interior angles; cap/wall dihedrals remain perpendicular. Polygon junctions are G0 miters. Existing rectangular edge subsets retain their miter/spherical-junction behavior. Single straight edges retain their original restrictions.

Circular caps require consistent existing Cylinder ancestry, an axis perpendicular to the cap, and a uniformly spaced faceted ring of 8–256 sides. Circular chamfer distance is radial setback on the ideal cap and axial depth on the cylinder; its carrier remains faceted. Circular roundovers use a sampled outer quarter-torus and a whole-STL-triangle error certificate. This is not exact analytic surface reconstruction. The original polygonal cylinder is the containment envelope; no automatic source refinement adds material. If the source circumference is too coarse to leave the required approximation budget, draw a finer source circle or increase chord error. Roundovers use 4–128 meridian segments and at most 4096 clipping planes; overly fine requests refuse rather than clamp quality.

A second treatment can use an untouched opposite cap band on the already modified convex carrier, including different dimensions or a different operation. The engine checks actual remaining support-plane clearance. This does not support re-blending an existing rounded cap, overlapping cuts, arbitrary modified neighboring faces or a general parametric feature tree. Save/load preserves baked geometry and cylinder attribution; undo/redo uses Hew's normal session history, and recordings replay the operations.

## Preview behavior

After a short coalescing interval, a worker loads an immutable source snapshot, resolves stable object identity and exact edge endpoints, evaluates the operation and returns disposable preview buffers. Obsolete workers terminate. Origin document epoch, history generation, exact source content and input revisions prevent stale publication/application. Preview does not mutate the live solid, history or recording. Cancel or Escape discards preview resources.

Apply deliberately rebuilds and validates a fresh candidate in the live Scene before committing one ordinary document action. It is synchronous and may pause for expensive operations; worker preview scheduling does not make that computation faster. When workers cannot start, the panel switches to an explicit synchronous Preview fallback. Large-document snapshot/hash cost and main-thread Apply remain performance limitations. See the measured verification report rather than assuming universal real-time performance.

## Fedora 45 x86_64 source build

Use the prerequisite installation steps in [the existing Fedora alpha handoff](dev/FEDORA45_EDGE_ALPHA.md): pinned Rust, Node and pnpm, wasm-pack, GTK3/WebKitGTK 4.1 development packages and Tauri's other system libraries. Native desktop execution was not validated on the cloud host because those GTK/WebKit development libraries are absent. This release provides reproducible source, not a certified Fedora binary.

For a fresh checkout:

```bash
gh repo clone EZOGuitars/HewEdgingThreshold ~/src/hew-threshold -- --branch threshold-v0.1.0
cd ~/src/hew-threshold
```

For the **existing alpha checkout**, first ensure `git status --short` is clean and preserve any local changes. Add the release remote once, retaining the existing development and upstream remotes:

```bash
cd ~/src/hew-edge-treatment-alpha
git status --short
git remote -v
git remote add threshold https://github.com/EZOGuitars/HewEdgingThreshold.git
git fetch threshold --tags
git switch --detach threshold-v0.1.0
git rev-parse HEAD
```

If `threshold` already exists, verify its URL instead of replacing it. Compare HEAD to the release receipt. Do not reset or force-switch over local work.

Build and run from either checkout, preserving caches:

```bash
pnpm install --frozen-lockfile
wasm-pack build crates/wasm-api --target nodejs --out-dir ../../tools/replay-runner/pkg-node
node --test tools/replay-runner/edge-treatment-smoke.mjs
pnpm --dir app test src/tools/EdgeTreatmentCircular.wasm.test.ts src/tools/EdgeTreatmentPerimeter.wasm.test.ts --maxWorkers=1
pnpm --dir shells/tauri exec node scripts/stage-cli-sidecar.mjs
export HEW_BETA_SHA="$(git rev-parse --short=12 HEAD)"
pnpm --dir shells/tauri exec tauri build --config src-tauri/tauri.edge-alpha.conf.json --no-bundle
ALPHA_ROOT="$PWD"
ALPHA_STATE="$HOME/.local/state/hew-edge-treatment-alpha"
mkdir -p "$ALPHA_STATE"/{config,data,cache,state,logs,models}
set -o pipefail
env XDG_CONFIG_HOME="$ALPHA_STATE/config" \
    XDG_DATA_HOME="$ALPHA_STATE/data" \
    XDG_CACHE_HOME="$ALPHA_STATE/cache" \
    XDG_STATE_HOME="$ALPHA_STATE/state" RUST_LOG=info \
    "$ALPHA_ROOT/shells/tauri/src-tauri/target/release/hew" \
    2>&1 | tee "$ALPHA_STATE/logs/console.log"
```

Stop after a failed command. The executable path assumes the established default shell Cargo target directory; custom target directories must use their actual Cargo-reported artifact. No-bundle avoids installing an RPM or replacing Hew. Never delete alpha XDG state to speed a build. Use model copies in the isolated `models` directory; explicit file-open/save paths remain the user's responsibility.

## Reproducing validation

```bash
cargo test -p kernel --test edge_treatment_specs --test multi_edge_treatment_specs --test polygon_edge_treatment_specs --test perimeter_edge_treatment_specs --test circular_edge_treatment_specs --test document_edge_treatment_specs
cargo test -p wasm-api --lib
cargo test -p mesh-export
cargo fmt --all --check
cargo clippy -p kernel -p mesh-export -p wasm-api --all-targets -- -D warnings
pnpm --dir app typecheck
pnpm --dir app typecheck:test
pnpm --dir app test --maxWorkers=1
pnpm --dir app lint
pnpm --dir app i18n:check
pnpm --dir app build
```

Generate the Node WASM package first as above. Browser tests are in `app/e2e/edge-treatment.spec.ts`; run under Hew's existing Playwright/dev-server configuration with a supported installed browser. Worker transport tests additionally validate the production worker artifact. STL stores no physical units; Hew exports millimeters. Supply 0.001 meters per STL unit when importing these files into an external unit-aware workflow. Do not infer physical accuracy merely from correct unit conversion.

## Unsupported cases and reporting bugs

Mixed straight/curved perimeters, nonuniform/elliptic attributed curves, arbitrary concave solids, attached boss/base globally nonconvex objects, arbitrary NURBS filleting, G2 continuity, variable radii, guitar-body contours and arbitrary sequential edits are not certified or claimed. Failures leave the original model unchanged. Export may refuse f32 precision loss instead of producing an unsafe file.

Report the exact Git SHA, beta stamp, Fedora/session/GPU/WebKitGTK versions, selected boundary, dimensions and units, chord error, native refusal text and reproduction steps. Review diagnostic bundles and model copies before sharing. Do not automatically upload private project models. Follow [CONTRIBUTING.md](../CONTRIBUTING.md) for upstream work; this release opens no upstream PR and merges/rebases no upstream branch.
