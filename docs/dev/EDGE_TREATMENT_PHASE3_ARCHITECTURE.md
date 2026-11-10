# Phase 3 architecture and contribution-readiness review

Current implementation and release gates: [Threshold verification](THRESHOLD_VERIFICATION.md). Earlier planning/checkpoint statements below are retained as historical records.

Implementation status after authorization: [foundation checkpoint report](EDGE_TREATMENT_PHASE3_IMPLEMENTATION.md). The design/review below is preserved as its original planning record.

Modularity constraint: see [the source-level modularity audit](EDGE_TREATMENT_MODULARITY_AUDIT.md). Retain dedicated feature strategies and adapters; shared-core extensions must be minimal, generic where justified, and reviewed before implementation.

Status: proposal for review; no Phase 3 implementation or synchronization is authorized by this document.

Inspected alpha: `ebb06e4f4a049a9a0c08f7d9acbe09d5c5482816`, on `feature/edge-treatment` in `EZOGuitars/Hew3dEdging`. The working tree was clean at investigation start. All findings below distinguish existing implementation, new reproduction, and proposed work. The companion [test matrix](EDGE_TREATMENT_PHASE3_TEST_MATRIX.md) is a specification, not an implemented test suite.

**Sequencing update:** the subsequent Unified UI and Geometry Foundation directive coordinates sequential, polygonal and circular/mixed-boundary treatments in tandem. The [shared control/engine design](EDGE_TREATMENT_CONTROL_ENGINE_DESIGN.md) is the current sprint contract proposal. Earlier baseline findings remain valid; a linear polygons-then-sequential-then-curves implementation sequence is superseded. No implementation approval is implied.

## 1. Recommendation and scope

Keep Hew's faceted half-edge kernel and the Phase 2 box evaluator. The smallest polygonal geometry slice is **one complete cap perimeter on a convex right polygonal prism**, with equal-distance chamfers or constant-radius faceted roundovers, using the existing indexed half-space boundary construction and profile-offset utilities. Adjacent roundover bands initially meet with the explicitly documented G0 miter seam.

This slice must share source/selection, limit, accuracy, performance and candidate interfaces with sequential current-solid treatments and genuine circular/mixed contours from the start. It is not a standalone prism-only engine or a reason to postpone controls.

A trapezoid's or hexagon's **planar corner angle** is not its upper edge's **dihedral angle**. A right extrusion still has perpendicular cap and wall faces. Generalizing those complete perimeters does not require solving arbitrary-dihedral fillets, arbitrary partial-edge termination, concave solids, or smooth corner blends first. The algorithm must not branch on polygon side count.

Required first acceptance: square, trapezoid, hexagon and octagon upper perimeters, including irregular and rigidly transformed variants. Selecting the opposite cap is the same geometric problem in its own outward frame; add lower-cap parity tests before claiming that capability. Vertical polygon edges with non-right dihedrals, partial polygon perimeters, holes and curved boundaries remain separately classified.

Do not make a parametric feature tree a prerequisite for successive independent cuts. Deliver baked, direct-modeling sequential treatments first; investigate optional editable recipes separately, subject to maintainer agreement and topological naming. Upstream currently describes Hew as having no parametric feature list.

## 2. Verified repository map and reuse

Paths below refer to the inspected alpha revision.

