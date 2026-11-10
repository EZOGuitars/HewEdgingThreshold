# Unified Edge Treatment foundation — implementation checkpoints

## Delivery scope

This is an incremental Phase 3 foundation milestone, not completion of Operation Ultimate Chickening. The recoverable Phase 2 baseline is `ebb06e4f4a049a9a0c08f7d9acbe09d5c5482816`, retained by local branch `baseline/edge-treatment-phase2-ebb06e4`. The approved architecture/review documents were preserved in commit `6192b64`; the native cap-band milestone is `346244f`; controls/integration and expanded acceptance tests are committed as `09c450e`.

All work remains on `feature/edge-treatment`. Publishing is explicitly withheld by the new authorization: no push, merge/rebase, upstream PR, installed-app change or dependency change is part of this delivery. The final receipt must distinguish local commits from remote availability.

## Checkpoint status

| Checkpoint | Implemented | Remaining acceptance |
| --- | --- | --- |
| A — common contracts and precision controls | Read-only native range probe; conservative geometric bounds with an independently prepared/certified endpoint; physical-meter slider writes; editable decimals independent of display precision/slider steps; range cache scoped to selection/mode/chord; existing accessible parameter help; generic optional tool invalidation/modifier hooks. | Exact maximal bounds are not claimed. A failed endpoint certification suppresses the slider instead of inventing a range. Smaller requests still require full numerical/export validation. Heavy synchronous jobs are not interruptible. |
| B — polygonal perimeters | Complete upper/lower cap perimeters of convex planar prisms, including triangles, asymmetric trapezoids, regular/irregular hexagons, octagons and other convex polygons. Uses source normals, G0 miter junctions and the existing indexed half-space boundary builder. | Partial nonrectangular perimeter selections, arbitrary edge dihedrals, collinear cap runs and nonconvex bodies remain unsupported. Cap selection is limited to 64 edges. |
| C — sequential treatments | A previously modified convex carrier can receive a treatment on another complete cap with an untouched perpendicular-wall band. Different upper/lower dimensions, roundover→roundover, chamfer→roundover and roundover→chamfer use the same candidate/commit/history interface. No feature tree. | This is not unrestricted successive blending: overlapping bands, re-treating the already blended cap, altered/nonperpendicular neighboring walls and analytic attributed sources remain gated. |
| D — curved boundaries | Existing circle/cylinder representation investigated; genuine attributed boundaries explicitly refuse with `UnsupportedCurvedBoundary` instead of being passed off as polygonal treatments. | Cylinder upper/lower and mixed straight/curved treatments are NOT implemented or accepted. Need a curved evaluator, source-envelope policy, finite toroidal patch certification and preserved ancestry through sequential results. |
| E — integrated acceptance | Native, actual-WASM, browser and regression checks are recorded below. | Complete sprint acceptance remains blocked by D and fully responsive heavy preview scheduling. Fedora desktop certification must be performed on Fedora. |

The original polygonal-boss regression is unchanged: globally nonconvex joined base/boss objects still refuse. A standalone polygon's complete cap perimeter now succeeds, while its individual obliquely terminated edges retain the old safe refusal. Do not conflate the two selection/geometry contracts.

## Geometry and validation decisions

`crates/kernel/src/edge_treatment/perimeter.rs` is the only new geometry module. `CapBand::resolve` recognizes the exact selected face perimeter and perpendicular adjacent walls. It preserves the existing convex source validator. Offset-edge collapse events and all unrelated supporting planes bound the untouched cap band. An existing upper treatment therefore restricts a later lower treatment by actual geometry, without requiring the object to be a pristine box.

`evaluate` creates chamfer or quarter-arc chord constraints from cap/wall normals. It reuses `multi::intersect_boundary`; existing arbitrary box-edge subsets and spherical three-edge corners remain on their Phase 2 evaluator. Straight single-edge profile/extrusion/explicit subtraction is unchanged. Follow Me and shared Boolean algorithms are unchanged.

