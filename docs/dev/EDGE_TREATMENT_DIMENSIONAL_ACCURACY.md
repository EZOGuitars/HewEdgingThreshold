# Edge Treatment dimensional accuracy verification

> Historical baseline report for `34a889c`. The
> [pre-Fedora stabilization report](EDGE_TREATMENT_STABILIZATION.md) supersedes
> the STL preparation policy and verification status. Original measurements are
> retained for reproducibility. The baseline test-typecheck PASS claim was
> incorrect: the preserved log contains TS2783 for a duplicate measurement key;
> stabilization removes that redundant key and reruns the check explicitly.

## Result and scope

The straight, perpendicular convex-edge chamfer and faceted circular roundover
passed independent dimensional measurements at the tested metric and imperial
scales. This is experimental geometry, not a production-readiness claim.
Curved paths, concave fillets, and new UI/geometry frameworks are outside scope.

There are **365 fresh-source native operations: 312 roundovers and 53 chamfers**.
The [complete measurement CSV](EDGE_TREATMENT_METRIC_MEASUREMENTS.csv) records
requested dimensions, independently recovered tangent setbacks, all three outer
and exported dimensions, absolute/relative errors, chord deviations, analytic
and measured volumes, and topology results. Lengths and volumes have explicit
millimeter/inch and mm³/in³ columns. A chamfer's radius columns mean its equal
setback distance; its chord/segment columns are zero (not applicable).

The [real-WASM measurements](EDGE_TREATMENT_METRIC_WASM.json) separately cover
unit conversion, committed geometry, STL import/export, all five chord tolerances, and repeated
switching. The WASM JSON also records independently measured chord edges,
all three outer dimensions, manifold/orientation checks, and ideal/faceted/STL
volumes, including measurements after each explicit external-unit round trip.
Neither converted parameters nor operation-reported volume/chord metadata are
used as proof of geometric accuracy.

## Reference matrix

- Metric cube sides: **10, 25, 100, 300, 1000 mm**.
- Main radii: **0.5, 1, 2, 3, 5, 6, 10, 12, 20, 25, 50 mm**.
- Additional metric equivalents: **6.35 and 12.7 mm** on each eligible cube.
- Decimal cases on separate 100 mm cubes: **0.1, 0.25, 0.5, 1.25, 2.54,
  6.35, 12.7 mm**.
- Imperial: fresh **12 × 12 × 12 inch** cubes (304.8 mm), radii **0.125,
  0.250, 0.375, 0.500, 0.750, 1, 2, 4 inches**.
- All matrix roundovers use each chord tolerance: **0.01, 0.025, 0.05, 0.1,
  0.25 mm**. Additional decimal cases use 0.01 mm.

Matrix combinations are deliberately restricted to `radius <= side/2` to keep
ample clearance. This is a conservative test-selection rule, not the kernel's
maximum-radius rule: some larger radii could also be geometrically valid.
Separate executable tests request radii equal to and beyond a 10 mm cube's
width; both refuse with `InsufficientClearance` and preserve the source.
Every operation constructs a fresh cube; no test chains treatments.

| Cube side | Native operations | Largest STL dimension/setback error, mm |
|---|---:|---:|
| 10 mm | 30 | 0 |
| 25 mm | 54 | 0.000000095367431 |
| 100 mm | 85 | 0.000000190734862 |
| 300 mm | 78 | 0.000000190734862 |
| 1000 mm | 78 | 0.000000190734862 |
| 12 in = 304.8 mm | 40 | 0.000012207031275 |

These are measured dataset maxima, not universal guarantees for arbitrary
coordinates, shapes, rotations, or magnitudes.

## Independent measurement methods and tolerances

`crates/mesh-export/tests/metric_accuracy.rs` constructs profile/extrusion cubes,
uses an independently identified corner edge, then reads **actual resulting
BRep vertices and edges**. It recovers tangent setbacks on the original two
side planes, measures coordinate extrema, and measures each arc chord's closest
point and endpoints relative to the reference circle. It counts the actual
chords rather than trusting a returned chord-error value.

BRep volume is independently integrated as oriented tetrahedra from polygon
loops about an interior witness. STL is independently decoded from binary bytes;
its volume is integrated separately from its triangles. Every matrix export
checks triangle count against the tessellated source, nonzero triangle area,
outward winding and stored-normal agreement, and exactly one reverse-oriented
partner for each directed edge. Native topology validation and watertightness
are also required for every case. No facets are silently discarded.