| Implementation | Verified role and reuse decision |
|---|---|
| `crates/kernel/src/topo.rs`: `Object`, `HalfEdge`, `Edge`, `Loop`, `Face`, `SurfaceRef` | Indexed half-edge topology, planar face carriers, inner loops and tracked watertightness. Reuse. `SurfaceRef` currently has only `Cylinder`; no persistent sphere, torus or general spline surface type was found here. |
| `crates/kernel/src/sketch.rs`: `Profile::new`, `Sketch::profile`, `CurveGeom`, `SketchCurveKind` | Validated planar profiles, circles/arcs and polygon attribution. `CurveGeom` stores circle center/radius; this is not a general curve representation. Reuse profile validation and attributed construction. |
| `crates/kernel/src/geom2d.rs`: `plane_axes`, `signed_area_on_plane`, `point_inside_polygon` | Existing crate-private planar geometry. Reuse with a local origin; do not expose another public geometry framework. |
| `crates/kernel/src/offset.rs`: `offset_profile`, `offset_face_boundary`, `offset_loop` | Existing straight/circular offset primitives, miter construction, collapse and orientation guards, validation and named miter bound. Reuse for feasibility and cross-section checks. These are not themselves a 3D treatment or a general topology-changing offset solver. |
| `crates/kernel/src/ops.rs`: `Object::from_extrusion`, `Object::boolean`, `Object::from_follow_me`, `Object::from_follow_me_to` | Existing extrusion, explicit Boolean operations, closed profile sweeps and partial-path handling. Available alternatives and test references, not automatic proof of treatment safety. |
| `crates/kernel/src/document.rs`: `Document::follow_me_merged` | Existing Follow Me automatically chooses subtraction for overlapping sweeps and union otherwise; also consumes drawing scaffolding/base geometry. Do not call this wrapper for a strictly subtractive treatment candidate. Its identity/history semantics differ. |
| `crates/kernel/src/boolean.rs`: `execute`, `plane_line`, `line_region_intervals`, `point_in_solid`, `assemble` | Polygon arrangement, intersection/classification and assembly implementation. Several helpers are private. Reuse via `Object::boolean`; do not duplicate them or expose everything speculatively. Tangency/degeneracy can refuse. |
| `crates/kernel/src/build.rs`: `Object::from_polygons`, attributed indexed builders | Reconstructs topology from indexed boundaries. Reuse; successful construction alone is not complete solid certification. |
| `crates/kernel/src/edge_treatment.rs`: `chamfer`, `roundover`, `resolve_edge_frame`, `check_clearance`, `evaluate_cut`, `validate_convex_solid` | Preserve legacy profile/extrusion/Boolean single-edge route and its tests. Eligibility requires perpendicular incident faces and perpendicular termination faces; the latter rejects upper hexagonal edges. Global convexity is an additional restriction. |
| `crates/kernel/src/edge_treatment/multi.rs`: `BoxFrame::resolve`, `batch`, `intersect_boundary`, `face_perimeter_edges` | Box-specific recognition and independent cell-volume model surround a more reusable convex indexed half-space boundary builder. Extract only the shared builder when justified by the new caller; keep box behavior and constraint order stable. |
| `crates/kernel/src/validate.rs`: `Object::validate`; `ops.rs`: `faces_improperly_contact`, `validate_sweep_result` | Structural invariants plus separate geometric intersection guards. Do not equate structural watertightness with global geometric validity. `validate_sweep_result` is a private push/pull-specific guard, not a public general-solid validator. |
| `crates/kernel/src/health.rs`: `diagnose` | Boundary/rim diagnostics. It returns `Solid` early when no boundary half-edges exist; this is not independent self-intersection certification. |
| `crates/kernel/src/document/edge_treatment.rs`: candidate, `prepare_edge_treatments`, `commit_edge_treatment` | Pure preparation, exact snapshot/generation guards and one `DocAction::EdgeTreated`. Reuse for new geometry. Candidates are ephemeral; this is undo history, not an editable feature graph. |
| `crates/kernel/src/document.rs`: `EntityRef`, `undo`, `redo`; `serialize.rs` | Object/entity stable IDs, snapshot replay and deterministic persistence. `EntityRef` does not provide stable face/edge IDs. Native versions are manifest 26 / geometry 8 at this baseline. |
| `crates/wasm-api/src/edge_treatment_api.rs`: `prepare_chamfer_edges`, `prepare_roundover_edges`, `edge_treatment_face_edges`, `recorded_treatment_edges` | Shared candidate lifecycle, typed diagnostics and exact source-hash/endpoint replay. Existing array entry points can represent polygon perimeters; a new WASM method is not inherently needed for 3A. |
| `crates/wasm-api/src/recording.rs`: `EdgeTreatment`, `EdgeTreatmentBatch` | Existing scalar and additive batch recording commands. Keep both compatible. Replay addresses a frozen source, not topology after parameter edits. |
| `crates/mesh-export/src/roundover_precision.rs`: both certificate functions | Checks serialized millimeter-float32 STL against cylinder/sphere references. Reuse cylinder checks for planar-cap roundovers; references are currently ephemeral. |
| `crates/tessellate/src/refacet.rs`: `export_triangles_with_faces`; `mesh-export/src/lib.rs`: `write_binary_stl` | Actual export triangulation and strict STL integrity writer. Nonzero exporter refaceting may change triangles; certify the actual requested export path. |
| `app/src/tools/EdgeTreatmentTool.ts`, `panels/EdgeTreatmentPanel.tsx`, `viewport/Viewport.tsx` | Current selection, units, cyan preview, cancellation, acknowledgment, commit and accessible help. Reuse. UI arrays use string IDs and convert to BigUint64Array at the WASM boundary. Geometry remains in Rust. |

The general kernel already represents nonconvex polygonal objects and faces with holes, extrudes validated profiles and triangulates nonconvex/holed faces. The current treatment restriction is an operation/validation boundary, not proof that the underlying representation must be replaced.

Circles are faceted topology with optional exact circle/cylinder ancestry, not continuous curved BRep edges. `SketchCurveKind::Polygon` expressly distinguishes actual straight polygon sides from a circle approximation. Nonuniform transformations can drop circle/cylinder claims: an ellipse must not be relabeled as a circle. No general ellipse/NURBS surface-intersection representation was located in the reviewed core types.

## 3. Existing behavior and reproduced limitations

**Newly rerun on the unchanged baseline:** all 38 existing native treatment specifications passed: 19 single-edge, 4 batch, 4 polygon/boss, and 11 document tests. The single-edge suite includes `every_box_edge_uses_its_own_adjacent_face_frame`, `roundover_works_on_every_box_edge_and_at_centimeter_scale`, and rigid-transform property tests. The batch suite includes `batch_frames_come_from_geometry_not_world_axes`. Horizontal, vertical and rotated box edges therefore have executable regression evidence; they are not globally Z-oriented algorithms.

The four existing polygon tests reproduce standalone hexagonal upper-edge `UnsupportedTermination`, vertical hexagonal `NonPerpendicularFaces`, and attached-boss `UnsupportedSolid`. The original base edge retains its ID/endpoints and perpendicular incident faces after adding the boss, but global convexity refuses it. An unmerged prism has separate ownership and still has the oblique-termination issue. These restrictions are different causes, not one selection bug.

### Confirmed sequential perimeter refusal (Fedora observation and actual-WASM reproduction)

The supplied Fedora screenshot shows a completed bottom perimeter, four selected upper edges and a second request of radius `0.0125 in`, chord error `0.0039 in`. It confirms selection still works while batch evaluation refuses. Dimensions of the pictured source and first radius were not supplied and are not inferred.

An additional deterministic reproduction used the existing generated Node WASM for the baseline:

| Step | Measured outcome |
|---|---|
| Fresh 100 × 60 × 20 mm box | One source object |
| Bottom perimeter, radius 2 mm, chord 0.05 mm, commit | Success; resulting mesh reports watertight |
| Re-pick top face perimeter | Four edge IDs returned |
| Request top radius 0.3175 mm / chord 0.09906 mm | `UnsupportedBox: multi-edge treatment requires an unmodified rectangular prism` |
| Compare saved bytes and undo depth before/after refusal | Both unchanged |
| Control: prepare/discard one of those upper edges with the same dimensions | Success; no commit or history change |