For cap polygon perimeter P and K = sum cot(interior-angle/2), inset area loss is P*s−K*s² before the first offset event. Each faceted profile interval integrates this quadratic exactly. The expected removal is independent of the constructed mesh's signed-volume measurement. Candidate construction retains original support planes, validates the indexed watertight boundary, verifies containment and compares removed volume using existing named budgets. All cuts lie in the resolved cap band. The tests additionally inspect actual exported STL and retain vertices outside the treatment band. Topology IDs are regenerated, not promised stable; replay addresses exact source geometry as before.

Roundovers retain 4–128 quarter-arc segments and the existing 4096-plane budget. No tolerance is silently relaxed. Temporary cylinder references feed the unchanged actual-f32 STL certificate; the result is still a faceted carrier, not a new persistent analytic surface claim.

`treatment_dimension_bound` returns one quarter of a geometry-derived clearance/collapse limit (single edge, supported cap band, or existing box subset). This is a deliberate conservative margin, not an arbitrary model-size UI cap or an exact supremum. `Scene::edge_treatment_limits_json` prepares and certifies that endpoint without minting a handle, replacing the Scene slot, changing history or recording a command. It does not certify every floating-point value in the interval. Near-tolerance numerical failures and export quantization still require per-request validation. Manual input may exceed the conservative slider bound and is evaluated normally.

## UI and performance decisions

`EdgeTreatmentTool.ts` owns canonical meters, selection, limit caching and preview resources. The panel remains in `EdgeTreatmentPanel.tsx`; geometry bounds do not live in React. Unit changes update editable decimal text without regenerating unchanged physical geometry. Sliders pass numbers directly to `setDimensionMeters`; text input is never snapped to slider increments. Hew currently exposes shared unit preferences rather than a per-document unit API; this implementation reuses those existing preferences and does not invent a persistence schema.

Normal edits retain trailing 180 ms coalescing and invalidate Apply immediately. A successful evaluation taking over 100 ms switches that tool session to explicit Preview for subsequent edits, with a visible explanation. This prevents repeatedly scheduling known expensive work while dragging. It does not make the first expensive evaluation cancellable, move it off the main thread, or satisfy the full asynchronous-preview goal. Unit-only switching performs no geometry evaluation. A resumable evaluator or isolated worker with source/revision ownership remains necessary before claiming responsive live previews for all operations.

Node/WASM measurements on this development host (2 warmups, 5 retained samples; prepare/certification, mesh copy and discard; no GPU or Apply):

| Selection, R/chord mm | Median ms | Maximum ms |
| --- | ---: | ---: |
| One edge, 2 / 0.05 | 3.22 | 3.49 |
| Three incident edges, 2 / 0.05 | 1.75 | 1.88 |
| Cap perimeter, 2 / 0.05 | 0.92 | 1.16 |
| All 12, 2 / 0.05 | 10.22 | 11.08 |
| All 12, 6.35 / 0.01 | 876.30 | 900.35 |

The fine case has 20 quarter-arc segments and 6,732 triangles. At chord 0.005 mm it still safely returns `ApproximationTooFine`. These are development-host samples, not Fedora/browser latency guarantees or p95 measurements. Range-query cost is additional and is cached, not included in this table.

## Modularity and upstream compatibility

Geometry stays in the dedicated feature modules; document.rs, mesh-export, topology, serialization and Follow Me receive no new production changes in this milestone. The WASM feature adapter adds a pure query and shares candidate certification internally. Only `Tool` optional lifecycle methods and their Viewport forwarding add shared integration: other tools need not implement them. Existing selection, DocChange, snapshots, renderer, history, persistence and export remain in use. Shared STL hardening stays untouched and independently reviewable.

