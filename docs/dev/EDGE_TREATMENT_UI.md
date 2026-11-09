# Edge Treatment tool (Phase 4B)

The Modify rail, Tools menu, command palette, and native Tools menu activate
one `Edge Treatment` tool. It supports Chamfer and Roundover, with no default
keyboard shortcut. Escape/Cancel returns to Select. Enter applies only a
current preview with any annotation warning acknowledged.

## Ownership and flow

`EdgeTreatmentTool` consumes the existing inference Snap's object and edge
BigInts and the viewport's face-edit eligibility/context policy. It owns one
transient subtree under the existing preview group, marked as a session ghost.
It does not register preview objects with inference or persistent render caches.
The orange line identifies the selected source edge; cyan faces/lines show the
actual candidate from `Scene.edge_treatment_mesh`, over the unchanged source.

The tool's transient state is input text, target, candidate handle, warnings,
and phase (select/pending/ready/error). It is not another document model.
Parameter edits immediately disable Apply and remove the old mesh; regeneration
is debounced by 180 ms. Explicit Preview regenerates immediately. The UI uses
the shared length parser and display units; explicit unit suffixes work. Raw
input is not rounded back through display precision. Unit changes preserve
parsed f64 values using explicit meters. Invalid/nonpositive/nonfinite inputs
are refused, never clamped.

Preparation uses the shared Phase 4A lifecycle. A successful prepare supersedes
the old slot. On a failed prepare, the old native candidate would remain alive;
the tool explicitly discards it so stale input cannot accidentally Apply it.
The returned handle, source, edge, operation, and dimensions must match the UI.
The displayed mesh and Apply use that same handle. Every MeshJs wrapper is freed
in `finally` after copying its buffers. Replaced/discarded Three.js geometries
and materials are disposed through the existing `clearPreview` helper.

Annotation warnings list the supplied annotation SIDs and explain detachment,
retained points, and undo restoration. The acknowledgment is reset for every
new candidate, including manual regeneration. Both button and keyboard Apply
check it. API error codes become localized, actionable inline feedback.

Apply calls only `Scene.commit_edge_treatment`, then uses the normal document
refresh and selection callbacks. It clears the tool's topology target before
refresh, selects the resulting object, and frees DocChangeJs. No JS geometry
mutation or extra history/recording action is introduced. The viewport's shared
document-change hook invalidates targets/previews on external edits, undo/redo,
and load. Context changes, tool switches, Escape, Cancel, and viewport disposal
release timers, candidates, GPU resources, and unit subscriptions.

## Scope and limitations

Geometry eligibility is unchanged: supported straight perpendicular convex
edges on convex planar solids. No curved perimeter, concave fillet, edge-chain
propagation, or topology/sketch remapping. Existing group/component editing
sessions and object-owned sketches are refused. The UI honors the normal
container editing policy; grouped objects cannot currently be treated through
a group session. Use a plain ungrouped object for the experimental UI.

The tool accepts an actual edge snap, never guesses an edge from a vertex,
face, or drawing axis. If an axis wins a coincident inference pick, move the
drawing axes or choose another edge. Source edge handles are discarded after
commit; they are not assumed to identify rebuilt topology.

Candidate computation remains synchronous WASM. Debouncing avoids repeated
work while typing, but complex/high-resolution requests can still stall the
main thread; no speculative worker/concurrency framework is added.

Native candidate guards remain deliberately conservative. Existing recovery
autosave persists the camera as non-undoable document view state. A changed
camera written during preview can stale a candidate; Apply refuses and asks
for regeneration. This must not be worked around by weakening native guards.
Browser geometry tests compare exact committed STL bytes so incidental camera
persistence does not produce false geometry/undo failures. Node/WASM and tool
integration tests additionally compare canonical document bytes.

## Verification entry points

Before UI integration, build and run the actual binding smoke:

```sh
wasm-pack build crates/wasm-api --target nodejs \
  --out-dir ../../tools/replay-runner/pkg-node --mode no-install
node --test tools/replay-runner/edge-treatment-smoke.mjs
```

Use wasm-bindgen matching Cargo.lock. The smoke covers BigInts, numeric handles,
both modes, metadata, actual mesh buffers, commit/discard, stale/foreign handles,
wrapper/Scene disposal, undo/redo, persistence, and recording/replay.

Frontend tests:

```sh
pnpm --dir app test --maxWorkers=2 \
  src/tools/EdgeTreatmentTool.test.ts \
  src/tools/EdgeTreatmentTool.wasm.test.ts \
  src/panels/EdgeTreatmentPanel.test.tsx
pnpm --dir app exec playwright test e2e/edge-treatment.spec.ts --project=chromium
pnpm --dir app typecheck
pnpm --dir app typecheck:test
pnpm --dir app lint
pnpm --dir app build
```

The additional Vitest real-WASM tool suite requires pkg-node and explicitly
skips when it has not been generated; the standalone smoke is not optional for
boundary verification. The browser test uses real pointer selection and WebGL
with actual candidates. In managed environments, a temporary Playwright config
can select installed Chromium and software rendering without changing project
dependencies or the shared browser matrix.

No-install wasm-pack may skip wasm-opt and retain a large name section. If that
exceeds the existing PWA precache ceiling, strip debug/name metadata with
Binaryen before the frontend build; do not raise the application cache ceiling
just to accommodate debug metadata. This does not change geometry arithmetic.

## Fedora experimental build handoff

1. Keep the feature branch and all phase patches; review native and frontend
   changes before making a release. No new project dependencies are required.
2. Install the repository-pinned Rust/WASM and pnpm toolchains, matching
   wasm-bindgen/wasm-pack, and Tauri 2's Fedora WebKitGTK/GTK/AppIndicator/build
   prerequisites. Follow the current Tauri Linux/Fedora setup instructions.
3. Run frozen-lockfile installation, build web and Node WASM packages, execute
   the real smoke, frontend checks, edge/document and export regressions.
4. Run `pnpm --dir shells/tauri build` (the existing script stages the CLI
   sidecar). Build on Fedora and label the artifact experimental; desktop
   packaging/WebKit behavior was not verified in the managed Linux browser run.
5. Exercise Apply/Cancel, warning acknowledgment, save/reopen, undo/redo,
   recording/replay, document replacement, and repeated tool switching on real
   models before distributing anything. Do not imply curved/concave support.
