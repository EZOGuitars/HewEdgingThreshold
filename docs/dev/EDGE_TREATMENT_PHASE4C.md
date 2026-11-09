# Edge Treatment Phase 4C engineering audit

## Executive result

The implementation remains an experimental straight convex-edge tool. Geometry
algorithms, native document identity/atomicity guards, STL integrity, and existing
Follow Me behavior are unchanged. Phase 4C adds exception-safe preview ownership,
accurate post-commit error reporting, early dirty-state notification, and an
isolated desktop alpha configuration. No Phase 5/curved-edge work was undertaken.

Final verification and publication receipts are recorded separately to avoid a
self-referential archive checksum/commit SHA. See also the
[Fedora guide](FEDORA45_EDGE_ALPHA.md) and
[manual checklist](EDGE_TREATMENT_ALPHA_CHECKLIST.md).

## Preservation and history

At entry, `feature/edge-treatment` matched the complete saved Phase 4B after-patch
(247,587 bytes), including its untracked source files. The baseline was
`b2e5baee8b0b84d7eb3d8c8f26ff36aabf7fa42e`. The checkout was shallow: one visible
commit. Fetching upstream with `git fetch --unshallow origin` recovered all 338
original reachable commits and upstream tags without changing the working tree.
A verified pre-QA Git bundle records a complete history.

The initial source archive contains 1,808 files, excludes build/dependency caches
and Git/authentication configuration, and was extracted and compared byte for
byte against the working source. Receipt:

- File: `source-before-qa.tar.gz` (kept with the external Phase 4C receipts).
- Size: 37,383,777 bytes.
- SHA-256: `4222fdaf8280ad7e96789f79293db0977456d9896ac05dfd3c23a8834f1179ca`.

A final source archive and full Git bundle include QA fixes, tests, handoff docs,
and organized commits. Their exact final sizes/checksums are in the delivery
receipt, not embedded in the source they hash. Generated SDKs/builds, dependency
caches, temporary files, credentials, and ignored internal design files are not
published. The original upstream remote is retained separately.

The user's explicit private-repository preservation instruction authorizes local
commits and a private push, superseding the audit brief's earlier no-commit/
no-push restriction. It does not authorize a public repository, release, upstream
PR, or deployment.

## Investigation of the original 12 frontend failures

The original Phase 4B broad run reported 6,633 passing / 12 failing tests in 304
files and took 829.94 seconds. The later isolated 390-test pass was not sufficient
to establish root causes. Phase 4C repeated the **entire** pre-QA source suite
with the original five-second timeout, two workers, Rust tools on PATH, and no
concurrent native/optimizer builds: **6,647/6,647 passed in 366.48 seconds**.

The same environment has a two-core CPU quota and an 8 GiB memory limit. Native
builds and wasm-opt overlapped the original run; original peak utilization was
not retained. No frontend OOM or disk-full error was recorded in those 12
failures. Contention is a plausible contributor, not a proven cause for each.

