# Phase 3 deterministic regression and acceptance matrix

Modularity constraint: see [the source-level modularity audit](EDGE_TREATMENT_MODULARITY_AUDIT.md). Retain dedicated feature strategies and adapters; shared-core extensions must be minimal, generic where justified, and reviewed before implementation.

This is a **test-plan specification**, not an implemented or executed new suite.
Architecture, source citations and approval gates are in [the engineering review](EDGE_TREATMENT_PHASE3_ARCHITECTURE.md). Current baseline is `ebb06e4f4a049a9a0c08f7d9acbe09d5c5482816`.

## 1. Classification and execution rules

- **A — Existing behavior:** assert successful geometry and established lifecycle/compatibility contracts, or an explicitly existing failure contract. Preserve the known working box evaluator and scalar operations. A negative/refusal regression is labeled negative, never counted as a successful geometry capability.
- **B — New milestone acceptance:** every claimed supported operation must succeed and pass independent validation. A safe refusal is a failed B acceptance case.
- **C — Future capability:** deterministic source/selection/parameters plus the baseline's specific expected refusal. Report supported-success count as zero until promoted to B. Do not use a catch-all `is_err()` to hide panic, wrong error class or broken source construction.

Each record must contain fixture ID, stored profile coordinates/curve metadata, extrusion/frame, operation/selection, units, requested lengths/error, classification and milestone, baseline/current outcome, exact refusal, measured validity/accuracy and timing. Missing certification is a separate field. Fixture construction failure is not an operation refusal.

The matrix has coordinated acceptance slices. The first polygon gate is the four requested convex profile families and irregular/transformed variants; the upcoming foundation also targets Q01/Q02 sequential cuts and C01/C04 genuine circular/mixed boundaries through the same controls/evaluator interfaces. See the [unified control/engine contract](EDGE_TREATMENT_CONTROL_ENGINE_DESIGN.md). Other C rows remain future work, not a mandate to implement everything in one pass.

## 2. Fixture definitions

All coordinates below are millimeters in a local CCW profile plane. Unless specified otherwise, extrude 40 mm. Convert to kernel meters once, construct via `Profile::new` and `Object::from_extrusion`, and assert source validity before treatment. Region/curve fixtures must go through attributed Sketch profiles, not fabricate circle metadata on straight edges.