The BRep dimensional acceptance budget is `max(1e-11 m, side * 1e-10)`. All
axis-origin BRep tangent/extent measurements happened to have zero residual
against the requested **f64** reference coordinates. This does not mean decimal
real numbers are exactly represented in binary, or that all rotated/translated
models have zero error. Prior rigid-transform tests remain relevant.

STL and real-WASM mesh acceptance use `8 * f32::EPSILON * side` in meters,
reflecting float32 serialization. This budget is 0.000290679932 mm on a 12-inch
cube. The measured errors below are substantially smaller. Conversion comparison
allows 2e-17 m (2e-14 mm), separately from geometry/mesh tolerances.

For BRep removed-volume comparison, the numerical acceptance budget is
`expected_faceted_removal * 1e-7 + 128 * f64::EPSILON * source_volume`.
The STL total-volume budget is `source_volume * 2e-6`. Both bounds are documented
because cancellation in small removed volumes differs from whole-solid error.

## Chord deviation

The independent reference for a quarter-circle with N equal chords is:

`e = R * (1 - cos(pi / (4N)))`.

| Requested maximum, mm | Largest measured BRep chord deviation, mm |
|---:|---:|
| 0.01 | 0.00999218122719 |
| 0.025 | 0.0246719817134 |
| 0.05 | 0.0485372166747 |
| 0.1 | 0.0967006872833 |
| 0.25 | 0.246233188097 |

The largest discrepancy between actual BRep chord measurements and that formula
was **1.55041e-14 mm**. The largest STL chord deviation was
**0.246233358054 mm**. Every native-matrix STL chord stayed below its own requested
tolerance, even before adding the allowed float32 budget. Coarse chord deviation
is intentional approximation and is not a radius/setback or conversion error.

The opposite-corner real-WASM STL tests expose a separate float32 limit:
**4-inch (101.6 mm) radius / 0.01 mm chord** measures
**0.010002231570536 mm**, exceeding the strict request by
**0.000002231570536 mm**. Its explicit-unit reimports retain that excess.
This passes the stated float32 numerical budget but **does not satisfy the
strict STL chord bound**. The JSON marks this with
`strict_stl_chord_compliant: false` and records the excess. The native f64
matrix remains compliant; its 101.6 mm radius case measures 0.009992181227186 mm.
At the required 6.35 mm opposite corner, STL chord deviation is
**0.009997820746607 mm**, still below 0.01 mm. Do not extrapolate the native
origin-corner STL result to every placement. No silent geometry change was made
to hide serialization quantization.

All five requested tolerances succeeded in the native matrix; at most **56** of the
allowed **128** chords were used. A separate **50 mm radius / 0.0001 mm error**
request exceeds the segment cap and correctly returns `ApproximationTooFine`.
It is an expected refusal, not a successful tolerance-compliant treatment.
No limit is silently clamped.

## Ideal volume, faceting, and numerical error

An ideal circular cut removes `L * R² * (1 - pi/4)`. For N inscribed chords,
the expected actual removal is
`L * R² * (1 - N*sin(pi/(2N))/2)`.
The difference is **deliberate faceting excess**, reported separately from
measured-minus-expected numerical error. Chamfer removal is `L * d²/2`.

Across all native cases, maximum independent BRep removed-volume error was
**4.10181e-7 mm³**; maximum relative error was **1.35334e-9**. Decoded STL total
volume differs from ideal-source-minus-faceted-cut by at most **3.40220 mm³**.
Much of that is whole-cube float32 coordinate quantization on the 304.8 mm cube,
not an error in unit conversion or a missing cut facet. Comparing independently
exported source and result STL volumes instead isolates cut error: its maximum
was **0.0326116 mm³**. Both comparisons are included in the CSV; the larger
whole-envelope error is not concealed.

For the required **12-inch cube, 0.250-inch / 6.35 mm radius, 0.01 mm chord**:

| Quantity | Independently measured / reference |
|---|---:|
| Native tangent setbacks | 6.35 / 6.35 mm = 0.25 / 0.25 in |
| Native outer dimensions | 304.8 × 304.8 × 304.8 mm |
| Actual chords | 14 |
| Actual BRep maximum chord deviation | 0.009989724636865 mm |
| Ideal circular removal | 2637.520523193 mm³ |
| Expected faceted removal | 2657.760562935 mm³ |
| Measured BRep removal | 2657.760562949 mm³ |
| Deliberate faceting excess | 20.240039743 mm³ |
| BRep removed-volume numerical error | 1.38113e-8 mm³ |
| STL outer dimensions | 304.799987792969 mm on each axis |
| STL maximum chord deviation | 0.009989792977206 mm |
| STL cut-volume error relative to exported source | 0.0000610242 mm³ |