The batch refusal occurs in `BoxFrame::resolve`'s 8-vertex/12-edge/6-quadrilateral gate, before its convex-solid check. The first batch is an intersection of convex half-spaces and remains convex. The successful scalar control also passes the current convex validator. This case is therefore **modified-topology eligibility**, not demonstrated nonconvexity, selection failure, excessive radius or invalid chord error. Preserve refusal until a new certified cap-band strategy exists. It is a mandatory Phase 3B success test, not a completed feature.

The earlier 350 native / 6689 frontend / 64 focused / 10 browser Phase 2 results remain historical evidence in `EDGE_TREATMENT_PHASE2_ALPHA.md`; those full sets were not all rerun in this architecture-only pass.

## 4. Geometry architecture comparison

| Approach | Correctness / numerical behavior | Cost, maintainability and disposition |
|---|---|---|
| Generalize existing convex half-space/indexed boundary | Strong containment and convexity construction; naturally produces polygon-corner miters. Near-parallel constraints and tiny faces still require refusal. Cannot represent general concave Boolean differences as one convex intersection. | **Preferred 3A.** Reuse `intersect_boundary`, with a generic perimeter planner and independent volume model. Preserve the box route. |
| Offset rings joined into indexed bands | Common-offset stations offer shared vertices and near O(edges × segments) construction. Requires constant topology, correct planar band quads and explicit collision checks. | Feasible optimization/reference construction. Do not introduce a second production boundary engine unless profiling justifies it. Existing offsets are valuable immediately for feasibility and independent checks. |
| Closed swept cutter + explicit Boolean subtraction | Existing `from_follow_me` produces validated watertight solids from closed profiles; closed and open *paths* are supported. It is not an open-sheet cutting tool. Sweeps can fold at tight turns, and valid sweep plus valid target does not guarantee a valid difference. | Useful small feasibility spike or secondary cross-check. Extra sweep/Boolean tolerances and contact cases make it less predictable for the initial convex perimeter. Keep Follow Me behavior unchanged; never reuse the auto-union wrapper blindly. |
| Local trimming / topology replacement | Can preserve unrelated topology and later handle locally convex regions of nonconvex solids. Intersection curves, termination, corner patches and stitching become explicit responsibilities. | Needed later; substantially more validation and implementation effort. Reuse half-edge structures and intersection predicates through narrow interfaces. |
| General analytic BRep / new kernel | Can eventually express exact curved surfaces and G1/G2 conditions; conversion, trimming, licensing, serialization and platform costs are large. | No evidence supports replacing Hew's kernel now. Circle/cylinder metadata is not already a general BRep engine. |
| Procedural definition + evaluated faceted output | Can retain intent and quality while keeping Hew's current mesh output. Requires deterministic evaluation, stable references and compatibility rules. | Investigate as optional treatment recipes after sequential direct modeling; not a prerequisite or a new global framework. |

Do not compare these alternatives using appearance alone. A post-approval spike should compare runtime, peak allocation, exact refusals, measured geometry, STL deviation and locality on the same four required fixtures. The existing boundary builder is the default unless measurements show a compelling alternative.

## 5. Concrete convex-prism design

### Eligibility and interfaces (proposed, not existing APIs)

Use a shared current-source/cap-region and boundary plan, with a prism recognizer as one proof provider, not the root interface. A private `prism.rs` is justified only if the family implementation warrants it; circular/mixed and sequential clean-band providers use the same request/plan lifecycle. A small `boundary.rs` may hold the existing builder moved mechanically from `multi.rs`. Keep tolerance definitions in `kernel::tol`; do not create another crate. See the unified design for control, range and resumable evaluation boundaries.

Conceptual internal records: `PrismEvidence` (caps, outward frame, wall correspondence, height, source validation), `PerimeterPlan` (ordered source edges, constraints, patch references, work estimate, validation expectations) and later `CapBandCertificate` (certified unchanged extrusion band near a selected cap). These are design names, not located capabilities.

Recognition must prove two parallel congruent simple caps, uniform nonzero extrusion, perpendicular planar side walls, consistent winding and vertex correspondence, one valid closed convex component, and no holes/analytic curved-edge claims. Counts derived from side count are necessary checks, never sufficient proof. Select a complete outer edge set belonging to one unambiguous cap; reject incomplete/ambiguous sets. Infer axes from faces and extrusion incidence, never global Z or bounding-box guesses.

Dispatch explicit geometric families. Exact Phase 2 boxes retain their evaluator, ordering, work limits and geometry. New convex right prisms use the new planner only for complete cap perimeters. Invalid geometry must not trigger a catch-all fallback. The box's 12-edge limit cannot become a global polygon side-count limit; retain a bounded constraints/faces/memory policy instead.

### Construction

Let `k` be the selected cap's outward unit normal, `n_i` each wall's outward unit normal, and `e_i` a point on that cap edge. Their dot product is zero for this milestone, irrespective of polygon corner angle.

- Chamfer distance `d`: keep the half-space through `e_i - d n_i` with outward normal proportional to `n_i + k`, for every perimeter edge.
- Roundover radius `r`: cylinder axis is the original edge line shifted by `-r(n_i+k)`. Sample its quarter arc with the existing accuracy policy and create chord planes. Intersect all planned planes with the **same original source** once using shared indexed construction. Adjacent bands automatically terminate at their common miter; no perpendicular end-cap assumption is needed.
- `offset_profile(profile, -r)` or `-d` supplies a feasibility/reference inset. Refuse collapsed, reversed, excessively mitered or topology-changing offsets; verify every inset vertex against all offset supporting lines and require positive directed edge lengths. Do not treat a successful `Profile::new` alone as proof of offset correspondence. The existing offset routine also refuses zero distance; use the original ring explicitly at the zero station.
- Require the treatment band to fit within the height, with a positive residual region and constant inset topology. Reject limits and intersecting regions without clamping. Untouched lower geometry remains original; top flat face is the inset footprint.

