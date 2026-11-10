# Edge Treatment modularity audit — design review

## Scope and conclusion

Read-only source audit of `feature/edge-treatment` at `ebb06e4f4a049a9a0c08f7d9acbe09d5c5482816`, compared with its upstream common base `b2e5baee8b0b84d7eb3d8c8f26ff36aabf7fa42e`. This covers the complete fork implementation, not only the last integration commit. The local branch and remote feature branch were previously verified at the same SHA. No implementation, refactoring, synchronization or publishing is authorized by this review.

The feature is already substantially isolated. Retain its dedicated geometry, document adapter, WASM adapter, tool and panel. A separate plugin, kernel, crate hierarchy or general-purpose feature framework is not justified. The main debt is a small number of feature-aware hooks in shared document/viewport infrastructure, duplicated lifecycle policy, and the need to distinguish broadly applicable STL hardening from feature-specific certification. Those are bounded integration issues, not evidence that Hew needs a redesign.

This audit supplements [the coordinated control/engine design](EDGE_TREATMENT_CONTROL_ENGINE_DESIGN.md), [Phase 3 architecture](EDGE_TREATMENT_PHASE3_ARCHITECTURE.md), and [acceptance matrix](EDGE_TREATMENT_PHASE3_TEST_MATRIX.md). All recommendations below are proposed work, not changes already made.

## Exact inventory of modified existing production files

There are 23 modified existing production/configuration files relative to the common base. New feature modules and tests are listed separately. Catalog changes are integration resources, not geometry-core modifications.

| Existing file | Verified change and assessment |
| --- | --- |
| `crates/kernel/src/lib.rs` | Module declaration and public exports. Necessary native API registration; retain. |
| `crates/kernel/src/document.rs` | Child-module/reexports, history labels, `DocAction::EdgeTreated`, touched-entity/history/undo/redo matches, and session-only `edge_candidate_identity`. Some shared integration is necessary; feature-specific policy mostly already lives in its child module. See transaction assessment below. |
| `crates/kernel/src/ops.rs` | Only `faces_improperly_contact` changes from private to `pub(crate)`. Reuses an existing safety predicate without altering its algorithm. Reasonable minimal extension; a later generic validation-module move is optional and must preserve all existing callers. |
| `crates/kernel/src/tol.rs` | Named `EDGE_TREATMENT_VOLUME_REL` and `EDGE_TREATMENT_VOLUME_ULPS`. Consistent with central tolerance policy. Do not rename these as universal tolerances without evidence that their budgets apply to other operations. |
| `crates/mesh-export/src/lib.rs` | Registers precision certification; adds `ExportError::StlIntegrity`; changes `write_binary_stl` to return failure and validates quantized solid ranges via `validate_stl_solid`. This is the largest substantive shared behavior change. Generic safety improvement, but broader than feature integration; review separately. |
| `crates/wasm-api/src/lib.rs` | Registers feature adapter, owns one prepared candidate in `Scene`, initializes/clears it on successful load, dispatches replay variants, documents stricter STL failure. Small integration hooks; retain ownership until a proven generic candidate facility is warranted. |
| `crates/wasm-api/src/recording.rs` | Adds `RecordedCall::EdgeTreatment` and `EdgeTreatmentBatch`. Necessary replay dispatch under the existing recording convention; command payload details remain in the feature adapter. Format-version policy needs explicit upstream review. |
| `app/src/App.tsx` | `TOOL_MENU_IDS` and command-dispatch entry. Necessary tool registration. |
| `app/src/i18n/historyLabels.ts` | Treatment history labels. Necessary localized history integration. |
| `app/src/i18n/nativeMenu.ts` | Native tool-menu identifier. Necessary registration. |
| `app/src/locales/en/messages.po` | Feature strings/catalog updates. Retain required strings; review unrelated catalog churn separately. |
| `app/src/locales/ja/messages.po` | Feature strings/catalog updates; same assessment. |
| `app/src/locales/pseudo-LOCALE/messages.po` | Feature strings/catalog updates; same assessment. |
| `app/src/palette/registry.ts` | Tool palette registration. Necessary discoverability integration. |
| `app/src/panels/MenuBar.tsx` | Tool menu registration. Necessary integration. |
| `app/src/tools/toolIcons.ts` | Feature icon mapping. Necessary UI registration. |
| `app/src/tools/toolRegistry.ts` | Tool identifier/registry entry. Necessary integration. |
| `app/src/viewport/Viewport.tsx` | Tool construction, panel state/mount, selection modifier forwarding, document-change invalidation and cleanup. Integration is necessary, but direct feature-aware lifecycle checks and construction details can be reduced; see below. Also moves dirty notification before fallible renderer refresh, a generic correctness fix. |
| `app/scripts/build-wasm.mjs` | Includes script mtime and verifies all required generated outputs before skipping a build. Generic build correctness; separate from geometry contribution. |
| `shells/tauri/scripts/stage-cli-sidecar.mjs` | Uses Cargo-reported artifact path, locked build and unchanged-content copy avoidance. Generic cache/custom-target support; separate build contribution. |
| `shells/tauri/src-tauri/locales/menu.en.json` | Native tool label. Necessary registration. |
| `shells/tauri/src-tauri/locales/menu.pseudo-LOCALE.json` | Native tool label. Necessary registration. |
| `shells/tauri/src-tauri/src/main.rs` | `main` registers/routes the tool menu item. Necessary native menu integration; no geometry logic. |