Binary-representation tails in the CSV are retained rather than being confused
with the rounded presentation above.

## UI/WASM conversions and display correction

The conversion definition is **1 inch = exactly 25.4 mm**. The existing unit
parser implements meters-per-inch as 0.0254; the kernel/WASM length unit remains
f64 meters. Tests exercise all seven requested conversion pairs plus
4 inches = 101.6 mm, for **both chamfer and roundover**, through the actual
`EdgeTreatmentTool` and generated Node/WASM bindings on fresh 12-inch cubes.
They commit the result and independently decode the exports.

Maximum conversion difference was **3.46945e-15 mm**, within floating-point
roundoff. Equivalent imperial and metric STL vertex coordinates were identical
in every pair (maximum difference **0 mm**). This equivalence result is
accompanied by separate setback and dimension measurements, not substituted
for them.

For the 0.250-inch / 6.35 mm roundover at the opposite corner of the real-WASM
cube, exported tangent setbacks measured **6.349987792969 mm** and
**6.349999904633 mm**. Maximum absolute error was **0.000012207031252 mm**
(**4.80592e-7 inches**), relative error **1.92237e-6**. Moving the cut to the
opposite side exposes float32 subtraction/placement effects that the native
origin-corner tests alone would miss.

All seven decimal metric values underwent **100 metric→imperial→metric cycles**
with zero change in the canonical WASM radius. The largest measured decimal
setback error was **0.000012207031276 mm**; for the smallest 0.1 mm radius its
relative error reached **0.0001220703123** (0.012207%). This is export precision,
not cumulative conversion drift.

The display defect is corrected solely in `app/src/tools/EdgeTreatmentTool.ts`:
unit switches format the selected unit using the existing formatter while
retaining canonical meters per field. An actual input edit replaces that cached
value. Formatting never feeds its rounded text back into the geometry. Fields
use the existing exact-format display policy, independent of rounded readout
preferences; a separate high-precision test proves that the formatter's finite
precision ceiling cannot silently change the actual WASM dimension. The UI now
shows `6.35 mm` → `0.25"` → `6.35 mm`. The inch glyph follows the existing
formatter's ASCII quote convention. No kernel or WASM API change was made.

A real Chromium pointer/React test repeats switching 100 times, commits, and
measures the STL. Native rendering remains unverified on WebKitGTK/Fedora.

## STL external units and import/export

STL contains no physical-unit metadata. Hew's exporter emits **millimeter
coordinates** regardless of the UI display preference. The real-WASM tests import
those exports with an explicit **0.001 meters per coordinate unit**, re-export,
and remeasure. They also encode an inch-coordinate copy of each STL and import
with **0.0254 meters per coordinate unit**, then remeasure. Both paths preserve
intended physical dimensions within the float32 budget. No accidental 25.4×
scaling was observed. The inch-encoding test itself uses float32 and therefore
has its own rounding; it does not claim inch-unit STL metadata exists.

Users must tell an external slicer/importer the coordinate units. UI unit
preference does not change this STL contract.

## Reproduce

Use the pinned tools and actual Node WASM build in the
[Fedora guide](FEDORA45_EDGE_ALPHA.md). No dependency or lockfile changes.

```bash
cargo test -p mesh-export --locked --test metric_accuracy -- --nocapture
pnpm --dir app test src/tools/EdgeTreatmentMetric.wasm.test.ts \
  src/tools/EdgeTreatmentTool.test.ts src/tools/EdgeTreatmentTool.wasm.test.ts \
  src/panels/EdgeTreatmentPanel.test.tsx src/settings/units.test.ts --maxWorkers=2
```

The native command emits `METRIC_HEADER,` and `METRIC,` CSV lines. The WASM
suite emits `METRIC_WASM,` and `METRIC_DECIMAL,` JSON lines. The committed CSV/JSON
are measurements from executed runs, not invented expectations. Dedicated WASM
verification requires the generated Node package to exist; the ordinary suite
skips these integration tests when it is absent, which must never be reported
as a real-WASM pass. The final handoff reports PASS, FAIL, BLOCKED, and NOT RUN
separately, including the disk-exhausted broad verification attempt.
