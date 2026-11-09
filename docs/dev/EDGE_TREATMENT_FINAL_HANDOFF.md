# Edge Treatment final engineering handoff

> Historical baseline report for `34a889c`. The
> [pre-Fedora stabilization report](EDGE_TREATMENT_STABILIZATION.md) supersedes
> the STL preparation policy and verification status. Original measurements are
> retained for reproducibility. The baseline test-typecheck PASS claim was
> incorrect: the preserved log contains TS2783 for a duplicate measurement key;
> stabilization removes that redundant key and reruns the check explicitly.

## Implementation status

Experimental straight convex-edge chamfer and faceted circular roundover are
implemented across the kernel, document/history adapter, WASM candidate lifecycle,
semantic recording/replay, and unified frontend tool. The kernel derives local
orientation from adjacent faces and reuses Profile, extrusion, and explicit
Boolean subtraction. It validates topology, watertightness, containment,
locality, and removed volume before returning a candidate. Preparation does not
mutate the source; successful Apply is one native history action. Failures use
typed refusals. Existing Follow Me behavior and dependencies are unchanged.

Scope remains complete straight edges between perpendicular planar faces of a
single supported convex solid, with perpendicular terminations. Curved paths,
concave fillets, partial-face regions, connected-edge propagation, and arbitrary
solid repair are not implemented. Groups/components and object-owned sketches
have explicit limitations documented in the [API](EDGE_TREATMENT_API.md) and
[UI guide](EDGE_TREATMENT_UI.md).

## Private GitHub preservation