Two additional existing test files changed: `app/src/tools/toolRegistry.test.ts` and `crates/mesh-export/tests/f32_winding_specs.rs`. The export diff also includes substantial inline tests; raw line counts overstate its production-code footprint.

No fork changes were found in the existing topology representation (`topo.rs`), Boolean implementation (`boolean.rs`), sketch/profile implementation (`sketch.rs`), structural validator (`validate.rs`), per-object history (`history.rs`), or serialization implementation (`serialize.rs`). `ops.rs` has the visibility-only change above: Follow Me behavior was not changed. No dependency manifests or lockfiles were changed.

## Dedicated subsystem boundaries already present

| Module | Actual responsibility and important entry points |
| --- | --- |
| `crates/kernel/src/edge_treatment.rs` | Pure geometry and typed refusal; `chamfer`, `roundover`, single-edge frame/clearance/cut checks and approximation policy. |
| `crates/kernel/src/edge_treatment/multi.rs` | `chamfer_edges`/`roundover_edges` implementation, `BoxFrame` recognition, connected boundary construction, junction surface references and independent volume accounting. Box eligibility belongs here, not in shared topology or frontend heuristics. |
| `crates/kernel/src/document/edge_treatment.rs` | `EdgeTreatmentCandidate`, `prepare_edge_treatments`, `commit_edge_treatment`, target eligibility, stale-source protection and annotation policy. Geometry preparation is detached; commit preserves object identity and emits ordinary `DocChange`. |
| `crates/wasm-api/src/edge_treatment_api.rs` | `prepare_treatments`, public prepare/info/mesh/commit/discard methods, target/context resolution, certification and replay. Owns marshaling and ephemeral handles, not a second geometry engine. |
| `crates/mesh-export/src/roundover_precision.rs` | `certify_roundover_stl` and `certify_treatment_surfaces` measure exported approximation against feature surface targets. Correct layer for actual f32/export effects; keep separate from generic writer integrity checks. |
| `app/src/tools/EdgeTreatmentTool.ts` | Tool selection, parameter state, request scheduling, preview resources and candidate lifecycle under existing ToolController conventions. |
| `app/src/panels/EdgeTreatmentPanel.tsx` | Operation-specific parameter display, accessible tooltips and user actions. Geometry-limit calculations must remain native, not migrate into this panel. |

The proposed slider/precision controls should be children of this panel. Extract a reusable dimensional input only if its contract is useful independently; reuse Hew's units parser/preferences rather than introducing another unit system. Preserve physical values independently of display formatting.

## Necessary core integration versus movable policy

### Document transactions

`Document::apply_object_op` in `document.rs` routes a `KernelOp` through per-object `History::apply` and face-ride planning. It is not an existing arbitrary detached-candidate replacement interface. `commit_repair` is a private repair-specific snapshot mechanism. Neither can simply replace the feature adapter without changing contracts.

The current `DocAction::EdgeTreated` follows the snapshot convention used by `Repaired`: undo/redo replace geometry and restore annotation state without recomputing. Its exhaustive match arms and localized history label are legitimate integration. Retain those small hooks for now rather than forcing all geometry through a new `KernelOp` or deleting/recreating objects. Candidate preparation/validation/annotation decisions belong in `document/edge_treatment.rs`, where they already reside.

The session identity field is feature-named but implements a generally useful stale-candidate guard. A future generic opaque document-session token and checked geometry-replacement transaction could serve other detached modeling tools. Introduce them only with a concrete second use or an agreed upstream contract. They must preserve ObjectId/SID, ownership, attributes, annotations, per-object history and single-action undo semantics. Do not expose unrestricted mutable replacement merely to avoid an enum variant.

Preparation currently snapshots `self.save()` and commit compares full saved content as well as history generation and identity. This is strong stale-state protection but scales with document size. Profile it; a generic revision token is a possible later optimization only if it detects every relevant mutation/load/undo/context change. Removing the snapshot check without equivalent coverage would weaken safety.

### Viewport and tool lifecycle

`Viewport` already owns ToolController, edit context, picking, preview group and render refresh. Registering the feature there is appropriate. The `makeEdgeTreatmentTool` factory and panel bridge could move into a small feature integration adapter, leaving explicit registration calls. Such extraction improves readability but does not require an extensibility framework.

