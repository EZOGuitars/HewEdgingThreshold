# Edge Treatment Fedora alpha — manual checklist

Use a separate experimental executable and state directory as described in
[Fedora setup](FEDORA45_EDGE_ALPHA.md). Test copies of models, not originals.
This is an alpha test of straight convex edges, not guitar-body perimeter work.

## Test record

| Field | Actual value |
|---|---|
| Tester/date | |
| Full Git commit / UI beta stamp | |
| Fedora release | |
| Wayland or X11 | |
| WebKitGTK version | |
| GPU/driver and any software-rendering notice | |
| Model copy location | |
| Diagnostic/reproducer location | |

For every row, record **Pass / Fail / Not tested** and brief actual results.

| # | Action | Expected result | Actual / notes |
|---|---|---|---|
| 1 | Start the alpha by its absolute path. | Separate alpha state; correct beta SHA; normal installed Hew is unchanged. | |
| 2 | If normal Hew is also running, start the alpha again. | Alpha stays independent; files are not forwarded to the installed copy. | |
| 3 | Create a 100 × 60 mm rectangle and extrude it 20 mm. Save a test copy. | One closed solid; ordinary save/reopen works. | |
| 4 | Name/color the solid and add a second, separate solid nearby. | Metadata and the second solid are visible for comparison. | |
| 5 | Activate Edge Treatment from Modify, then from Tools or Search. | One tool with Chamfer/Roundover modes; no unexpected modeling action. | |
| 6 | Pick the middle of an exposed straight vertical edge away from drawing axes. | Selected source edge is orange; actual candidate is cyan. | |
| 7 | Choose Chamfer, enter `2 mm`. | Preview removes only the corner; source remains unchanged until Apply. | |
| 8 | Change distance several times, including explicit `0.1 in`. | Old preview disappears immediately; Apply waits for the latest valid preview. Units are honored. | |
| 9 | Cancel. Inspect the model and Undo history. | Original geometry/metadata remain; no temporary object or new treatment operation. | |
| 10 | Preview again and switch to Select or another modeling tool. | Cyan preview and selected-edge cue are removed; no treatment committed. | |
| 11 | Apply a `2 mm` chamfer. | One modification on the same object; name/color preserved; second solid unchanged; old edge selection cleared. | |
| 12 | Undo, then redo several times. | Exact original/treated geometry alternates with no extra objects or progressive drift. | |
| 13 | Undo back to the box. Choose Roundover, radius `2 mm`, chord error `0.05 mm`. | Rounded subtractive corner preview; a faceted approximation, not a cylindrical groove. | |
| 14 | Change radius and chord error rapidly. | Latest candidate replaces earlier previews; Apply never commits obsolete input. | |
| 15 | Enter zero, a negative value, invalid text, and a nonfinite value. | Clear validation feedback, no clamping, Apply disabled, no geometry/history change. | |
| 16 | Request a radius larger than available clearance or an extremely fine tolerance. | Actionable refusal; source and unrelated geometry remain intact. | |
| 17 | Apply a valid roundover; select another eligible original edge and repeat. | Valid successive operations; newly slanted/curved rims may legitimately refuse. | |
| 18 | Save, close the alpha, restart, and reopen the test model. | Committed geometry, object metadata, and annotations survive. Preview state is not saved. | |
| 19 | Export STL and inspect it in a trusted independent viewer/slicer. | Watertight result with the expected corner; an unsafe export refuses rather than dropping triangles. | |
| 20 | Try a curved/concave/nonperpendicular edge. | Supported-scope explanation, no silent fallback or source mutation. | |
| 21 | Try an object with an attached/object-owned sketch. | Explicit rejection; sketch remains attached and unchanged. | |
| 22 | Try group/component editing, a hidden target, or drawing in progress. | Appropriate context refusal; no accidental definition edit. | |
| 23 | Add a source-linked dimension or leader annotation, then preview. | Warning lists the affected annotation IDs and explains retained points and Undo. | |
| 24 | Leave that warning unacknowledged; try Apply/Enter. | No commit. Acknowledgment is required. | |
| 25 | Acknowledge, regenerate Preview, and inspect the checkbox. | Acknowledgment resets for the new candidate. | |
| 26 | Acknowledge and Apply, then Undo/Redo. | Annotation detaches explicitly; recorded points remain; Undo restores attachments. | |
| 27 | During preview, edit/delete/undo the source or load another document. | Preview/selection invalidates or Apply refuses stale state; newer model is never overwritten. | |
| 28 | Wait through recovery autosave or save a changed camera view during preview, then Apply. | A conservative stale refusal is allowed; regenerate. No geometry corruption or duplicate operation. | |
| 29 | Repeat preview/Cancel/tool-switch cycles at least ten times. | No orphaned cyan mesh, stale target, or unbounded resource growth; no history pollution. | |
| 30 | Make a recording containing Apply, Undo, and Redo; replay where supported. | One modeling command per successful Apply; previews/candidates are not modeling commands. Record any explicit replay refusal. | |
| 31 | Restart after the previous actions. | Separate alpha recovery/preferences work; normal installation's state remains untouched. | |