| ID | File | Exact original test | Original error | Classification |
|---|---|---|---|---|
| F01 | `src/App.openInNewWindow.test.tsx` | App — File ▸ Open onto a non-pristine window (Tauri) > opens a picked .hew file into a NEW window, leaving the current (non-pristine) window untouched | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F02 | `src/App.openInNewWindow.test.tsx` | App — File ▸ Import… onto a non-pristine window (Tauri) > opens a picked import file into a NEW window, leaving the current (non-pristine) window untouched, no discard prompt | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F03 | `src/App.test.tsx` | App — tray layout persistence > collapsing a section via its header persists to the trayLayout singleton | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F04 | `src/App.test.tsx` | App — unified Open dialog > File ▸ Revert to Saved > a .skp document > revert keeps the camera, does not zoom to fit, and does not reopen the import report | TestingLibraryElementError: Unable to find role="dialog" and name "Open report" | Unresolved; load-sensitive/non-deterministic in available runs |
| F05 | `src/App.test.tsx` | App — unified Open dialog > File ▸ Revert to Saved > a .skp document > refuses with a toast while another .skp open is still stepping | TestingLibraryElementError: Unable to find role="dialog" and name "Open report" | Unresolved; load-sensitive/non-deterministic in available runs |
| F06 | `src/App.test.tsx` | App — unified Open dialog > File ▸ Revert to Saved > a change to the document while the file is being read cancels the revert and says so | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F07 | `src/App.test.tsx` | App — unified Open dialog > File ▸ Revert to Saved > View ▸ X-ray and Wireframe > X-ray and Wireframe are one mode: choosing one turns the other off, choosing it again returns to Shaded | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F08 | `src/panels/LibraryDialog.test.tsx` | LibraryDialog > list view: Name/Type/Size columns > Escape mid-drag tears down the List view resize listeners — a later pointermove no longer mutates the persisted width | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F09 | `src/panels/LibraryDialog.test.tsx` | LibraryDialog > materials match Components’ "in this model" treatment > refreshes the dot and the "in this model" scope filter when materialInPalette gets a new identity, without closing the dialog | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F10 | `src/shop/ShopApp.test.tsx` | ShopApp — landscape orientation (design_handoff_shop_mode §5) > rotating back to portrait restores the dock/bottom sheet and preserves the drag-opened detent + hides | Error: Test timed out in 5000ms. | Unresolved; load-sensitive/non-deterministic in available runs |
| F11 | `src/kernelErrorsDump.test.ts` | refusal_copy.gen.rs > stays in sync with app/src/kernelErrors.ts | Error: rustfmt failed (status null): | Confirmed environment limitation |
| F12 | `src/nativeMenuParity.test.ts` | native menu labels come from the one table (docs/LOCALIZATION.md "Native menus") > the committed menu.en.json is what the generator produces | AssertionError: run `pnpm --dir app i18n:menus`: expected '{\n  "submenu:app": "Hew",\n  "submen…' to be '{\n  "submenu:app": "Hew",\n  "submen…' // Object.is equality | Confirmed implementation artifact omission, corrected in 4B |

F11 is directly reproducible without the Rust PATH: `spawnSync('rustfmt', ...)`
returns no exit status when the executable is unavailable. With the pinned Rust
binary on PATH the fixture generator succeeds; no generated Rust copy changed.
F12 compared a correctly generated menu table containing the new tool with a
stale committed JSON table. Phase 4B regenerated the English and pseudo-locale
menu artifacts; the parity tests now pass. This was an implementation artifact
omission, not a bad assertion.

F01–F10 remain **unresolved as to root cause**. Eight exhausted the five-second
test timeout, and two failed to observe the asynchronous Open report dialog.
A clean broad pass establishes non-determinism in the available runs; it does
not prove the absence of a scheduling, order, or cleanup defect. No default
timeout was increased, no assertions were weakened, and none of these tests was
removed or edited in Phase 4C.

### Shared state, ordering, and cleanup review

- `App.test.tsx` shares a mutable mock Scene and file-host/recovery fixtures
  within the file. Before/after hooks reset tray layout, welcome/recovery state,
  mock results, and temporary `.skp` open methods. Its boot wait has a separate
  two-second deadline; dialog `findByRole` has its own asynchronous wait.
- `App.openInNewWindow.test.tsx` resets calls and the native invoke implementation
  before each test, but still exercises real asynchronous App orchestration.
- Library tests share mutable fixture maps/listeners and localStorage preferences;
  the file resets those before each test. Resize tests explicitly exercise
  Escape/unmount teardown.
- Shop tests share fixture state, storage, orientation, and long-press timers.
  Before hooks clear fixture/storage state and set portrait dimensions; after
  hooks restore window size. Several tests use actual timer delays.
- The shared setup unmounts React trees. Vitest's configured fork isolation is
  retained. File order can differ because of duration/failure scheduling caches;
  the exact original complete order was not saved. No order dependence or
  attributable async cleanup defect was demonstrated in these six files.

### Toolchain audit