| ID | Deterministic profile / source | Baseline and proposed promotion |
|---|---|---|
| O01 | Square `(0,0),(100,0),(100,100),(0,100)` | A boxes; first 3A required acceptance |
| O02 | Rectangle `(0,0),(100,0),(100,60),(0,60)` | A boxes |
| O03 | Same rectangle, H=23; separate 100×60×20 and 300×200×100 boxes | A unequal dimensions |
| O04 | 100×60 rectangle, H=0.25; cuts 0.025/0.05 mm and separately excessive values | A valid thin cases + negative cases; no automatic radius reduction |
| O05 | Rigid transforms of O01–O04, frames below | A frame preservation |
| T01 | Equilateral `(0,0),(100,0),(50,86.6025403784)` | Complete cap C → B in 3A extended coverage |
| T02 | Isosceles `(0,0),(100,0),(50,70)` | Same |
| T03 | Scalene `(0,0),(110,0),(30,70)` | Same; qualify scalar eligibility per edge |
| Z01 | Isosceles trapezoid `(0,0),(120,0),(90,60),(30,60)` | C → first required B in 3A |
| Z02 | Right trapezoid `(0,0),(120,0),(80,60),(0,60)` | C → B; freeze any existing eligible scalar vertical-edge behavior |
| Z03 | Asymmetric trapezoid `(0,0),(120,0),(95,60),(15,60)` | C → first required irregular B |
| P05 | Regular pentagon, circumradius 50, angular phase 0.17 rad | C → extended B |
| P06 | Regular hexagon, same radius/phase | C → first required B |
| PI06 | Irregular convex hexagon, construction below | C → first required irregular B |
| P08 | Regular octagon, same radius/phase | C → first required B |
| PI08 | Irregular convex octagon, construction below | C → first required irregular B |
| P07/P10/P12/P16 | Regular convex 7/10/12/16-gons, same radius/phase | C → extended B, same algorithm |
| PI07/PI10/PI12/PI16 | Irregular convex polygons, construction below | C → extended B, no side-count-specific cases |
| C01 | Attributed full circle, diameters 10/25/100/300 mm; 24/48/96/192 source facets | C at baseline → B target in the coordinated curved slice, after source-envelope and patch-certificate contracts |
| C02 | Same circle stations **without** analytic claims, explicitly a polygonal surrogate | Baseline batch refusal; polygon success must be labeled polygon-only, not C01 curved acceptance |
| C03 | Ellipse, semiaxes 50/30 mm, fixed sampled stations at those four densities | C for ideal ellipse treatment; carrier is polygonal unless new metadata is approved |
| C04 | Rounded rectangle, outer 100×60, corner radius 10; attributed tangent straight/arc segments | C at baseline → B target for the coordinated convex tangent mixed-boundary slice |
| C05 | Rounded rectangle with unequal arc radii 5/10/15/10, valid fixed tangent construction | C mixed curvature; source validity checked first |
| N01 | L `(0,0),(100,0),(100,30),(30,30),(30,100),(0,100)` | C nonconvex |
| N02 | Notched rectangle `(0,0),(120,0),(120,80),(75,80),(75,40),(45,40),(45,80),(0,80)` | C nonconvex |
| N03 | Concave hexagon `(0,0),(100,0),(100,80),(60,35),(20,80),(0,80)` | C reentrant corner |
| N04 | Irregular reentrant loop `(0,0),(120,0),(110,90),(75,55),(60,95),(30,45),(0,80),(-10,40)` | C; certify simple source before accepting fixture |
| N05 | Narrow-U family defined below, web w=0.25/1/4 and gap g=0.25/1/4 | C feature-size stress; dimension table stored with generated coordinates |
| N06 | Double-notch family defined below, g=1/2/5 | C closely spaced transitions; reject invalid fixture construction separately |
| H01 | 120×80 profile with centered 20×10 hole, correct CW inner loop | C inner-perimeter/hole semantics |
| S01 | 100×100×20 base; centered 40×30×15 rectangular boss | C globally nonconvex |
| S02 | Same base, centered regular hexagonal boss (R=20, H=15) | C; existing attached-boss reproduction retained |
| S03 | Same base, centered regular octagonal boss (R=20, H=15) | C |
| S04 | Same base, two 20×20×15 bosses centered at (30,50)/(70,50); narrow versions centered at (40,50)/(60+g,50), g=0.25/1/4 | C multiple features/collision |
| S05 | Same base with 30×20 recess, depths 5/15, and separately through-hole | C pocket/inner-wall cases |
| S06 | Separate unmerged hexagonal prism over base | Ownership control: two objects, not one merged solid |
| Q01 | O03 after one committed bottom perimeter treatment | Confirmed C now; mandatory B in the coordinated sequential slice |
| Q02 | Box with disjoint top chamfer and bottom roundover, unequal dimensions | C now → B in the coordinated sequential slice |
| Q03 | Different sizes/types meeting at a common vertex or overlapping band | C future joint solver; cannot pass by independent result concatenation |
| F01 | Synthetic tapered blank: L=500, nut width=43, heel width=57, center thickness=6, radius=304.8 mm | C separate constant-radius surface generator; not an asserted VDM dimension set |
| F02 | Same blank, radii 241.3 and 406.4 mm (9.5 and 16 inches) | C same surface contract |
| F03 | Same widths with center thickness 0.5 mm, plus widths approaching/exceeding 2R | C explicit thickness/domain negatives, no clamping |

Explicit concave generators (each path closes to its first point):

