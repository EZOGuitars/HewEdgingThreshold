# Connected Edge Treatment alpha

This milestone integrates simultaneous treatments of original edges on one
unmodified rectangular solid. General polygonal, nonconvex, curved-edge,
concave-fillet and variable-radius support remain out of scope. Follow Me is
unchanged. This is an experimental alpha, not a general fillet kernel.

## Interaction

Activate Edge Treatment, then click the middle of a straight edge. A plain
click replaces the selection; Ctrl-click (Command-click on macOS) toggles an
edge on the same solid. All selected source edges are highlighted orange.
Adding an edge from another solid refuses without committing anything.

**Select face perimeter** arms a face pick. Click a face interior to replace
the selection with its topological perimeter. For this milestone the batch
must belong to an unmodified rectangular solid. **Clear** cancels the candidate
and selection; **Cancel**, Escape and switching tools cancel without history.
Every accepted selection/parameter change debounces a fresh cyan preview.
One common distance or radius/chord setting applies to the whole selection.
Apply commits the entire candidate once; one Undo restores the original.

Radius, chamfer distance and maximum chord error have information buttons
supporting hover, keyboard focus, click/touch toggle and Escape dismissal.
Only the selected operation's parameters appear. Canonical units and the
accuracy-based chord-error default are unchanged. A valid roundover preview
reports its actual segments per quarter arc. Lower chord errors request more
facets; there is no manual segment override in this milestone.

Two selected edges at a corner meet along a **visible miter crease**. Three
selected incident edges use a faceted spherical octant. Four edges around a
face have mitered corners. All twelve edges combine cylindrical bands and
spherical corner patches. Disconnected subsets on the same box are supported.
Do not mistake a two-edge miter for a smooth three-surface blend.

## Implementation and safety

- `crates/kernel/src/edge_treatment/multi.rs`: `chamfer_edges`,
  `roundover_edges` and `face_perimeter_edges`. One-edge calls delegate to
  the existing implementation. Batch frames derive from source topology,
  not world axes. The original box is recognized conservatively (8 vertices,
  12 edges, 6 rectangular faces); no fitting or repair is performed.
- All edge constraints are planned against the same original solid. A shared
  indexed half-space boundary construction applies the plan; it neither
  repeatedly treats changing edge handles nor concatenates separate meshes.
  Shared edge intersections are calculated once per clipping plane.
- Result validation checks topology, watertightness, one connected component,
  original/constraint/result supporting planes and independently integrated
  removed volume, counting shared corner cells once. Convex supporting-plane
  and indexed-boundary checks underpin containment and non-self-intersection;
  this is not a general nonconvex intersection validator.
- Radius/distance must be strictly less than half the shortest box dimension
  in batch mode, even for a subset that could admit a larger local cut.
  Roundovers retain the 4–128 segment bounds and add a 4096-plane batch work
  budget. Excessive accuracy requests refuse rather than silently clamp.
- `Document::prepare_edge_treatments` freezes all selected edges in one
  candidate. Existing snapshot/generation guards, annotation detachment,
  commit action, identity, persistence and undo machinery are reused.
- `crates/mesh-export/src/roundover_precision.rs`:
  `certify_treatment_surfaces` checks actual millimeter-float32 STL triangles
  against temporary cylinder/sphere references. Sphere minima include triangle
  interiors. Existing STL writer integrity checks run before certification.
  Single-edge certification remains unchanged. These references are not
  persistent analytic-surface claims.
- Scene adds `prepare_chamfer_edges`, `prepare_roundover_edges`, and
  `edge_treatment_face_edges`. JavaScript uses BigUint64Array handles. Existing
  candidate/mesh ownership and scalar entry points remain. Info adds `edges`,
  `segments` and a junction description. `edge` remains the primary edge.
- The additive `edge_treatment_batch` recording stores original source hash,
  stable object ID, every exact endpoint pair and common parameters. Replay
  resolves every pair uniquely against one source state, then commits once.
  Existing single-edge recordings remain compatible. Older binaries do not
  understand the new batch recording command. The model file format is unchanged.

## Polygon and boss investigation: intentionally not enabled

`crates/kernel/tests/polygon_edge_treatment_specs.rs` reproduces four cases:

| Case | Verified native result |
|---|---|
| Standalone convex hexagonal prism, upper edges | `UnsupportedTermination`: oblique end faces |
| Same prism, vertical edges | `NonPerpendicularFaces` |
| Hexagon split into the base top face and extruded as a boss | One valid watertight, globally nonconvex object; `UnsupportedSolid` |
| Original exterior base edge before/after the attached boss | Succeeds before; after extrusion the same ID/endpoints and perpendicular adjacent faces remain resolved, but preparation returns `UnsupportedSolid` |