For strict convex straight-sided profiles, the directed length of edge `i` after inset `s` is `L_i - s*(cot(alpha_i/2)+cot(alpha_(i+1)/2))`. The smallest positive zero is the first edge-collapse event; use it as an explicit bound with named clearance tolerances, alongside height and miter limits. This avoids assuming the polygon's inradius is the first relevant event. Verify the computed ring against all inset supports as well.

The legacy box route retains its conservative `2*a < min_dimension` policy for distance/radius `a`. Do not fall back to a new evaluator merely to evade an existing refusal. Harmonizing conservative old limits with the new cap-band domain requires a separate reviewed contract, not an incidental test change.

At radius `r`, with height coordinate `z` and cap height `H`, the ideal inset through the band is

`s(z) = r - sqrt(r² - (z - (H-r))²)`, for `H-r <= z <= H`.

This describes the intended cylinder envelopes and mitered polygon corners, not a smooth corner blend. Inset events can occur before full polygon collapse, especially on irregular polygons: detect the first edge-collapse event and refuse beyond it for 3A.

### Independent volume and dimensional oracles

For a convex polygon with area `A`, perimeter `P`, interior angles `alpha_i`, and `K = sum(cot(alpha_i/2))`, a constant-topology inward offset has area `A(s) = A - P*s + K*s²`. This identity is valid only before its first combinatorial event.

Expected chamfer removal: `P*d²/2 - K*d³/3`.

Expected ideal roundover removal: `P*(1-pi/4)*r² - K*(5/3-pi/2)*r³`.

For the actual faceted boundary, use independent station integration:

- `I1 = sum(delta_z * (s_j+s_next)/2)`;
- `I2 = sum(delta_z * (s_j²+s_j*s_next+s_next²)/3)`;
- faceted removal `P*I1 - K*I2`.

Compare triangulated signed source/result volumes to the faceted expectation, and report deliberate ideal-to-faceted excess separately. Compute `A`, `P`, angles and signed volumes in test code independently of the production planner. Validate tangent setbacks and cylinder radius with geometric distances, then decode STL and repeat measurements. Extend the existing cylinder certificate rather than assuming the theoretical sagitta includes export quantization.

Preserving the envelope means containment and untouched-region preservation, not necessarily an unchanged world-axis bounding box after rotating a cut solid. Measure dimensions in the design/extrusion frame; use world AABBs for containment only where appropriate.

## 6. Sequential modeling and persistent operations

### Coordinated sequential slice (formerly 3B1): successive direct-modeling cuts

Generalize recognition from an entire pristine prism to a certified clean cap band on the *current convex solid*. A previous bottom treatment can remain untouched while top constraints are applied. Prove the incident walls extend throughout the new band and other existing faces do not intrude into it. For opposing operations, test their depth bands for overlap and global result validity; do not merely add radii and assume arbitrary geometry is safe.

Required success cases: the exact screenshot second request, unequal opposite roundovers, then upper `0.25 in` roundover plus lower `0.125 in` chamfer. Each commit is one native action, preserves earlier output outside the new region, and has independent undo/redo, save/load, replay, volume and STL checks. Exercise both orders. This requires no new file format and no feature tree if the result remains baked geometry.

There is an additional metadata obligation: the current boundary builder initializes persistent output `SurfaceRef` entries to `None`, which is valid for its current planar-box inputs. Reusing it on a previously scalar-rounded source would otherwise discard existing cylinder ancestry and could change later refaceted exports outside the new cut. Separate persistent source-face attributes/provenance from temporary new-patch certificate references; retain valid untouched/sub-face claims under the existing map-or-drop contract. General convex carrier validity must be separated from a strategy's prohibition on curved *input boundaries*, while leaving the legacy validator and its old call paths intact. Do not simply delete its `surface.is_some()` guard.

An imported or directly edited shape without a provable clean band must refuse; semantic resemblance to a box is insufficient. Nonintersecting upper/lower cases do not establish support for touching, mixed or different-radius junctions.

### 3B2: optional editable recipes, subject to upstream agreement

Existing `ObjectId`/SID stability and exact endpoint replay do not solve topological naming. `commit_edge_treatment` explicitly invalidates face/edge runtime IDs. A geometric hash detects mismatch; it does not establish identity or resolve symmetric alternatives.

A possible recipe stores an immutable canonical base snapshot/hash, versioned evaluator, ordered operation IDs, type, dimensions in meters, quality policy, durable recipe-local source references, evaluation order and validation summaries. Initial references can identify entities in that frozen base's canonical topology, guarded by the base hash. Generated descendants need explicit provenance from operation ID and geometric role; raw facet index is not stable when segment counts change. Exact signatures can verify/disambiguate, never silently select the nearest surviving edge.

On edit/suppression/reorder, evaluate the entire affected chain on a detached copy, resolve every dependency uniquely, validate all results, then replace in one transaction. An unresolved/ambiguous reference or downstream collision refuses without replacing the live solid. Begin with editing only a terminal treatment whose current output fingerprint still matches. An unrelated direct edit must invalidate editable intent or require explicit reconciliation; regeneration must never erase that edit.

Keep evaluated geometry authoritative for display/export. A newer/unsupported recipe evaluator may allow opening the baked solid with editing disabled, but saving must preserve opaque recipe data or refuse rather than silently discard it. Recipes would require approved manifest/schema/version behavior and file-format documentation. Retain Phase 2 recording variants and legacy evaluation paths; do not reinterpret old source-hash/endpoint recordings as editable history. Full dependency-graph UI and global feature history are separate proposals, not assumed upstream goals.

## 7. Junctions, nonconvex solids and curved boundaries