Official upstream branches inspected: `main`, `feature/plugin-system`, `site-live`, `v11/packaging`. Main remains `f23fc98bd79cb24b617f93ac96ad0601fe4a3c42`. No True Curves feature branch was available; the inspected recent pull-request listing contained no matching curve/fillet/chamfer/roundover PR. No merge, rebase or branch overwrite was performed. Existing circle ancestry and `SurfaceRef::Cylinder` are verified in source; unpublished future interfaces cannot be certified compatible.

A coarse circle carrier and its ideal circle are different source envelopes. Reconstructing an ideal toroidal roundover may exceed the authoritative inscribed carrier or violate the requested chord error. The polygon evaluator deliberately refuses attributed circular boundaries. A future dedicated evaluator must certify both meridional and circumferential approximation, preserve old geometry outside the band, and keep output ancestry suitable for a later operation. Reusing the polygon algorithm and dropping analytic claims would not meet curved acceptance.

## Verification record

- Baseline before source changes: all 38 existing focused kernel/document tests passed.
- Current focused kernel/document suite: 42 tests, including 32 generated convex-perimeter property cases within one test. Includes rigid transforms, duplicate/oversized/overlapping refusal, source immutability and actual curve-ancestry refusal.
- Native WASM API library: all 257 tests passed, including new polygon/sequential preparation, pure range probing with an existing handle, commit, exact undo/redo, save/load and replay.
- Mesh-export: entire package tests passed; shared STL writer/certificate implementation was not changed.
- Frontend focused control/units tests: 218 passed before the final slider ARIA-only adjustment; final broader run below supersedes this intermediate result.
- Actual generated release Node WASM smoke: 4 passed. New perimeter tool tests independently decode STL topology, winding, volume, dimensions, containment of unchanged vertices and persisted vertex sets.
- Rust formatting and kernel/WASM all-target Clippy with warnings denied passed during development; final checks are recorded in the delivery receipt.

Final frontend/browser/build results are appended below. No full native workspace test run or Fedora desktop build is claimed here. The existing host lacks GTK3/WebKitGTK 4.1; the broad native-workspace test footprint previously exceeded available disk. Preserve caches instead of deleting them to manufacture a clean build.

## Fedora handoff and remaining work

The separate identity remains `com.hew3d.Hew.EdgeTreatmentAlpha`, with no-bundle overlay and isolated XDG roots. No installed Hew application or alpha state was modified.

Local commits are not available through `git pull` until a push is separately authorized. For the existing checkout, first preserve local edits and verify the branch; after an authorized publication, use:

```bash
cd ~/src/hew-edge-treatment-alpha
git status --short
git switch feature/edge-treatment
git pull --ff-only origin feature/edge-treatment
git rev-parse HEAD
# Confirm this matches the delivery SHA before continuing.
pnpm install --frozen-lockfile
wasm-pack build crates/wasm-api --target nodejs --out-dir ../../tools/replay-runner/pkg-node
node --test tools/replay-runner/edge-treatment-smoke.mjs
pnpm --dir app test src/tools/EdgeTreatmentPerimeter.wasm.test.ts --maxWorkers=2
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

Use the already configured Rust/Node/pnpm environment from [the Fedora handoff](FEDORA45_EDGE_ALPHA.md). These commands preserve caches; do not run clean/reset or install a bundle. Stop on a failed check/build. Sidecar staging honors the existing Cargo target configuration; the launch path assumes the established default shell target directory.

Next smallest steps are a finite-domain curved-surface certificate/source-authority decision and a bounded asynchronous evaluation interface. Neither should weaken the working convex cap-band implementation or its manufacturing checks.

## Final QA detail and changed-file map

The full frontend invocation executed 6,701 tests across 308 files: 6,700 passed and one failed because that invocation lacked `rustfmt` on PATH. Re-running `src/kernelErrorsDump.test.ts` with the existing Rust environment passed both tests in that file. This is an environment-corrected result, not a claim that the initial full command exited green. After the final cache invalidation/ARIA changes, 72 focused tests passed; after adding the explicit real-WASM circular-boundary refusal case, the final control/panel/perimeter run passed 43 tests. The latter includes all ten generalized perimeter/refusal cases. No missing generated WASM module is silently skipped by the new acceptance file.

All 11 browser tests passed, including original connected two-/three-/four-edge treatments, tooltips and the new trapezoid slider/sequential case. The final targeted browser pass against optimized web WASM passed all three tests (metric precision, parameter help, and trapezoid slider/sequential treatment). The production frontend/PWA build passed after optimization. Production/test TypeScript checks, application lint and shipped-language checks passed. The first production build correctly refused an unoptimized 17 MB web WASM asset above the existing 10 MB PWA limit; applying Binaryen stripping/optimization reduced it to approximately 8.4 MiB. The PWA limit was not changed, and generated build outputs are excluded from commits.

The rerun existing metric suite independently decoded exported geometry. In its 28 emitted metric/chord/decimal measurement records, maximum absolute parameter error was 0.0000122070312751 mm (0.0000004805917825 inches), maximum outer STL dimensional error was 0.0000122070312747 mm, maximum conversion error was 3.47e-15 mm, and equivalent imperial/metric mesh error was zero. Maximum measured chord deviation across those differing requests was 0.2407697678 mm for the 0.25 mm setting, with no reported chord excess. The largest relative error was 0.0001220703123 for a small feature at an f32 STL coordinate; it is quantization, not intentional display rounding. These figures describe the existing single-edge metric measurement matrix, not exhaustive dimensional certification of every new polygon/scale combination. New cap-band tests independently verify STL volume/dimensions and unchanged-region vertices on their specified fixtures; full scale/translation coverage remains future work.

Production source changes since the Phase 2 baseline:

- Geometry: `crates/kernel/src/edge_treatment.rs`, `crates/kernel/src/edge_treatment/multi.rs`, new `crates/kernel/src/edge_treatment/perimeter.rs`.
- WASM integration: `crates/wasm-api/src/edge_treatment_api.rs` only; no main Scene/core document storage change.
- UI/orchestration: `app/src/tools/EdgeTreatmentTool.ts`, `app/src/panels/EdgeTreatmentPanel.tsx`, `app/src/tools/types.ts`, `app/src/viewport/Viewport.tsx` and the three existing locale catalogs.
- Tests: new `crates/kernel/tests/perimeter_edge_treatment_specs.rs` and `app/src/tools/EdgeTreatmentPerimeter.wasm.test.ts`; updated tool/panel/metric tests and `app/e2e/edge-treatment.spec.ts`.
- Documentation: preserved architecture/test-matrix/control-design/modularity review, cross-links and this checkpoint report.

No source files are intentionally left out of the milestone commits. Generated WASM, frontend output, logs, caches and credentials are not included. No dependencies, alpha identity, native shell configuration or installed application were changed.


## Offline incremental transfer without a push

The delivery includes `hew-edge-treatment-phase3-foundation.bundle`, containing only the new local commits and requiring the existing Phase 2 baseline/history. Transfer that file to Fedora (for example `~/Downloads/hew-edge-treatment-phase3-foundation.bundle`). With a clean working tree in the existing alpha checkout, use these commands **instead of the `git pull` line above**, then continue at `pnpm install`:

```bash
cd ~/src/hew-edge-treatment-alpha
git status --short
git switch feature/edge-treatment
git bundle verify ~/Downloads/hew-edge-treatment-phase3-foundation.bundle
git fetch ~/Downloads/hew-edge-treatment-phase3-foundation.bundle feature/edge-treatment
git merge --ff-only FETCH_HEAD
git rev-parse HEAD
```

Compare the resulting full SHA with the final delivery receipt. The bundle is incremental, not a standalone replacement clone: verification must find its prerequisite baseline. No force operation, remote publication, alpha-state removal or installer is involved. The accompanying final receipt identifies the bundle's branch tip; the documentation commit cannot embed its own SHA.
