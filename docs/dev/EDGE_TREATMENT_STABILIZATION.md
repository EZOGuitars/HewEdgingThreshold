# Pre-Fedora Edge Treatment stabilization

## Result and preserved baseline

This is a targeted stabilization pass, not a new development phase. Baseline
`34a889caa3661cfb2e1b5d98a67b190e5c5b84e0` was confirmed on the private remote
`EZOGuitars/Hew3dEdging`, branch `feature/edge-treatment`, **before** corrections.
The original ten signed-off commits are retained unchanged. The final delivery
receipt records the appended stabilization SHA and its independently verified
remote state; no force push, public repository, upstream PR, or Fedora-specific
code change is part of this pass.

The reproducible tolerance exceedance is explained by **STL float32 coordinate
quantization**, not a defective kernel segment calculation. New Scene roundover
preparations explicitly refuse uncertifiable STL geometry. The geometry kernel,
Follow Me, file format, recording schema, dependencies, and requested parameters
are unchanged. The required 0.250-inch / 6.35 mm case remains supported.

## Exact reproduction and root cause

Reference: fresh 12-inch cube, side **0.3048 m**, selected edge at
`x=0.3048, y=0`, radius **0.1016 m (4 inches)**, requested chord error
**0.00001 m (0.01 mm)**. The reference circle center is independently known as
`(side-radius, radius)` in the transverse plane.

`crates/mesh-export/tests/stl_chord_precision.rs` independently reconstructs the
arc from actual vertices and measures each chord's closest point/endpoints.
It compares BRep, the export tessellator's f64 soup, actual decoded binary STL,
and the separate f32-meter render buffers. It also confirms that applying the
writer's mm→f32 conversion directly reproduces the decoded STL exactly.

| Stage | Maximum chord deviation, mm |
|---|---:|
| Analytic `R*(1-cos(pi/(4*N)))`, N=56 | 0.009992181227178 |
| Actual kernel BRep | 0.009992181227214 |
| Actual export tessellation, f64 meters | 0.009992181227214 |
| Actual decoded STL, f32 millimeter coordinates | 0.010002231570536 |
| Separate viewport tessellation, f32 meter coordinates | 0.010002908875012 |

The BRep and export tessellation agree exactly in this measurement. Their
approximately 3.6e-14 mm difference from the analytic formula is insignificant
here. Quantization adds approximately **0.000010050343322 mm** to the worst
STL chord deviation, exceeding the remaining **0.000007818772786 mm** margin.
The resulting strict-bound excess is **0.000002231570536 mm** (about 2.23 nm).
Different float32 grids explain why preview-buffer and STL measurements differ.
Preview buffers are rendering approximations, not geometric accuracy evidence.

Located implementation:

- `crates/kernel/src/edge_treatment.rs::roundover_segments`: minimal eligible
  segment count, using the native geometry budget; **N=56 is correct**.
- `crates/tessellate/src/refacet.rs::export_triangles_with_faces`: f64 stored
  export facets, separate from f32 render buffers.
- `crates/mesh-export/src/lib.rs::write_binary_stl`: converts meters to
  millimeters and serializes coordinates as f32, where the strict excess begins.

There is no evidence justifying a change to the geometry kernel or its segment
policy. The independent measurement method is corroborated, not blamed.

## Narrow correction: explicit preparation certification

New `crates/mesh-export/src/roundover_precision.rs::certify_roundover_stl`
accepts a known source/edge and its native roundover candidate. It invokes the
existing STL writer, including finite-coordinate, winding, watertightness, and
manifold integrity checks, then reads the actual serialized triangle positions.
The reference circle axis/center comes from source edge endpoints and adjacent
face normals, not global axes. Inherited faces are excluded from circular-surface
checks; transition facets are identified geometrically.

For each transition triangle, radial distance from the reference axis is convex,
so its maximum is attained at a vertex. A separating half-space proves that the
axis is outside the triangle's perpendicular projection; the minimum is then on
one of its three projected edges. The check measures every edge's closest point
and endpoints. It covers both ends, longitudinal edges, cap-adjacent facets, and
quantization-induced twist, rather than only the lower cap or a theoretical sag.
An additional **64 × f64 epsilon × coordinate/length scale** arithmetic budget
is included in the certificate, separately from measured float32 quantization.
Nonfinite or uncertifiable cases refuse.

`crates/wasm-api/src/edge_treatment_api.rs::Scene::prepare_treatment` calls the
certificate after native geometry validation and before publishing a handle.
Failure returns typed code **`StlChordPrecision`**. No geometry is repaired,
clamped, translated, or automatically resegmented. Failure leaves document bytes,
history, and recording untouched; existing failed-prepare ownership semantics
remain. The frontend maps this code to actionable localized feedback.