**Continuity:** faceted carriers are generally G0. Ideal cylinder-to-plane and compatible sphere-to-cylinder targets can be G1, but their tessellations do not become exactly tangent-continuous surfaces. A two-cylinder miter generally remains G0 even in the ideal construction. Sphere/cylinder ideal principal curvatures differ, so tangent matching does not imply G2. Smooth shading changes none of these facts.

For a later convex wedge with interior material angle `beta` (strictly between 0 and pi), an ideal tangent circle has face setback `r*cot(beta/2)`, bisector-center distance `r/sin(beta/2)`, and arc sweep `pi-beta`. Thus radius equals setback only at 90 degrees. An asymmetric chamfer uses separately named face distances `d1`, `d2`; its bevel width is `sqrt(d1²+d2²-2*d1*d2*cos(beta))`, not the 90-degree shortcut. Conditioning and material-side classification must be certified before accepting near-flat or acute angles. Variable-radius work requires an envelope/continuity construction and derivative/clearance conditions; lofting independent circular sections alone is not a validated tangent blend.

3A keeps positional miter junctions. Preserve the box-only three-edge sphere policy; arbitrary polygon three-edge corners must not reuse spherical octants by rotation alone. Later junction planning must solve all incident treatment envelopes together, establish trim boundaries, detect overlap and degenerate patches, and budget surface/trim tessellation error. Unequal-radius and mixed-type junctions are not independent cuts pasted together.

For globally nonconvex solids, replacing `validate_convex_solid` is a new validation strategy, not deleting its check. Separate local classification (incident faces, oriented material wedge, edge neighborhood, termination) from global result proof. Restrict cut volumes to an explicit influence region; compare old/new boundary outside it, use robust solid membership/intersection checks and validate every shell, vertex link and triangle. Reuse `faces_improperly_contact` and existing Boolean machinery only after auditing their coplanar, hole, tolerance and complexity contracts. A globally extended half-space could remove an unrelated boss; it cannot be the universal nonconvex evaluator.

Concave *perimeter turns* are different from concave *edge dihedrals*. Guitar-waist upper rims may be locally convex cap/wall edges along a concave path. A conventional internal concave fillet can **add material** to a sharp cavity; that needs an explicitly additive operation contract. Do not relax subtractive containment to accommodate it. 3C should first target locally convex exterior cuts on nonconvex bodies, including the original base edge after a boss.

Curved-path work needs offset curves, curvature/feature-size clearance, stable transported frames, finite termination and shared trim/junction geometry. A circle can yield toroidal ideal patches, for which no `SurfaceRef::Torus` currently exists. Existing Follow Me honestly leaves turning-path walls faceted. Ellipses and mixed curves need representations their metadata can actually describe. A dense untagged polygon is still a polygon; success on its short edges does not certify a continuous circular roundover.

Allocate total error across source outline approximation, path sampling, section tessellation, junction trimming, evaluation arithmetic and serialized output. Increasing density must reduce a measured error to a stated ideal target; more triangles alone is insufficient. Preserve deterministic face provenance for export validation, including post-load retessellation and nonzero exporter refaceting, which are current certification gaps.

## 8. Face selection versus surface shaping; fretboard workstream

A perimeter operation acts on an ordered boundary and adjacent material. A surface operation changes a face's height/curvature over a domain. Reuse selection/preview/history plumbing, not an inappropriate common geometry solver.

For multiple loops, require explicit loop selection and orientation/material-side semantics. Future hole-loop support must distinguish outer-boundary subtraction from cavity-wall fillets. Do not select every loop implicitly or substitute a convex hull. Current `face_perimeter_edges` rejects holes; preserve that until a reviewed contract exists.

A constant-radius tapered fretboard can be a dedicated cylindrical surface clipped by a tapered blank. Let longitudinal coordinate be `y`, width `w(y)`, centered transverse coordinate `x`, flat bottom `z=0`, and specified center thickness `T_c`:

`z_top(x,y) = T_c - R + sqrt(R²-x²)`, with `|x| <= w(y)/2`.

For `R=12 in=304.8 mm` (also 9.5/16 inches or custom), require `max(w)/2 < R` and positive remaining edge thickness. The cylinder axis follows the centerline. Preserve nut/heel taper planes, lengths and alignment. Include the centerline exactly in tessellation so the specified center thickness is not lowered by a chord spanning across it. Clip shared indexed chord bands to the blank; validate actual radial and vertical height error, which are different metrics.

This center-thickness contract makes edge thickness vary along a taper. Constant edge thickness at *every* section requires a changing center height and is not the same straight cylinder; do not silently promise both. An edge-thickness reference at one named section is another explicit contract. Do not implement until the reference is selected and reviewed. Compound radius is subsequent, not an initial requirement.

VDM-A/B need an actual licensed outline, thickness, hardware/neck reference regions and treatment-clearance fixtures. No VDM model or CAM setup was evaluated here. VDM-C is independent of edge filleting. VDM-D requires downstream unit/import measurements and separate CAM/toolpath/collision validation; a valid STL is not a safe CNC program.

## 9. Validation, tolerances and performance contract

For each claimed success require source and result validation, finite coordinates, closed consistent topology, one intended material component, manifold edge incidence **and vertex links**, consistent winding, nondegenerate faces/triangles, positive oriented volume, absence of improper contact, measured locality/containment and expected removal. For convex 3A, support-plane construction supplies an additional proof unavailable for nonconvex work. Do not present `Object::validate`, `health::diagnose` or an Euler count alone as all those checks.

Independent export checks decode actual STL vertices/normals/indices, check each undirected edge has two opposite incidences, vertex links/components, positive volume, triangle intersections and requested patch deviation including triangle interiors. Surface-distance sampling is a diagnostic, not a maximum-error proof. Report requested/measured lengths, absolute mm/inch error, relative error, actual maximum chord deviation, conversion and STL errors, and ideal/faceted volume separately. Use `1 in = 25.4 mm` exactly; retain canonical meters on display-unit changes.