An unmerged prism over a base is a separate object and still refuses oblique
termination. Ownership is therefore a distinct issue from these demonstrated
geometry restrictions. No validator was weakened. General prism termination
and safe local treatment of globally nonconvex solids need separate work.

## Verification and certification limits

The integration test inventory is:

- Native kernel single-edge regressions, connected/disconnected box subsets,
  rigid transforms, canonical selection order, safe refusal and the four
  polygon/boss reproductions.
- Native Scene atomic preview/cancellation/staleness, one commit, undo/redo,
  save/load and recording replay.
- Actual Node-target WASM through EdgeTreatmentTool: both operations for two
  edges, three edges, a face perimeter, disconnected edges and all twelve.
  Checks preview purity, cancellation, native watertight status, history,
  persistence/replay and decoded STL closure, edge incidence, winding,
  positive volume, containment bounds and volume reduction.
- Browser pointer tests: Ctrl-add two/three edges and face-perimeter selection
  for both operations, cancellation, Apply, one recorded batch, undo/redo and
  independent decoded STL checks. Existing single-edge/browser regressions remain.

Save/load preserves exact canonical model data, but reconstructed loop starts
can select different triangulation diagonals. Tests therefore compare exact
saved geometry and decoded STL vertex sets, plus mesh integrity and volume
within 1e-6 of source volume, rather than requiring identical STL records.
This tolerance addresses quantized triangulation; it is not a relaxed chord
error. STL uses millimeters by Hew convention and embeds no physical unit.

Remaining certification gaps: exhaustive box edge subsets, very small/large
coordinates, all near-limit radii/tolerances and transformed batch STL accuracy
have not been comprehensively sampled. Preparation certifies the immediate
candidate export; post-load alternative triangulations are not independently
recertified against persisted ideal-surface references. Export checks use stored
facets (`segments_per_turn = 0`); alternative exporter refaceting settings are
outside this certificate. Node WASM and Chromium
are not a Fedora WebKitGTK test. Fedora Wayland/X11, GPU behavior, touch and
installed-app coexistence require local confirmation. Record the exact SHA.

## Recorded integration checks

These results come from the development host (Debian Linux, two CPU cores),
not Fedora 45. Tests use the pinned Rust 1.97.1 and Node 26.7.0 toolchains.

| Check | Result |
|---|---|
| Kernel single-edge specifications | 19 passed |
| Kernel connected/disconnected batch specifications | 4 passed |
| Polygon/boss refusal reproductions | 4 passed |
| Document treatment specifications | 11 passed |
| Complete native WASM API library suite | 256 passed |
| Complete mesh-export test suite | 56 passed |
| Complete frontend suite | 6689 passed across 307 files |
| Final focused panel/tool/actual-WASM suites | 64 passed across 5 files |
| Final Chromium browser lifecycle, connected selections and accessible help | 10 passed |
| Workspace Rust formatting and all-target Clippy with warnings denied | Passed |
| Production and test TypeScript checks; application lint; shipped-language check | Passed |
| Node release WASM and optimized web release WASM builds | Passed |
| Production frontend/PWA build | Passed; current WASM reused |
| Native release CLI build and sidecar staging | Passed |
| Staged-change secret scan | No leaks found |

The complete native workspace test phase was stopped at its entry by a
resource preflight: the previous full run generated 3.02 GB of test executables
alone, versus approximately 1 GB free here. Existing caches were preserved.
The standard verification script's packaging tests (2), Rust formatting and
workspace/all-target Clippy completed first. The focused native suites above
all ran; no full-workspace or end-to-end `verify-full.sh` green is claimed.

A native desktop build/run is **not verified on this host**: pkg-config cannot
find GTK 3 or WebKitGTK 4.1. This is a host prerequisite gap, not a successful
Fedora build. The alpha identity/overlay is unchanged. Use the Fedora checklist
for WebKitGTK, Wayland/X11, GPU, state isolation and installed-app coexistence.

The first release sidecar staging run took 7m46.935s under concurrent build/test
load and included cache population, so it is not a controlled cold benchmark.
The identical next run took 0.191s (Cargo 0.07s), reported unchanged bytes and
skipped copying the sidecar. No Fedora timings or fast-profile results are
claimed. WASM optimization and normal frontend rebuilding remain part of the
existing release pipeline.

## Build scope

CLI staging uses Cargo's authoritative executable path (including a custom
CARGO_TARGET_DIR) and copies only changed bytes, avoiding gratuitous sidecar
mtime invalidation. WASM freshness also requires the complete generated output
inventory and includes its build script as an input. Existing caches and build
profiles are preserved. Content-addressed transitive invalidation, fast custom
profiles, comprehensive benchmarks and `alpha:*` wrappers are deferred.

Use the existing-checkout instructions in [Fedora handoff](FEDORA45_EDGE_ALPHA.md).
Never install an RPM, replace an installed executable or delete alpha state.