A conservative displacement allowance could force extra tessellation even when
an actual serialized candidate already satisfies the request. Instead, this
check certifies the actual candidate or explicitly refuses it. No format policy
is inserted into the pure kernel and no new persistent curve metadata is added.

An explicit user request of **0.0099 mm** for the original 4-inch radius uses
**57 segments**, one more than the original 56 (about 1.8% more arc segments).
Its independently decoded chord deviation is **0.009655542226261 mm**, and its
conservative certificate is **0.009655542230593 mm**. This demonstrates a small
workaround without silently changing the user's original 0.01 mm request.

The required **0.250-inch / 6.35 mm** case still uses **14 segments**, with
independently decoded STL deviation **0.009997820746607 mm** and certified bound
**0.009997820750951 mm**, both below 0.01 mm. Its dimensional accuracy is unchanged:
maximum measured setback error approximately **0.0000122070313 mm**. Imperial
and metric meshes remain equivalent, and repeated unit switching introduces no
canonical drift. See the [compact executed measurements](EDGE_TREATMENT_STABILIZATION_MEASUREMENTS.json).

## Additional confirmed verification defect

Rechecking the preserved baseline log
`metric/test-types-final.log` found **TS2783**: `requested_chord_mm` was specified
both explicitly and by spreading `measured` in the dimensional test's JSON
output. The earlier handoff incorrectly described that test type-check as PASS.
Its shell sequence had continued to lint after the failed check, masking the
failure at the aggregate-command level. The preserved failure log remains intact.

The redundant property is removed from
`app/src/tools/EdgeTreatmentMetric.wasm.test.ts`; measurement values and geometry
are unchanged. Type-check/lint/build commands now stop on failure, and their
individual outcomes are checked. Historical reports are marked as superseded;
original measurement data and the ten baseline commits are not rewritten.

## Verification and disk control

Entry disk space was **776 MiB free** on the 7.9 GiB workspace. Removing 35
reproducible generated test executables recovered roughly 855 MiB of logical
binary data; source, patches, Git history, archives, and reports were retained.
No dependency or source pruning was used to make the tests fit.

The root workspace contains 19 packages. To avoid linking every integration
binary before running any tests, the complete default Rust test set is selected
by workspace target class and each distinct integration target name:

```bash
export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
cargo test --workspace --locked --lib --message-format=json -- --test-threads=2
cargo test --workspace --locked --bins --message-format=json -- --test-threads=2
# For every distinct integration target reported by cargo metadata:
cargo test --workspace --locked --test TARGET --message-format=json -- --test-threads=2
cargo test --workspace --locked --examples --no-run --message-format=json
cargo test --workspace --locked --doc --message-format=json -- --test-threads=2
```

Cargo's workspace feature selection is retained. The external orchestration
receipt records every exact command, exit status, test count, duration, and free
space. After a successful command, only generated test executables identified
by Cargo compiler-artifact JSON are removed (and example executables after their
compile check). Libraries/dependencies are reused. This closes the same test
coverage without rerunning the disk-exhausting all-binaries-at-once invocation.
It is **not** a claim that `scripts/verify.sh` or `verify-full.sh` passed in full;
those scripts also include additional frontend/site/desktop lanes.

**PASS:** all **134 commands** completed successfully in **1,398.9 seconds**
(about 23 minutes 19 seconds): **3,281 tests passed, zero failed, three ignored**.
All 130 distinct integration target names (137 package targets), workspace
library/binary tests, ten example compilations, and the doctest phase were
covered. The examples are `test=false` in Cargo metadata and are compile-only;
the doctest phase contained zero runnable cases, not additional test passes.
Lowest logged command-start free space was **1,507 MiB**; instantaneous peak
build usage was not sampled. There was no disk exhaustion in this run.

The previously blocked `commands_library` and `commands_view` targets passed
**16** and **18** tests respectively. The complete default root Rust test
coverage gap is closed through bounded execution; the old all-at-once failed
invocation remains a historical failure, not a newly passing command.
[Every command, status, count, and duration](EDGE_TREATMENT_STABILIZATION_RUST_RESULTS.json)
is preserved in the source documentation; raw logs and the orchestration script
are retained with the external stabilization receipts.

The three **NOT RUN / intentionally ignored** tests are:

- `pdfwrite::tests::dump_sample`: optional PDF output generator.
- `dae-import::real_file_smoke`: requires `HEW_DAE_FILE` pointing to an external
  local `.dae` that was not supplied.
- `gltf-import::report`: diagnostic printer, not an assertion-based test.