Earlier Phase 4B runs and the first 4C control used Node 24.22.1/pnpm 11.25.0;
Phase 4B's wasm-pack was 0.13.1. Those differ from repository pins. Phase 4C
installed checksum-verified official Node **26.7.0**, pnpm **11.20.0**, and the
repository install script's wasm-pack **0.15.0**. Rust **1.97.1** and
wasm-bindgen **0.2.127** match their pins. No project dependency/lockfile changed.
The final full-suite run uses these pins, the same normal timeout, and no
concurrent heavy builds. Toolchain drift is confirmed; it is not proven to
explain F01–F10.

## Confirmed lifecycle defects and narrow fixes

Four new fault-injection tests failed before the fixes and passed afterward:

1. Throw during the first BufferGeometry setup: no dispose call was made for
   that allocation before attachment (expected one, observed zero).
2. Throw during edge setup after the surface was assembled: the unattached
   edge allocation escaped disposal (expected two, observed one).
3. Throw in the host's refresh callback after native commit: UI misleadingly
   said the treatment could not be validated, although the model had committed.
4. Throw from a preview dispose listener after native commit: DocChangeJs was
   not freed and the host was not notified.

These establish cleanup/feedback contract gaps. They do **not** establish a
measured GPU memory leak: ordinary Three.js GPU upload is deferred until render.
The fix tracks geometries/materials locally until all are ready, publishes the
complete preview together, and disposes pre-attachment allocations on failure.
MeshJs is freed in a nested finally even if cleanup raises an exception.

Apply now separates a refused native commit from later view/cleanup failure.
It invalidates topology state, attempts host notification even after preview
cleanup failure, always frees DocChangeJs, and says the treatment **was applied**
when only refresh failed. The viewport notifies document/dirty state before GPU
refresh so a renderer exception cannot suppress the changed-model indication.
A real Chromium fault-injection test verifies changed geometry, one recorded
treatment, visible dirty state, disabled duplicate Apply, and successful Undo.
No alternative undo/transaction framework was added.

Candidate replacement, failed preparation, rapid input edits, parameter/handle
matching, acknowledgment reset, foreign/consumed handles, load/context changes,
and tool/viewport teardown retain the Phase 4A/B ownership design. Apply remains
disabled as soon as input changes. Failed preparation releases the retained old
slot; an old preview cannot be applied as newly entered parameters. The real-WASM
tests continue to cover actual annotation warnings, wrapper ownership,
canonical-byte preservation, commit, undo/redo, persistence, and replay.

## Document and command audit

- Typed preparation/commit refusals leave canonical document/history untouched;
  success is one native document action and one semantic recorded treatment.
- Geometry is never independently edited from JavaScript. No preview/cutter
  enters the document. Object/SID, metadata, default materials, and face colors
  retain their existing validated behavior.
- Matching commit attempts consume their handle. Native identity, generation,
  exact-state, topology/watertightness, containment, and volume checks remain.
- Replay requires the source hash, stable object SID, and exactly one exact
  endpoint match; an ambiguous or missing match refuses rather than guessing.
  Tests execute mismatched source/edge rejection and replay after reconstruction;
  the ambiguity guard is verified in code, not claimed as a separately executed
  ambiguous-valid-solid test.
- Recording is synchronous/in-memory and returns no fallible Result; the new
  semantic call is appended only after successful native mutation/reconciliation.
  Existing camera persistence has its own recorded SetCameraState variant.
  No ordinary recording error path was found that returns a failed edit after
  mutating the model. Fatal OOM/process panic remains outside these ordinary
  typed-failure guarantees, as with other Scene operations.
- Post-commit renderer failure is now explicitly distinguished from native
  refusal; model, history, and recording survive it. Native transactions remain
  caller-owned and nest under the existing conventions.
- Recovery camera saves can still stale a preview under the deliberately
  conservative exact-state guard. Regeneration is required; the guard was not
  weakened. Browser geometry assertions account for independent camera autosave,
  while Node/WASM tests compare exact canonical bytes.

## Rendering and Fedora isolation

The new tool uses existing Three.js BufferGeometry, Float32/Uint32 attributes,
MeshBasicMaterial/LineBasicMaterial, and the normal WebGL2 pipeline. There are no
custom Chromium shaders or new browser extensions. Buffers are copied before
MeshJs.free, and object IDs stay BigInt/decimal strings rather than Numbers.
Pointer events, debounce timers, and React controls use the existing event path.
The translucent depth-independent cyan overlay is deliberately not committed
geometry and does not populate persistent caches.

