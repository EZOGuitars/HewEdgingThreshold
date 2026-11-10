# Threshold engineering verification and release gates

## Provenance and scope

Threshold extends local development baseline `572c9ab43245f6738c0fd53be19a326739f4c77a` on `feature/edge-treatment`. Phase 2 baseline `ebb06e4f4a049a9a0c08f7d9acbe09d5c5482816` and both upstream/development remotes remain preserved. Native circular geometry and preview addressing were committed as `5947d11`; worker/control integration is `f93af40`. The release receipt identifies the final exact development/release SHA; both refer to the same source history, not an unrelated source dump.

Publication is authorized only to private `EZOGuitars/HewEdgingThreshold`, after critical gates pass. `EZOGuitars/Hew3dEdging` is not pushed. The release repository was independently inspected as private, empty (Git refs API returned 409) and writable by the authenticated account. Privacy/refs are checked again before publication. No force-push, upstream merge/rebase or upstream PR is permitted.

## Implemented geometry and controls

- Existing complete convex polygon cap bands and compatible opposite-cap sequential treatments remain supported, including triangles, asymmetric trapezoids, regular/irregular hexagons, octagons and other convex prisms.
- Uniform circular cylinder caps support upper/lower chamfer and faceted roundover, either cap first, with independent dimensions on successive opposite bands. Existing Cylinder ancestry is reused; old sidewall attribution survives clipping and save/load.
- Radius/distance/chord inputs remain canonical f64 meters. Five requested precision examples are tested, including 0.1875 inches/4.7625 mm. Display rounding and slider increments do not feed back into geometry. Sliders retain conservative native geometric bounds and independent endpoint validation.
- Worker previews use the same geometry/limits/candidate APIs for all supported representations. Snapshots and stable SID/hash/exact-endpoint addresses cross the boundary; raw Scene handles do not. Preview transport version 1 adds opaque origin epoch/history-generation tokens without changing recording or model formats.
- Rapid edits coalesce; obsolete workers terminate. Revision, exact source content, source epoch/history generation and input identity reject stale results. Reloading identical model bytes still invalidates a pending preview. Cancellation disposes worker and transient mesh resources; previews create no live undo actions or recording calls.
- A circular center snap can outrank face inference. The feature tool now uses existing `Scene::pick_face` as a perimeter-mode fallback, preserving eligibility/context checks without changing shared inference priorities.
- Apply rebuilds and certifies a fresh live-Scene candidate and commits through existing document/history APIs. Worker mesh data is never trusted as a replacement document. **Apply remains synchronous** and can pause for expensive geometry.

## Geometric safety and certification

`crates/kernel/src/edge_treatment/circular.rs` resolves axis, cap orientation, source radius and angular stations from existing topology/Cylinder metadata. It requires a closed convex carrier with a complete uniformly spaced ring, perpendicular cap and an untouched adjacent band. The existing supporting-plane/structural/contact tests remain; allowing validated Cylinder claims is local to the feature's circular-carrier resolver. Planar/single-edge validators retain their previous restrictions.

Circular cut samples form inscribed cap-band frusta through the existing indexed half-space clipping builder. They stay inside the original faceted cylinder envelope. The candidate passes topology/watertightness, support-plane containment, orientation and removed-volume checks. Expected removed volume comes from independent integration of polygonal cross-sectional area scaled by squared sampled radius, not from a second reading of the constructed mesh volume. Rotated/translated circular frames are tested.

Temporary `TreatmentSurface::Torus` references describe finite outer quarter-torus targets for certification only. They are not persistent analytic-surface claims. `crates/mesh-export/src/roundover_precision.rs` bounds deviation over **every point of the actual f32 STL triangles**, using meridian coordinate intervals and recursive subdivision. Reflection into the supported quadrant bounds finite-domain error, including triangle interiors and circumference faceting. Work/depth limits cause refusal, never acceptance by sampling. Independent JS barycentric samples supplement this proof; they do not replace it.

The existing generic STL writer is unchanged. Circular tests independently decode closed/manifold/outward meshes, expected volume, top/bottom radial dimensions and save/load/replay. They also test nonzero export resolutions (8, the UI default 48, and 128): trimmed cylinder bands must retain their stored seam-compatible facets rather than independently refit old walls. Existing re-faceting legitimacy checks provide that fallback without a shared export-policy change.

Source circumferential sagitta must leave a conservative error budget. A too-coarse source returns `SourceResolution`; increasing meridian segments cannot erase source circumference error. Circular rings are limited to 8–256 sides, meridian roundovers to 4–128 segments, and clipping to 4096 planes. Invalid parameters, overlapping bands, degeneracy, precision loss and unsupported geometry refuse without modifying the live solid.

## Architecture and upstream findings

Algorithms stay in `edge_treatment/{circular,perimeter,multi}.rs`; document adapters remain in their existing feature module. Preview addresses and source tokens stay in `wasm-api/src/edge_treatment_api.rs`. The new worker owner/entry point are `app/src/tools/edgeTreatmentPreview.ts` and `.worker.ts`; controls remain in the existing tool/panel. No plugin framework, dependency or core topology/persistence redesign was introduced. Shared document/Scene storage, inference algorithms, Follow Me and generic STL writer receive no new behavior changes.

The inspected official upstream branches are main, feature/plugin-system, site-live and v11/packaging. Main was `f23fc98bd79cb24b617f93ac96ad0601fe4a3c42`; no available True Curves feature branch was found. Existing CircleGeom/Cylinder attribution is reused. Compatibility with an unpublished future API is not claimed. Source provenance, original licensing and reviewable milestone commits remain intact; generic STL hardening remains separate from feature-specific certification.

## Test record