None was counted as a pass or silently enabled/disabled by stabilization.
Final workspace Clippy with warnings denied and Rust formatting check passed
again after the code changes.

Focused checks executed for the change:

- Three STL precision specifications: exact reproduction, no mutation,
  required 6.35 mm case, explicit finer 57-segment alternative, arbitrary-axis
  rotation, and explicit refusal at large coordinate magnitude.
- Native WASM atomicity/history regression: refused request preserves exact
  bytes and depth; required case prepares; finer case commits once, exports,
  and Undo/Redo restores exact snapshots; no refused treatment is recorded.
- **242 frontend tests in five files**, including **30 actual-WASM dimensional
  cases**, lifecycle fault injection, unit conversions, 100 switching cycles,
  unchanged canonical dimensions, all five chord settings, STL integrity, and
  external-unit reimports. The 4-inch case now explicitly verifies refusal,
  then equivalence for the user-selected 0.0099 mm setting.
- **Four actual Node/WASM smoke tests**; **three Chromium/WebGL integration
  tests**, including 6.35 mm measurement, dirty-state/renderer fault, and Undo.
- Application and test type checks, lint, English catalog generation, production
  frontend/PWA build, release Node/web WASM builds, Rust formatting, and workspace
  Clippy with warnings denied. WASM builds used pinned wasm-pack 0.15.0 and
  matching wasm-bindgen, `--mode no-install --no-opt`, followed by metadata-only
  stripping for the web build, as in the preserved handoff.

No expensive full frontend suite was repeated: the production change is a
synchronous treatment preflight plus a localized error branch; unrelated App
open/dialog orchestration is untouched. The previous **6,683-test** runtime pass
and **6,654-test** Phase 4C pass remain historical evidence, not newly executed
checks. The current focused tests exercise the changed runtime paths directly.

## Historical timing failures

The original failure list and raw logs were re-reviewed alongside reset and
cleanup hooks in `app/src/App.test.tsx`, `App.openInNewWindow.test.tsx`,
`panels/LibraryDialog.test.tsx`, `shop/ShopApp.test.tsx`, and shared
`app/src/test/setup.ts`. These files share mutable mock/storage/orientation
fixtures but contain explicit resets; shared React cleanup unmounts after tests.
Their source and normal five-second timeout are unchanged by stabilization.

The ten failures comprise eight timeouts and two missing asynchronous Open
report dialogs. The old run overlapped heavy native/optimizer work under a
2-core quota, so contention is plausible, but original peak utilization and
exact complete ordering were not saved. There was no recorded frontend OOM or
disk-full failure explaining those ten. Subsequent controlled complete runs
passed. No reproducible test-order dependency, attributable cleanup defect, or
shared-state cause was demonstrated. They remain **unresolved as to root cause**;
no speculative production changes, timeout increases, assertion weakening, or
test removal were made. The separate confirmed Rust-PATH/menu-artifact issues
remain distinct, as detailed in the [Phase 4C audit](EDGE_TREATMENT_PHASE4C.md).

## Remaining limits and Fedora recommendation

The certificate applies to a newly prepared candidate at its current world
coordinates. Pure kernel/native document APIs retain their f64 geometric
contract. Existing saved polygon meshes do not store their original radius/chord
request, so generic STL export cannot infer or retroactively enforce it. Later
transforms or edits require recertification where that original bound matters.
No file-format change or new analytic metadata was introduced to conceal that
limitation. Older semantic recordings containing a now-uncertifiable placement
can explicitly refuse under the new validation policy.

STL still has no physical units and uses float32 millimeter coordinates.
Ordinary dimensional quantization remains documented; certification enforces the
requested transition chord bound, not infinite precision of every dimension.
Preview buffers remain f32 render approximations. Curved/concave/partial-edge
features remain unsupported. Synchronous computation, conservative stale-state
checks, and the historical timing uncertainty remain unchanged.

No Fedora-specific code/config change was attempted. WebKitGTK/Fedora execution
is **NOT RUN / environment-unavailable here**, not PASS. The existing isolated
[Fedora guide](FEDORA45_EDGE_ALPHA.md) and
[manual checklist](EDGE_TREATMENT_ALPHA_CHECKLIST.md) remain applicable; use the
new final SHA from the delivery receipt. Keep the installed Hew untouched.

With the confirmed precision issue explained and explicitly guarded, this is a
candidate for **controlled experimental Fedora alpha testing**, not production
approval. A rejected request should be retried with a deliberately finer chord
setting or more suitable placement; do not bypass the guard. Stop here for
engineering review and actual Fedora/WebKitGTK results. No Phase 5 or upstream
PR is started.