Named operation/precision tolerances must distinguish construction, angular classification, volume cancellation and export quantization. Do not reuse the imported-mesh tolerance for native geometry or increase tolerances to make a near-collapse pass. Use local frames for conditioning, typed refusal for unresolved predicates and an explicit boundary between accepted and indeterminate cases.

Retain the existing 4–128 section range and explicit work limits until benchmarks justify changes. Preflight resource estimates before allocation. Debounce and stale-handle checks prevent obsolete Apply, but the existing synchronous WASM evaluator cannot cancel halfway through a long call. A worker/cooperative cancellation architecture needs evidence from near-budget timing and separate approval; do not introduce it preemptively. Preserve Cargo/WASM/Vite/Tauri caches and unchanged-sidecar copying behavior. No new dependency or build-system rewrite is proposed.

## 10. Dependency map and coordinated milestones

The current sprint sequencing is coordinated by [the unified control/engine design](EDGE_TREATMENT_CONTROL_ENGINE_DESIGN.md), not a series of isolated shape tools.

| Foundation / slice | Dependencies and acceptance gate |
|---|---|
| F0: upstream and shared contract | Agree source authority/True Curves semantics, canonical dimensions, source tokens, range certainty, finite patch accuracy and asynchronous ownership. Retain Phase 2 and qualify sequential, polygon and circle/mixed fixtures together. |
| F1: shared analysis, limits and evidence | Current-region/cap-band and boundary evidence, geometry-owned whole-operation limits and cost estimates. Reuse indexed boundary and profile offsets where valid. No frontend box-size formula or current-box-only root abstraction. |
| F2: controls and job lifecycle | Precise text + slider, current unit context, latest-only previews, cancellation and accurate atomic Apply integrated against the real evaluator contract from the beginning. Preserve old APIs/recordings and range-query purity. |
| F3-S: sequential geometry slice | Q01 exact top-after-bottom regression and Q02 opposing different dimensions/modes. Prove clean current bands, preserve prior geometry/attributes and validate next-operation eligibility after save/load. No feature tree prerequisite. |
| F3-P: polygon geometry slice | Complete cap chamfers/roundovers on square/trapezoid/hexagon/octagon and irregular/transformed variants. Shared G0 miter policy, independent volume and export certificates; triangles and other side counts extend tests, not algorithms. |
| F3-C: circular/mixed geometry slice | Genuine circle and convex tangent line/arc perimeters, honest source-envelope authority and finite patch certificates, compatible curve metadata and sequential eligibility. Do not count a dense straight polygon surrogate as success. |
| F4: responsive expensive evaluation | Shared cost profiling and resumable/off-thread geometry/validation jobs; optional declared draft quality. The measured ~883 ms case cannot remain an unbroken drag-time main-thread call. This may precede enabling heavy F2/F3 cases. |
| F5: integrated acceptance | All three slices exercise the same controls, range/preview/apply and real-WASM/browser/export contracts. Full supported-host checks and upstream policy gaps resolved before contribution. Preserve isolated alpha state and installed Hew. |
| Subsequent geometry | Locally convex cuts on nonconvex bodies, concave paths/dihedrals with explicit material semantics, arbitrary angles, partial termination and advanced mixed/unequal-radius junctions remain separately gated. No deletion of safety guards. |
| Optional editable recipes | Durable naming, provenance, deterministic evaluator and approved persistence before editing/suppressing/reordering intent. This is not a prerequisite for baked sequential cuts. |
| Separate fretboard workstream | Choose thickness reference; constant cylindrical radius/taper clipping with independent sagitta and thickness checks, followed by CAM/import validation. Compound radius remains subsequent. |

Shared source/boundary/accuracy evidence precedes all three geometry slices, which develop in tandem with controls. Existing right-prism recognition is one useful source-evidence subtype; clean current bands and circular/mixed path semantics must influence the foundation now. Stable naming and versioned regeneration are additional dependencies of editable intent, not of every successive direct-modeling operation. General nonconvex validation and G1/G2 blending need their own proofs.

Implementation PRs can remain small and independently reviewable without turning the capabilities into unrelated tools. Each slice carries specs, property/integration tests and docs; no ignored or safely refusing Category B case counts as successful implementation. Coordinate exact upstream interfaces before cross-crate, public-WASM or persistence changes.

## 11. Risk register

| Risk | Required mitigation / release gate |
|---|---|
| Acute polygon corners, short edges, nearly parallel walls | Bound miter growth; certify inset event/positive edges and refuse ambiguity. No spikes or welding. |
| Bands intersect, top vanishes or thickness is insufficient | Full band/offset feasibility; independent volume and locality. Equality at a limit is a negative case. |
| Endpoint/partial selection extrapolates beyond intended region | Keep polygon 3A complete-loop-only; design explicit termination separately. |
| Modified convex body mistaken for unsupported nonconvexity | Distinct source-family diagnostics; sequential screenshot/control regression. |
| Local exterior cut damages a distant boss/pocket | Explicit influence-region cut and global difference/locality checks; global half-spaces forbidden as universal nonconvex solution. |
| Sweep self-intersection or Boolean tangency | Pure detached preparation, existing typed failures, independent cutter/result validation; no auto-union. |
| Mixed/unequal junctions leave holes or overcut | Joint neighborhood solve, shared boundaries and collision tests; defer unsupported combinations. |
| Face winding, reflected frames, multiple shells, vertex pinches | Oriented material-side checks, topology + vertex links + geometric intersections; reflection tests follow actual transform support, not assumed rigid-map behavior. |
| Large coordinates, tiny cuts, f32 STL collapse | Local arithmetic, named budgets, decoded-output certificate and safe precision refusal. |
| Output triangulation changes on save/load/refaceting | Re-certify actual output; semantic geometry comparisons rather than byte/diagonal assumptions. |
| Reordering or radius edit loses references | Versioned provenance and fail-closed resolution; immutable source and detached all-or-nothing regeneration. |
| Dense paths freeze UI or exhaust WASM memory | Preflight complexity limits, measured worst cases; do not silently reduce quality. |
| Feature history conflicts with direct modeling/upstream plans | Separate recipe decision and maintainer coordination before schema/API implementation. |
| Manufacturing trust exceeds proof | Report untested regimes and CAD-vs-CAM distinction; actual VDM/source-unit fixtures required. |
| Upstream overlap or licensing contamination | Pinned review, scoped integration branch only after approval, original algorithms/compatible licenses, no SketchUp SDK material. |