- N05: `(0,0),(2w+g,0),(2w+g,40),(w+g,40),(w+g,w),(w,w),(w,40),(0,40)`. This independently controls arm/bottom web thickness and internal gap.
- N06: `(0,0),(100,0),(100,80),(65+g,80),(65+g,40),(60+g,40),(60+g,80),(60,80),(60,40),(55,40),(55,80),(0,80)`. Two 5 mm notches leave a controlled intervening web.
- C04/C05: use a 100×60 rectangle and bottom-left/bottom-right/top-right/top-left corner radii as specified. Connect straight tangent segments to CCW quarter arcs centered one local radius from each corner; use 6/12/24/48 samples per quarter and exact shared cardinal endpoints. Store per-run circle attribution, including distinct unequal-radius runs.

For regular polygons use ordered angular samples. For irregular convex fixtures, take positive weights `w_i = 1 + 0.12*((7*i mod 5)-2)`, angles `theta_i = 0.17 + 2*pi*sum(w_0..w_(i-1))/sum(w)`, and points `(50*cos(theta_i), 35*sin(theta_i))`. Ordered distinct points on an ellipse produce a strictly convex polygon with varying edge lengths/angles. Generate once, review and store canonical decimal coordinates and expected source measurements; cross-platform trig differences must not masquerade as evaluator nondeterminism. No random unseeded radius perturbation is allowed as a claim of guaranteed convexity.

C03 can also be obtained by nonuniformly scaling a circle extrusion in its cap plane; assert that unsupported circle/cylinder ancestry is dropped as documented. Do not add fake circular metadata to preserve a nonexistent exact ellipse claim.

For S01–S05, use the actual `Document::apply_object_op` with `KernelOp::SplitFaceInner` and `KernelOp::ExtrudeSubFace` pathway, as in the current polygon tests, or the corresponding established operation variants. For pockets, verify actual negative extrusion/Boolean semantics and one connected source. A pair of intersecting unmerged objects is not the boss/pocket fixture.

## 3. Frames, dimensions and parameter coverage

For the required 3A fixtures run:

- Identity, extrusion-axis permutations through X/Y/Z, rotation about normalized `(1,2,3)` by 0.67 rad, and another rotation placing neither cap nor edge on an axis.
- Translation `(30,-20,10)` mm and `(300,-200,100)` mm; additional large-coordinate cases are separate precision/refusal probes.
- Uniform scales producing 10/25/100/300/1000 mm characteristic sizes; derive valid heights/radii explicitly. Nonuniform or shearing transforms must be reclassified from their resulting geometry; do not assume they preserve a right prism or analytic curve.
- Lengths 0.1/0.25/0.5/1/1.25/2/2.54/3/5/6/6.35/10/12/12.7/20/25/50 mm where clearance permits. Every operation starts from a fresh source unless labeled sequential.
- Chord errors 0.01/0.025/0.05/0.1/0.25 mm, plus near-boundary precision/cap cases. A requested valid combination that exceeds a documented work cap is a reported capacity refusal, not an accurate successful result.
- Equivalent imperial pairs `0.125/0.25/0.375/0.5/0.75/1/2 in` ↔ `3.175/6.35/9.525/12.7/19.05/25.4/50.8 mm`; independent measurement, repeated display-unit switching and no cumulative conversion drift.

For angle stress add valid polygons with very acute and nearly collinear corners, and run cuts on both sides of the named miter/collapse threshold. These are explicit B successes inside the certified domain and exact negative cases outside it. Do not silently remove a tiny edge or merge collinear vertices.

## 4. Operation grid and claimed outcomes

| Operation | Existing boxes | First polygon milestone | Later capability |
|---|---|---|---|
| One upper/lower edge, chamfer/roundover | A success for eligible requests | Preserve legacy behavior; new oblique termination not claimed | Explicit finite termination |
| One extrusion-direction edge | A success, including X/Y/Z after frame changes | Classify actual dihedral; a non-right polygon corner is not automatically eligible | Arbitrary-dihedral solver |
| Two adjacent / three incident edges | A documented miters/spherical corner policy | C unless existing scalar/box path applies | General corner policy |
| Complete upper cap perimeter | A | O01 is the A control; required B: Z01–Z03, P06/PI06, P08/PI08 and their transformed variants | Coordinated circle/mixed slice; concave loops remain later |
| Complete lower cap perimeter | A | Opposite-frame B parity gate before claiming support | Mixed/opposing interactions |
| Two disconnected / partial perimeter sets | A | C general termination; do not extrapolate full-loop proof | Local trim/termination planner |
| All edges | A for valid box dimensions | C arbitrary polygon vertical dihedrals/junctions | General network treatment |
| Upper and lower treatments sequentially | Confirmed batch refusal after first; negative preservation contract | Required B in the coordinated sequential slice | Intersecting/mixed junctions remain gated |
| Hole loops / multiple loops / entire surface | C | C | Distinct loop and surface contracts |