The direct `instanceof EdgeTreatmentTool` checks for model-change notification, selection-modifier forwarding and cleanup are the main avoidable feature coupling. Evaluate optional, generic Tool lifecycle hooks (document changed / pointer modifiers) against existing tool conventions before adding them. Continue using normal cancellation on tool disposal. Do not put treatment eligibility, radius bounds, box recognition or curve reconstruction into Viewport, renderer, picker or ToolController.

Moving `onDocumentChangedRef` before fallible GPU refresh is a generic dirty-state correctness change, independent of Edge Treatment. Keep its regression coverage and propose it as an independently reviewable upstream fix.

### Export safety

`write_binary_stl` and `validate_stl_solid` now refuse unsafe quantized output instead of dropping triangles or returning a partial file. This must not be rolled back or bypassed to make the feature more isolated. It intentionally makes some formerly successful unsafe exports fail: it is not behavior-identical for every existing model.

Writer packing and generic integrity helpers could move into a private mesh-export child module if this improves that crate's organization; retain the public export entry points and typed errors. Feature-specific cylinder/sphere/plane deviation certificates stay in `roundover_precision.rs`. Do not inject feature knowledge into STL packing, and do not treat generic manifold validation as proof of requested radius or chord accuracy.

## Extension-point recommendation

Existing public profile/extrusion/Boolean primitives, document transactions/DocChange, ToolController, picking, preview meshes, units and export APIs are sufficient for the current alpha. No new framework is necessary.

For the approved future design, the smallest coherent additions are feature-local interfaces for: source-bound requests; eligibility and whole-operation limits with certainty; ordered semantic boundary views; preview/final evaluation with cost estimates and revision tokens; and persistent output attribution suitable for another treatment. Sequential, polygonal and curved treatments should be strategies behind those same contracts, not new tool types or shared-core special cases.

Only two shared extensions currently have a plausible broader purpose: generic tool lifecycle notifications and a reviewed detached geometry transaction/session token. Both remain conditional design proposals. True analytic surface/provenance schema additions require upstream coordination, not a private parallel type hierarchy. Asynchronous work ownership/cancellation should be designed with the existing runtime; do not put browser jobs or scheduling into the pure kernel.

The measured all-edge fine preview (~883 ms in the existing Node/WASM probe, not a Fedora latency certification) remains a release-relevant constraint. Debouncing does not make synchronous WASM interruptible. Keep source revisions, cancellation, quality budgets and job ownership in the orchestration boundary; geometry strategies report cost/complexity consistently. Full-document snapshots and actual export certification also belong in profiling. Preserve strict final validation regardless of preview quality.

## True Curves compatibility and upstream impact

Inspected official upstream main was `f23fc98bd79cb24b617f93ac96ad0601fe4a3c42`. Existing `topo.rs` has circle ancestry and `SurfaceRef::Cylinder`; the fork has not changed that schema. Those interfaces can inform a read-only semantic-boundary adapter. They do not establish a published general torus, sphere or future True Curves contract. Compatibility with unpublished work cannot be certified.

The current batch builder is specialized to pristine boxes and initializes output face surfaces without preserving arbitrary source surface ancestry. That is safe within its restricted input contract, but it is a blocker to blindly reusing it for sequential/curved geometry. Preserve mapped attributes/provenance, define source-envelope authority, and coordinate analytic surface/domain types upstream before widening eligibility. Keep current polygonal/nonconvex and sequential batch refusal until validated replacements exist.

The known upstream overlap is Viewport and locale catalogs; new view-plane inference also affects tool integration semantically. A thin feature adapter reduces merge pressure, but does not eliminate integration testing. Keep generic STL fixes, generic dirty-state fix, build improvements, pure geometry, document/WASM adapters and frontend integration independently reviewable. Preserve the published alpha history; upstream PR decomposition is a separate approved operation, not a reason to rewrite it now. Review recording-format documentation/version policy and any future persistent curve schema explicitly.

## Acceptance and preservation gate

A later modularity refactor must retain existing geometry/refusal, replay, stale-candidate, annotation, preview cancellation, undo/redo, persistence and f32 STL tests. Assert that unrelated tools retain their lifecycle behavior and unsafe exports still fail atomically. New sequential/polygonal/curved strategies must share limits, diagnostics and lifecycle contracts and retain output eligible for subsequent operations.

This audit ran no new geometry tests because no source was changed. The preceding design-review checks passed 38 relevant native and 215 frontend tests; broader alpha QA and its desktop certification gaps are recorded in the existing review documents. Those results do not certify an unimplemented generalized engine.

Only design documents are added/updated by this review. No source refactoring, dependency changes, commits, pushes, alpha identity changes or installed-app changes were made. Await approval before implementing the proposed architecture.