## 12. Upstream Hew Contribution Compliance

### Inspected upstream and branch relationship

Official repository: [hew3d/hew](https://github.com/hew3d/hew). Current `main` inspected through GitHub API at **`f23fc98bd79cb24b617f93ac96ad0601fe4a3c42`**. The fork's common upstream base is **`b2e5baee8b0b84d7eb3d8c8f26ff36aabf7fa42e`**. Upstream is one commit ahead of that base (`feat(app): infer the drawing plane from the view`); the fork has 12 independent commits after it. The local upstream tracking ref remains at the base: it was not mistaken for current upstream and was not updated by a branch synchronization.

GitHub comparison lists 44 upstream-changed paths. Four overlap paths changed by the fork: `app/src/viewport/Viewport.tsx` and the three locale catalogs. Additional semantic integration risk exists in `SnapConstraint.fallbackPlane`, `Tool.followsViewPlane`, `SnapService.resolve`'s new fallback argument and held-pointer hover behavior. No Rust-kernel change appears in that upstream commit. This is a path/patch assessment, not a tested merge or a claim of conflict-free integration.

The upstream recursive tree was nontruncated; no `AGENTS.md` or pull-request template was found. `CLAUDE.md` supplies agent instructions. At the pinned upstream revision, `CONTRIBUTING.md`, `README.md`, `CLAUDE.md`, `LICENSE`, `LICENSE-EXCEPTION`, `docs/dev/DEVELOPMENT.md` and `docs/agents/ARCHITECTURE.md` are byte-identical to the fork's copies. The detailed roadmap differs because of view-plane work.

### Actual documented requirements

Pinned references: [CONTRIBUTING.md](https://github.com/hew3d/hew/blob/f23fc98bd79cb24b617f93ac96ad0601fe4a3c42/CONTRIBUTING.md), [CLAUDE.md](https://github.com/hew3d/hew/blob/f23fc98bd79cb24b617f93ac96ad0601fe4a3c42/CLAUDE.md), [DEVELOPMENT.md](https://github.com/hew3d/hew/blob/f23fc98bd79cb24b617f93ac96ad0601fe4a3c42/docs/dev/DEVELOPMENT.md), [ARCHITECTURE.md](https://github.com/hew3d/hew/blob/f23fc98bd79cb24b617f93ac96ad0601fe4a3c42/docs/agents/ARCHITECTURE.md).

- Open an issue outlining changes larger than a small fix before implementation; geometry is spec-first.
- Fork/branch from main, focused PRs, kernel validator coverage/property tests and UI component tests. Kernel/UI mixed PRs will usually be asked to split.
- `scripts/verify.sh` must pass for upstream review; local guidance also calls for `verify-full.sh` before pushing. No test weakening, skipped invariants or warning suppression to obtain a green build.
- Kernel/inference/tessellate purity; discussed changes before new kernel dependencies, public WASM APIs, file format changes or cross-crate refactors. Lengths are f64 meters and geometric tolerances are named in `kernel::tol`.
- Serialization changes require `docs/dev/HEW_FILE_FORMAT.md` updates in the same commit; compatibility must be explicit.
- Conventional Commit titles, human author/committer, impersonal reviewable descriptions, no AI/tool attribution; DCO 1.1 sign-off on every commit. There is **no CLA**.
- Actual license is **AGPL-3.0-only plus the Hew Plugin API Exception**, not the shorter GitHub license-detection label alone. The exception concerns qualifying external plugins; it does not exempt these core modifications from AGPL obligations. `brand/` is separately CC BY 4.0. No SketchUp-SDK-derived material may enter the source or dependency chain.

Module-level Rust documentation, existing Scene candidate conventions, React tool state and CSS variables are observed conventions consistent with these rules. No mandatory universal SPDX-per-source-header format was found in the reviewed policy files; preserve existing notices rather than invent one.

### Relevant public development activity

Issues/PR enumeration covered 99 entries, supplemented by keyword search and reading issue/comment threads. Discussions are disabled on the inspected repository. Search absence is not proof that no unannounced implementation exists.

- [Issue 36](https://github.com/hew3d/hew/issues/36), exact-measurement UX, includes [a `granroth` response](https://github.com/hew3d/hew/issues/36#issuecomment-5983356197) explicitly discussing a future roundover feature and distinguishing it from drawing a rounded slot. This supports keeping boundary treatment and surface/profile construction distinct.
- [Issue 60](https://github.com/hew3d/hew/issues/60), plugin API/reference plugin, includes [a `granroth` comment](https://github.com/hew3d/hew/issues/60#issuecomment-6512751844) identifying fillets as a planned first-class Hew feature. The thread also discusses geometry writes as undoable commands and viewport tools. Its relative schedule is historical context, not a promised delivery date.
- No dedicated generalized Edge Treatment module was located in the inspected upstream tree; existing Follow Me subtraction and slice/chamfer examples already cover adjacent functionality. Maintainer coordination is therefore important: do not assume this fork's design matches ongoing unpublished fillet work or that it will be accepted.

No upstream issue, PR, discussion message, merge, rebase or force-push was performed.

### Readiness classification and corrective actions

| Area | Classification | Evidence / action |
|---|---|---|
| Existing Rust/WASM/React architecture and explicit object ownership | **Compliant** within reviewed scope | Uses existing kernel, Scene and document action; no alternate engine/framework proposed. |
| Dependency changes in Phase 1–2 fork | **Compliant** within reviewed scope | No Cargo manifest/lock or JS package/lock delta from the upstream base. No new dependency proposed for 3A. This is not a complete transitive-license audit. |
| DCO and contributor identity | **Compliant** within reviewed scope | All 12 fork commits inspected have human attribution and sign-off. Human contributor remains responsible for understanding and licensing the contribution. |
| License/provenance | **Not yet evaluated** as a complete audit | License/exception and policy reviewed; no third-party implementation copied in this investigation. Review every incorporated algorithm's actual provenance before submission; retain notices and audit any later dependency. |
| Scope/review decomposition | **Needs adjustment** | `ebb06e4` combines kernel, bindings, UI, tests and build work across 27 files. Preserve that released alpha SHA; prepare focused upstream patches on a separate approved integration branch, not by rewriting the published branch. |
| New batch property coverage | **Needs adjustment** | Batch tests cover examples, order and one rigid transform. Existing scalar proptests do not substitute for generated batch corner/subset/scale/transform coverage required of new mutations. |
| Tolerance/certificate reuse | **Needs adjustment** | Export certificates repeat decoding/predicate logic and include inline `64 * f64::EPSILON` budgets. Review a shared, named precision policy rather than adding more literals or suppressing lints. No numerical behavior was changed here. |
| Complete verification and platforms | **Needs adjustment** | Phase 2 full native gate/desktop checks were incomplete due host storage and GTK/WebKitGTK prerequisites. Passing focused checks is not upstream full-gate compliance. Run supported CI/host gates; cross-platform native/WASM reproducibility remains to be established. |
| Recording versus native-format documentation | **Requires maintainer clarification** | The batch recording serialization is documented in `EDGE_TREATMENT_API.md`, but `HEW_FILE_FORMAT.md` was not changed for that command extension. Clarify the project's broad serialization-update rule; document recording compatibility without inventing an unnecessary native geometry-version bump. |
| Proposed feature history / naming | **Requires maintainer clarification** | Upstream explicitly favors direct modeling without a parametric feature tree. Editable recipes need agreement on scope, naming, versioning and interaction with direct edits before implementation. |
| Planned fillets and new public interfaces | **Requires maintainer clarification** | Public comments announce first-class fillet intentions; user approval in this fork is not evidence of maintainer agreement. Discuss the proposal and reusable API boundaries before upstream work. |
| Synchronization | **Not yet evaluated** by build/merge | One upstream commit and overlap paths identified; no trial merge or semantic/browser regression run against upstream was performed. |

### Safe upstream strategy and proposed PRs

Keep `feature/edge-treatment` and the verified alpha recoverable. After user approval and authorized maintainer coordination, use a separate integration branch based on the agreed upstream revision. Repackage reviewed changes without changing the published alpha history. Resolve Viewport/snap semantics explicitly; regenerate catalogs from merged source rather than hand-merging generated entries. Re-run kernel, replay, actual-WASM, browser and supported desktop gates. Do not automatically merge/rebase this feature branch during investigation.

Proposed contribution boundaries, with meaningful specs/tests/docs in each:

1. Small reusable boundary/validation utilities and deterministic fixture infrastructure, with no new UI or behavior expansion.
2. Convex polygonal cap chamfer planner and independent geometry/export acceptance.
3. Constant-radius cap roundovers, miter policy and actual-output error certificate.
4. Document/recording adapters and the smallest approved WASM changes, retaining legacy commands and atomicity.
5. Frontend selection/diagnostics/quality help integration and browser lifecycle tests, following agreed backend interfaces.
6. Sequential certified-cap-band support, then separately any approved naming/recipe/persistence proposal.
7. General nonconvex validation/local cutting and later corner/curved-surface work, each separately scoped.

Existing Phase 1–2 kernel/document/API work is a prerequisite contribution to discuss and decompose; these PRs cannot assume an upstream Edge Treatment API that is not there. Fork-only alpha identity/handoff and incidental build improvements should not be bundled into an unrelated geometry PR. Tests and documentation belong with their behavior, not in a delayed catch-up PR.

Questions for maintainers: Is there an existing fillet design/branch to build upon? Are G0 perimeter miters acceptable as an explicit first capability? Which reusable predicates should be public within the kernel? Is evaluator-versioned optional treatment intent compatible with direct modeling? What is the approved recording/schema policy? Which cross-platform gates and manufacturing claims are expected? No upstream acceptance or universal compliance is claimed.

## 13. Preservation and approval gate

Leave the Phase 2 evaluator paths, APIs, recordings, tooltips, units, face-perimeter selection, multi-selection, alpha app ID `com.hew3d.Hew.EdgeTreatmentAlpha`, caches, models and isolated state unchanged. The Fedora checkout remains `~/src/hew-edge-treatment-alpha`; installed Hew is outside this work. New strategies must fail closed and preserve current refusals until their Category B success criteria pass.

Review decisions requested: one shared control/range/evaluation foundation for sequential, polygonal and circular/mixed boundaries; source authority compatible with True Curves; geometry-owned honest limits, precise editing and responsive accurate jobs; explicit G0 miter policies; sequential baked cuts before optional recipes; and upstream coordination before API/persistence changes. The coordinated milestone order is in the unified design, superseding the earlier linear sequence. This pass writes review documents only. No implementation, dependency change, branch synchronization, commit or push is part of this delivery.