Scalar eligibility is per-edge. Do not label every edge of a nonrectangular prism unsupported just because the batch recognizer refuses it. During fixture qualification, record native baseline outcomes and preserve any already-working perpendicular scalar cases. Only the specified Category B successes count toward the new milestone.

Every successful geometric record runs chamfer and roundover independently. Complete-perimeter acceptance is not satisfied by repeatedly committing individual-edge results.

## 5. Sequential acceptance: exact Fedora regression and mixed treatments

**Q01-Fedora:** preserve the supplied observation: successful bottom-face perimeter; four selectable upper edges; second roundover radius `0.0125 in = 0.3175 mm`, maximum chord `0.0039 in = 0.09906 mm`; UI reports the unmodified rectangular-solid restriction. The first radius and original dimensions are unknown from the supplied description and must not be invented.

**Q01-reproducer:** fresh 100×60×20 mm box → bottom perimeter R=2 mm / chord=0.05 mm → commit → re-pick four upper edges → second request R=0.3175 mm / chord=0.09906 mm. Actual baseline Node WASM returns `UnsupportedBox`, with identical saved bytes and undo depth after refusal. A scalar upper-edge control prepares/discards successfully. Keep this safe refusal as a negative regression until the sequence becomes Category B in the coordinated sequential slice (originally 3B1).

The coordinated sequential slice (the original 3B1 target) requires the sequence to succeed with both independent dimensions retained, one action per commit, no alteration below the new top influence band and no regression to the existing bottom geometry. Also require:

1. Opposite-face roundovers of different sizes, both evaluation orders.
2. Upper roundover `0.25 in = 6.35 mm`; lower chamfer `0.125 in = 3.175 mm`, with sufficient thickness.
3. Chamfer → roundover → different-distance chamfer on separately certified regions.
4. Undo third/second/first independently, redo, save/reopen, replay from the matching source, and export each accepted state.
5. Adjacent/overlapping bands, vanished caps and insufficient thickness refuse atomically until their junction policy is supported.
6. A direct unrelated edit is retained; any future recipe regeneration must not silently overwrite it.

These tests do not establish editable feature history. Recipe edit/suppress/reorder gets its own B gate with naming ambiguity, evaluator-version and downstream dependency tests.

## 6. Independent validation for every B success

Use at least three distinct layers: native topology/geometry predicates; an independent analytical/geometric oracle; decoded actual export checks. Reusing the production planner's output as the expected answer is not independent.

- Check source/result finiteness, directed loops, manifold edges and vertex links, one intended component, outward winding, positive local-origin signed volume and nondegenerate triangles.
- Test improper face/triangle contact beyond legitimate shared topology. Watertight flags, two edge incidences and Euler characteristic alone do not rule out a self-intersection or pinched vertex.
- Validate every output vertex against original convex supports for 3A, and independently verify unchanged geometry below/outside the influence band. Future nonconvex tests require region-limited differences and general solid membership.
- Compute profile area/perimeter/angle coefficient independently; compare actual removed volume with the closed-form/piecewise-linear volume in the architecture report. Report ideal/faceted difference as tessellation error, not floating-point error.
- Measure radius/setbacks against ideal cylinders/planes. Include triangle-interior distance extrema where required; vertices on the ideal circle do not bound chord sag by themselves.
- Decode millimeter-float32 STL; check closure, vertex links, winding, intersections, dimensions, volume and maximum deviation. Test immediate export, save/load export and supported refaceting settings. Do not require identical triangle order/diagonals when geometry is equivalent.
- Compare native and actual WASM outcomes and measurements under documented named tolerances; exact selection permutations must be deterministic for one source and evaluator. Geometry equivalent under a rigid transform need not have identical world AABBs or byte order.
- Report requested dimensions, measured dimensions, absolute mm/inch error, relative error, maximum chord deviation, conversion error, STL dimensional error, expected/actual removal and actual segment/triangle count.
- STL has no embedded unit. Test external import scales explicitly: 0.001 meters per STL unit for Hew millimeters and a documented inch conversion path, without an unintended 25.4× factor.