## First-session dimensional acceptance

Before the longer stress/context checks, prioritize launch, tool visibility,
straight-edge selection, chamfer preview, roundover preview, measured dimensions,
unit switching, invalid inputs, single Apply, Cancel, Undo/Redo, save/reopen,
STL dimensions, repeated preview cleanup, and safe unsupported-geometry refusal.

Use **two fresh 12 × 12 × 12 inch cubes** (304.8 mm per side), not sequential
cuts on one source. On the first, request a **0.250-inch roundover** and
**0.01 mm maximum chord error**. On the second, request the equivalent
**6.35 mm roundover** with the same chord tolerance. Pick corresponding supported
edges. Check `6.35 mm` → `0.25"` → `6.35 mm`, repeat switching, and confirm no
changed model until Apply. Each Apply adds exactly one treatment; Cancel adds
none. Undo/Redo and save/reopen must preserve the intended result.

Independently inspect both exports as **millimeter-coordinate STL**. Confirm
304.8 mm outer dimensions, 6.35 mm tangent setbacks (within the documented
float32 precision budget), equal physical geometry, outward watertight facets,
and no 25.4× scaling. Expect a 14-chord quarter-circle at the chosen tolerance;
maximum chord deviation is about 0.0099898 mm, rather than a perfectly analytic
circle. Use the [measurement report](EDGE_TREATMENT_DIMENSIONAL_ACCURACY.md)
for numerical references and keep display rounding separate from measurement.
Record actual measurements and external viewer/slicer units; a visually rounded
corner alone does not pass dimensional acceptance.

## Report a failure

Write down the row number, exact steps, dimensions/units, selected edge, actual
message, commit/stamp, and whether the model changed. Include screenshots and a
copy of a minimal reproducing model only when sharing is appropriate. Preserve
the diagnostic log and recording before retrying. A message saying the treatment
**was applied but the view could not refresh** means the model committed; use
Undo or reopen it rather than applying the same operation again.

Do not treat a passed Chromium test as a passed WebKitGTK test. Curved guitar-body
perimeters, concave fillets, connected-edge propagation, attached-sketch remapping,
and cross-version topology replay remain outside this alpha's scope.

## Connected-edge milestone

Start each case from a fresh rectangular extrusion or Undo to its original box.
Use a 100 × 60 × 20 mm box, 2 mm radius/distance and 0.05 mm chord error.

| Action | Expected result | Actual / notes |
|---|---|---|
| Plain-click an edge, Ctrl/Command-click an adjacent edge. | Count 2; both orange; cyan candidate has the documented miter crease. | |
| Toggle an edge off with Ctrl/Command-click. | Count and preview update; obsolete Apply is disabled immediately. | |
| Select three edges meeting at one corner. | Faceted spherical junction, no gaps or extra solids. | |
| Select face perimeter, then click a face interior. | Four original edges selected; complete candidate prepared together. | |
| Select all twelve box edges. | Cylindrical bands and spherical corners; preview remains responsive. | |
| Select two disconnected edges of the same box. | Both cuts in one candidate. | |
| Ctrl/Command-click an edge on another solid. | Explicit same-solid refusal; no commit or changed model. | |
| Clear, Cancel, Escape and switch tools during batch preview. | No geometry/history mutation or orphaned preview. | |
| Apply a batch, Undo once, Redo once; save/reopen and export STL. | Whole operation restored atomically; valid closed mesh. | |
| Reduce maximum chord error. | Actual reported segment count grows when required, or an explicit safety-limit refusal; no silent clamp. | |
| Hover/focus/click all three information buttons and press Escape. | Readable operation-specific help; no accidental Apply. | |
| Try the hexagonal boss and the original base edge after boss extrusion. | Safe refusal remains; general polygonal/nonconvex support is not part of this milestone. | |