The managed system is Debian 13. `pkg-config` cannot find WebKitGTK 4.1 or GTK3;
no MiniBrowser/WebKitWebDriver is installed. No WebKitGTK/Tauri application was
executed, and **WebKitGTK compatibility is not claimed**. Verify WebGL2/shader
compilation, BigInt boundary calls, disposal/repeated switching, timing, and
Wayland/X11 behavior on Fedora using the checklist.

The ordinary desktop identifier is `com.hew3d.Hew`; it has official updater
endpoints, file associations, and a single-instance plugin. Running an unmodified
experimental binary can share state/routing with an installed Hew. The new
`tauri.edge-alpha.conf.json` uses a distinct identifier/product name, no updater
endpoints, and no bundling. Three tests verify isolation invariants and inherited
hooks/security. The guide launches directly with separate XDG roots, never an
installer, and avoids custom CARGO_TARGET_DIR because the existing sidecar staging
script locates its output at the fixed repository-root target path.

## Verification and portability

- Pinned full frontend: **6,654 tests / 305 files passed**, normal timeout,
  two workers, 351.94 seconds; no concurrent heavy builds.
- Kernel edge-treatment/document specifications: **30 passed**.
- WASM API and mesh-export native suites: **305 passed**; headless API:
  **368 passed**.
- Focused lifecycle/real-WASM frontend: **26 passed**; actual Node/WASM smoke:
  **4 passed**; Chromium/WebGL integration: **2 passed**.
- Frontend production/PWA build, application/test type checking, lint,
  native wasm-api/CLI check, wasm-api Clippy with warnings denied, and Rust
  formatting checks passed. Alpha isolation has three tests in the broad run.
- Original upstream history secret scan: no findings; final source/history
  scans and remote/archive verification are recorded in the delivery receipt.

The final source archive is extracted, inventory/hash-checked, and checked from
that clean source using cached third-party crates and an isolated native target.
This is not a claim that the entire desktop or every target built cold. Full
Rust-workspace, Fedora packaging, and WebKitGTK execution remain unverified.
Raw commands/logs, machine-readable failures, scanner reports, archive receipts,
and final GitHub preservation receipts are retained outside the source tree.

## Reviewable contribution structure

Follow `CONTRIBUTING.md`: coordinate large work with maintainers before an
upstream PR, keep invariants/specs with each change, run `scripts/verify.sh` on a
fully provisioned desktop runner, and use conventional commit subjects and human
DCO sign-offs. No upstream issue/PR is opened by this task.

Recommended layers/dependencies:

1. Shared kernel + straight chamfer/roundover and necessary numerical guards,
   with validator/property specs. Review numerical thresholds explicitly.
2. STL integrity, depending on (1) for its edge-treatment regressions; retain
   export behavior/error tests rather than hiding unsafe triangles.
3. Document adapter + exact snapshot history and stale/reference policies,
   depending on (1).
4. WASM lifecycle/real preview/semantic recording and replay, depending on (3).
5. Unified frontend and localization/registration/resource tests, depending on (4).
6. Demonstrated exception-handling QA fixes and fault-injection coverage.
7. Fedora isolation/handoff/audit/checklist, with the remaining risks explicit.

Existing unrelated history is not rebased or rewritten. New preservation commits
append to the exact upstream baseline and retain original licenses/history.
The private import should keep Actions disabled initially so retained upstream
release/deploy workflows cannot run merely because historical tags/main are
copied. No public fork, release, deployment, force push, or upstream PR is part
of this preservation.

## Remaining risks

The ten earlier async/timing failures have no established root cause despite
clean broad runs. WebKitGTK/Fedora/desktop coexistence remain manual QA items.
Candidate computation is synchronous, group/component editing and owned sketches
are unsupported, axis snaps can win coincident edge picks, and exact-state
staleness remains conservative. The alpha is suitable for controlled testing of
model copies; it is not a production-readiness claim. A follow-up metric accuracy
matrix is separately authorized and follows preservation, without expanding the
geometry algorithms.