Attribution-free sampled circles/ellipses need an independent ideal-outline distance budget before any continuous-curve accuracy claim. As density rises, record source-outline error and treatment error separately. No circular success is credited merely because every polygon facet got cut.

## 7. Negative, lifecycle and determinism coverage

For every relevant accepted family include zero, negative, NaN/infinite dimensions, extreme radii, collapse equality, first inset event, short edges, near-degenerate angles, extreme translations, self-intersecting source, open/inside-out/disconnected or vertex-pinched source, invalid/stale/foreign IDs, and excessive tessellation/work requests. Assert exact typed refusal and unchanged source, metadata, history and existing committed treatments. A failure during export certification must not publish a candidate.

Property tests should generate valid convex right prisms, safe dimensions and rigid transforms; vary side count without a side-count dispatch; permute selected edge order and source loop seam. Separately generate invalid requests around certified boundaries. Preserve seeds and minimized counterexamples. Add analogous generated batch-box coverage: existing scalar property tests are not enough for the new batch route.

Actual WASM and browser gates must exercise existing Ctrl/Command toggle semantics, same-solid restriction, face-perimeter selection, parameter/unit changes, debounce, old-handle retirement, cyan preview purity, Clear/Cancel/Escape/tool switch, annotation acknowledgment, stale Apply, one commit/undo/redo, persistence/replay and actual STL. Measure near-budget latency and memory. Current synchronous WASM does not promise mid-call cancellation.

Keyboard/hover/click help remains tested, including the stable icon pointer target, neighboring help-button access and no accidental viewport Enter-to-Apply. Cross-platform tests must include supported desktop/browser paths, not infer Fedora WebKitGTK behavior from Chromium alone.

## 8. Benchmarks and execution order

1. Re-run and freeze A before modifications, including old scalar/batch recordings.
2. Qualify deterministic sources and the sequential/polygon/circle/mixed selection fixtures together; report source failures distinctly.
3. Agree real geometry-owned limits, canonical editing units, range certainty, True Curves/source authority, quality and job lifecycle contracts before UI or evaluator specialization.
4. Develop the sequential, polygonal and circular/mixed B slices in tandem against that shared control contract, with native independent oracles and source-output-next-operation checks.
5. Extend polygon side-count/parameter/transform coverage without new side-count algorithms; keep all unsupported C cases explicit.
6. Run actual-WASM/browser lifecycle, precision, independent export and responsiveness gates across all three slices. Include the ~883 ms fine all-edge case and no stale/draft commit.
7. Promote Q01/Q02 and C01/C04 only when their success/certification criteria pass, including sequential use after save/load. No continuous-circle claim from C02.
8. Schedule other C families and editable feature recipes only after their additional dependencies and semantics are reviewed.

Performance rows: source vertices/faces, selected edges, section segments, planned planes, result triangles, preparation/validation/export time, peak allocation and preview latency. Compare warm identical, UI-only and geometry-change paths using preserved caches. Do not claim cold/Fedora timings from concurrent development-host builds.

VDM body-outline fixtures require licensed source geometry and critical neck/hardware references. Fretboard fixtures require length, nut/heel widths, thickness reference and 9.5/12/16-inch radii. Their absence is a missing benchmark input, not a passed test. A CAM import/toolpath/collision test remains separate from CAD/STL integrity.