- Before implementation: the existing document, connected-box and generalized perimeter suites passed.
- Native focused kernel/document tests: 45 passed, including generated convex polygon cases, source immutability, rotated frames, circle quality refusal and original joined-boss/global-nonconvex refusals.
- Native WASM API library: 259 passed, including circular certification/sequences, isolated-snapshot addressing, exact undo/redo and replay, history ABA, reload identity and malformed preview versions.
- Mesh-export package: 56 passed; generic writer, old certificates, metric/f32 precision and other formats remain covered.
- Actual generated Node WASM circular suite: all four upper/lower operation-order cases pass. It independently measures STL geometry and checks nonzero export resolutions. Existing polygon, connected-edge, metric and lifecycle suites remain included in frontend regression.
- Final frontend regression: **6,712 passed across 310 files**. Final browser regression: **14 passed**, including native pointer selection, circular sequences, tooltips, exact units, cancellation, source reload/ABA and worker interaction checks. The Node smoke passed all four tests. Production/test TypeScript, lint, i18n and production/PWA build checks passed. Earlier contention/HMR failures are superseded by quiet final runs.
- The optimized production worker (not only Vite development worker) was instantiated in Chromium and evaluated a circular roundover successfully: 624.7 ms including its limit query, 2,949,120 bytes of WASM linear-memory capacity, and nonempty preview buffers.

An initial concurrent frontend run had unrelated DOM test timeouts under optimizer/browser/compiler contention and was stopped. Quiet runs replace that result. A development-server reload interrupted one browser test while a worker file was edited; it passed when rerun with stable source. These failures were investigated, not counted as passes.

## Performance and measurement limits

The original Node/WASM warmed reference remains approximately 1–10 ms for ordinary selections and 883 ms for the fine all-edge case. Worker scheduling does not make the geometry algorithm faster. Browser jobs include snapshot loading, optional range endpoint validation, initialization and transport; their wall time is not interchangeable with warmed Node prepare-only time.

The final headless Chromium/software-rendering fine-box run measured: 3,007.2 ms worker computation including a range query, 3,265.6 ms end-to-end preview, 5.8 ms main-thread dispatch, 10 ms focused text-input delivery, and 8,192,000 bytes worker WASM memory capacity. It created at most one worker at a time and returned to zero active workers. Synchronous final Apply took 1,061.9 ms. These are development-host samples, not Fedora guarantees, resident-memory accounting or p95 data. Raw metrics accompany the release.

Final preview compositor frame gap was 430.2 ms; earlier headless runs reached 2,463 ms despite main-thread long tasks below 100 ms. The final preview had no reported main-thread long tasks. A raw requestAnimationFrame-only gate therefore did not establish JavaScript blocking; direct focused keyboard input and main-thread task duration are the interaction gates (400 ms input, 100 ms dispatch/task budgets), while frame gaps remain explicitly reported. Playwright locator actionability waits were excluded by focusing the test field before starting computation. No claim of smooth Fedora hardware rendering follows from these tests. Main-thread Apply, software-rendered frame jitter, worker startup and large-document snapshot/hash costs remain limitations.

The browser stress test covers rapid supersession, ten repeated cancellations, bounded worker ownership and an identical-byte document reload during computation. It verifies source purity before Apply and after cancellation. Reload resets session history, so its combined source/history flag changes; canonical geometry must still remain unchanged and the stale preview must be rejected.

## Scope exclusions and release mode

Mixed straight/curved and nonuniform/elliptic attributed boundaries are not implemented or claimed. Arbitrary globally nonconvex solids, overlapping/re-blended caps, arbitrary sequential edges, NURBS, G2, variable radius and guitar-body contours remain unsupported. A standalone polygon perimeter is supported; a joined polygonal boss on a base retains the original safe refusal. Persistent ideal toroidal provenance and retrospective certification of arbitrary later re-triangulation are not added.

The host cannot locate GTK3 or WebKitGTK 4.1 through pkg-config. No Fedora desktop binary, installer, native launch or hardware-frame-rate certification is claimed. The authorized fallback is a reproducible **source-build alpha release**. Complete upstream source/licensing notices, locked build inputs, Fedora instructions, source archive, incremental bundle, verification/limitations and checksums are the release assets. Compiled caches, local state, credentials and stale binaries are excluded.

The final publication gate is the completed native/WASM/browser/frontend checks plus privacy, source completeness, secret scan, exact commit/tag/asset correspondence and remote verification. If a critical gate remains failed, publication must stop; this document alone is not proof that publication occurred.


## Additional measured geometry costs

Development-host Node release WASM, two warmups and three retained samples; prepare/certification, mesh-buffer copy and discard. No GPU, range query or Apply is included. These are samples rather than controlled cross-version speedup claims.

| Case | Radius/chord mm | Median/max ms |
| --- | --- | --- |
| Straight single edge | 2 / 0.05 | 14.14 / 14.88 |
| Trapezoid perimeter | 3 / 0.02 | 9.04 / 14.98 |
| Cylinder, 64 source sides | 3 / 0.3 | 43.62 / 61.46 |
| Cylinder, 128 sides, finer tolerance | 3 / 0.1 | 230.83 / 241.46 |
| Opposite cylinder cap after first treatment | 2 / 0.3 | 132.71 / 150.45 |

The final stress test used fourteen worker instances across initial evaluation, supersession, repeated cancellation and reload, with peak concurrency one and zero active workers at completion. Its source geometry remained unchanged even during the explicit identical-byte reload. WASM memory figures describe linear-memory capacity, not resident memory or total GPU/browser memory. Any serialized document change conservatively invalidates preview; saved view-state changes can require regeneration as well. A quality/precision-limited range probe can suppress a slider while exact entry remains available under full validation.