Repository: **[EZOGuitars/Hew3dEdging](https://github.com/EZOGuitars/Hew3dEdging)**.
It was inspected while empty and verified **PRIVATE** under the authenticated
`EZOGuitars` personal account, not assumed organization permissions. The user's
supplied repository replaces the earlier requested name. Creation of that earlier
name had been refused by the integration; supplying this repository resolved the
preservation blocker.

Branch: **`feature/edge-treatment`**. The exact final SHA and archive checksums
are in the external delivery receipt so this source document does not contain a
self-referential commit/archive hash. `main` retains the original upstream
baseline **b2e5baee8b0b84d7eb3d8c8f26ff36aabf7fa42e**; all original 338 reachable
commits and upstream tags were recovered from the formerly shallow clone and
preserved. No existing remote branch was overwritten, no force push was used,
and no public repository or upstream PR was created.

Local `origin` is the private repository; `upstream` separately retains
`https://github.com/hew3d/hew.git`. Fresh clones need to add that upstream remote
explicitly because Git does not clone another checkout's remote configuration.
A separately fetched bare clone verifies the feature commit/tree, original
baseline ancestry, complete tracked file inventory, and upstream history/tags.

Preserved signed-off commits, in dependency order:

| Commit | Responsibility |
|---|---|
| `a351614` | Shared kernel, chamfer/roundover, validators, numerical guards/specs |
| `b0bbbe8` | STL precision, winding, complete small-facet export/regressions |
| `cd2511a` | Document candidates, atomic commit, native history/staleness |
| `810756d` | WASM ownership/lifecycle, semantic recording/replay, API docs |
| `291c7bc` | Frontend controls, previews, registration, localization, runtime tests |
| `48e9823` | Demonstrated exception-cleanup and post-commit reporting/dirty-state fixes |
| `0205f3f` | Engineering audit, isolated desktop config, Fedora guide/checklist |
| `49a3483` | Supplied private repository URL in Fedora instructions |
| Subsequent accuracy commit | Canonical-unit display fix, imperial/metric measurements and regressions |
| Subsequent handoff commit | Consolidated report and final Fedora/manual handoff |

The first seven commits are preserved unchanged. All new commits use the
configured human contributor identity, conventional subjects, and DCO sign-offs;
original upstream history is not rewritten. GitHub's Actions-settings API denied
the attempt to disable workflows. No workflow runs were listed after the initial
history import. That is an observed check, **not** confirmation that Actions are
disabled. No release/deployment was intentionally requested. Subsequent feature
pushes do not match the retained upstream main/tag push triggers.

## Source and archive verification

The pre-QA source snapshot contained 1,808 files and was extracted and compared
byte for byte. Its SHA-256 is
`4222fdaf8280ad7e96789f79293db0977456d9896ac05dfd3c23a8834f1179ca`
(37,383,777 bytes). The Phase 4C QA snapshot contained 1,813 files; its SHA-256 is
`9de1f3e6e6312ad56fbf5255e0277fb7fad72ad4a0535f0d11463728537846e0`
(37,325,412 bytes). Clean-source `cargo check -p kernel -p wasm-api --locked`
passed from that extraction using cached third-party crates and an isolated
native target (58.72 seconds).

The final delivery includes a fresh complete source archive plus a verified full
Git bundle, with inventory/hash comparisons and a clean-source compile check.
The final receipt provides exact file counts, sizes, SHA-256 values, commit SHA,
and remote verification. Archives, patches, and raw logs remain preserved outside
the repository; source code, tests, measured CSV/JSON, and documentation are
tracked. Generated builds/caches, credentials, authentication config, temporary
files, and intentionally ignored internal design material are excluded.

An ignored-file inventory found no files outside the known generated/cache and
internal-design prefixes. Gitleaks 8.30.1 found no secrets in the recovered
upstream history, Phase 4C full source, or final Phase 4C history. Final accuracy
source/history scans are included in the delivery receipt. Scanner outputs are
redacted; no credentials are copied into the repository.

## Verification status

**PASS** means executed successfully; **FAIL** means an executed check failed;
**BLOCKED** means the needed environment is unavailable; **NOT RUN** means not
attempted. A failed broad gate is not promoted to PASS by narrower tests.

| Status | Check | Result / practical limit |
|---|---|---|
| PASS | Original pinned Phase 4C frontend | **6,654 tests, 305 files**, 351.94 s, normal timeout, two workers |
| PASS | Final full frontend after unit fix | **6,683 tests, 306 files**, 343.94 s; no skipped tests, no concurrent heavy builds |
| PASS | Focused unit/real-WASM/panel/units regression | **241 tests, 5 files**, including 29 dimensional real-WASM cases |
| PASS | Fresh native dimensional/STL matrix | **3 specifications**, 365 successful operations plus explicit invalid/cap refusals |
| PASS | Kernel treatment/document regression | **30 tests** (19 geometry, 11 document) |
| PASS | Earlier pinned WASM/mesh-export native regression | **305 tests**, before the new dimensional specifications |
| PASS | Headless API regression | **368 tests** |
| PASS | Actual Node/WASM ownership/commit/replay smoke | **4 tests** |
| PASS | Chromium/WebGL pointer/React integration | **3 tests**, including metric unit switching and renderer-refresh fault |
| PASS | Application/test type checks and lint | No errors; no dependencies or lockfiles changed |
| PASS | Rust formatting and focused Clippy | Warnings denied |
| PASS | Phase 4C production/PWA build and native API/CLI checks | Executed; desktop host itself was not built |
| FAIL | `scripts/verify.sh` broad gate | Reached workspace tests, then disk exhaustion during linking of `api` test binaries `commands_view` / `commands_library`; linker bus errors, no complete workspace test result |
| FAIL | Strict exported chord bound, opposite-corner 4-inch case | 0.010002231570536 mm exceeds 0.01 mm by 0.000002231570536 mm; float32-budget test passes, strict geometric inequality does not |
| BLOCKED | Complete workspace verification in this disk allocation | Reproducible generated binaries were removed to recover space; patches/history/archives/logs retained. Focused affected task checks were rerun |
| BLOCKED | WebKitGTK/Tauri desktop execution here | Managed host is Debian; GTK3/WebKitGTK 4.1 development/runtime packages are absent |
| NOT RUN | Fedora 45 KDE/laptop build and manual acceptance | Must run on the target laptop; no remote installation attempted |
| NOT RUN | Comprehensive `verify-full.sh` / all replay/visual-golden lanes | The incomplete broad gate is a prerequisite; targeted real replay and Chromium checks do not substitute for every lane |

Final focused native WASM/mesh-export regression: **308 passed** (255 WASM,
53 export). Pinned wasm-pack **0.15.0** rebuilt the actual Node and web packages
with matching wasm-bindgen **0.2.127**, `--mode no-install --no-opt`; metadata-only
stripping used Binaryen before the final successful production/PWA build.
This verifies release Rust compilation and runtime bindings, not a claim that
the normal size-optimization pass was run. The rebuilt Node package passed the
four-test smoke and the 241-test focused frontend set; the rebuilt web package
passed all three Chromium integration tests. The 29-case real-WASM dimensional
suite also independently checks actual chord edges, exported dimensions,
orientation/manifold edges, and volume, including all five chord tolerances and
both explicit external-unit import workflows. Test type checks and lint passed
again after those additional measurement assertions.

The **ten earlier asynchronous/timing failures remain unresolved as to root
cause**. Their original five-second timeout is unchanged, assertions were not
weakened, and clean full-suite runs do not prove the absence of scheduling,
ordering, or cleanup defects. The [Phase 4C audit](EDGE_TREATMENT_PHASE4C.md)
lists every original failure and distinguishes the two confirmed issues:
missing Rust PATH and stale generated native-menu artifacts. Both were corrected.
Preview disposal fault tests establish ownership/cleanup contracts, not a
measured proof of zero long-running GPU growth on WebKitGTK.

## Dimensional acceptance

The [detailed accuracy report](EDGE_TREATMENT_DIMENSIONAL_ACCURACY.md),
[365-row measurements](EDGE_TREATMENT_METRIC_MEASUREMENTS.csv), and
[real-WASM unit/decimal measurements](EDGE_TREATMENT_METRIC_WASM.json) are the
numerical evidence. They include independent BRep and decoded STL measurements,
not just successful conversion or operation-reported values.

- Fresh metric cubes: **10, 25, 100, 300, 1000 mm**.
- Fresh imperial cubes: **12 inches per side**, radii **0.125–4 inches**.
- Five chord tolerances: **0.01, 0.025, 0.05, 0.1, 0.25 mm**.
- Maximum absolute dimensional error across the tested BRep/STL/WASM data:
  **about 0.0000122071 mm = 4.806e-7 inches**. Origin-corner BRep setbacks and
  extents had zero residual against their requested f64 references; this is
  not a universal exactness guarantee.
- Maximum BRep chord deviation: **0.246233188097 mm** under the 0.25 mm
  request; maximum decoded STL deviation: **0.246233358054 mm**. All exports
  in the native matrix remained below their own requested chord tolerance.
- **Strict STL chord limitation:** the real-WASM opposite-corner 4-inch radius
  case measures **0.010002231570536 mm** against a **0.01 mm** request,
  an excess of **0.000002231570536 mm**. The explicit-unit roundtrips retain it.
  It is within the documented float32 error budget, but fails the strict STL
  bound; JSON records the noncompliance rather than calling it exact success.
  The required 6.35 mm case measures **0.009997820746607 mm**, below its bound.
- The actual-versus-analytic chord formula differed by at most
  **1.55041e-14 mm**; maximum used segments **56 / 128**. A deliberately finer
  50 mm / 0.0001 mm request correctly refused with `ApproximationTooFine`.
- Ideal circular and expected faceted removed volumes are separate from measured
  numerical error. Maximum BRep removed-volume error **4.10181e-7 mm³**;
  decoded STL total-volume comparison **3.40220 mm³**, predominantly whole-cube
  float32 quantization. Comparing exported source/result instead gives maximum
  cut-volume error **0.0326116 mm³**. Neither quantity is hidden as rounding.
- Unit conversion definition: **1 inch = exactly 25.4 mm**. Maximum measured
  conversion discrepancy **3.46945e-15 mm**; imperial/metric paired mesh
  difference **0 mm** in the dataset. Repeating switches 100 times left the
  canonical dimension unchanged, including values beyond display precision.

For the mission acceptance case, a **0.250-inch** and **6.350 mm** roundover
produce matching geometry. Native tangent setbacks are **6.35 mm**. Independent
actual-WASM STL setbacks at the opposite cube corner are **6.349987792969 mm**
and **6.349999904633 mm**, maximum error **0.000012207031252 mm**. At 0.01 mm
chord tolerance, N=14; native maximum chord deviation **0.009989724636865 mm**.
Ideal removal is **2637.520523193 mm³**, expected faceted removal
**2657.760562935 mm³**, independently measured BRep removal
**2657.760562949 mm³**. Faceting excess is deliberate, not conversion error.

Every native matrix STL was decoded and checked for triangle completeness,
watertight paired edges, outward orientation, dimensions, volume, and precision.
STL carries no units; Hew emits millimeter coordinates. Tests specify external
import scales for both millimeter-coordinate and explicitly converted
inch-coordinate STL and remeasure after import/export. No unintended 25.4×
scaling occurred. External viewers must still be told the coordinate units.

## Unit-display fix and changed files

The runtime correction is confined to `app/src/tools/EdgeTreatmentTool.ts`.
Canonical meters are retained while existing formatting displays the chosen
unit; actual input edits replace the canonical value. Unit switches show
`6.35 mm` → `0.25"` → `6.35 mm` and do not feed display rounding back into the
kernel. No geometry algorithm, serialization, WASM API, or Follow Me behavior
changed for this correction.

Regression changes: `app/src/tools/EdgeTreatmentTool.test.ts`, new
`app/src/tools/EdgeTreatmentMetric.wasm.test.ts`, expanded
`app/e2e/edge-treatment.spec.ts`, and new
`crates/mesh-export/tests/metric_accuracy.rs`. Measurement reports/data and the
Fedora guide/checklist complete the change. The earlier Phase 4C lifecycle fixes
are separately reviewable in `48e9823`.

## Fedora 45 handoff and stop point

Follow the [exact build/launch guide](FEDORA45_EDGE_ALPHA.md), including its
short copy/paste sequence, pinned toolchain, isolated source/build/state
locations, expected executable paths, log capture, and cleanup procedure.
Target hardware: ThinkPad P14s Gen 7 AMD, Ryzen AI 7 PRO 450, 32 GB RAM, Fedora
45 KDE. The guide does not overwrite or install over the existing Hew copy.
The separate alpha identifier, updater configuration, no-bundle build, and XDG
roots still require real Fedora/WebKitGTK coexistence verification.

Clone/check out with authenticated HTTPS:

```bash
gh auth login --hostname github.com --git-protocol https
gh auth setup-git
git clone --branch feature/edge-treatment \
  https://github.com/EZOGuitars/Hew3dEdging.git "$HOME/src/hew-edge-treatment-alpha"
cd "$HOME/src/hew-edge-treatment-alpha"
git checkout feature/edge-treatment
git remote add upstream https://github.com/hew3d/hew.git
git rev-parse HEAD
```

Compare HEAD with the final delivery SHA before building. Run the
[manual checklist](EDGE_TREATMENT_ALPHA_CHECKLIST.md), prioritizing launch,
selection, both previews, dimensions/units, invalid input, single Apply, Cancel,
Undo/Redo, save/reopen, STL, resource lifecycle, and unsupported geometry.
It includes two fresh 12-inch cubes for equivalent 0.250-inch / 6.35 mm trials.

Upstream contribution remains a **reviewable experimental branch**, not a
production-ready submission. Maintainer coordination, a fully provisioned
`verify-full.sh`/CI run, actual WebKitGTK/Fedora tests, and the ten unresolved
historical timing failures remain open. No upstream PR was opened.

Development stops here for engineering review and Fedora results. No curved-edge
or concave-geometry phase is started.
