# Edge-treatment application boundary (Phase 4A)

The frontend's modeling boundary is `Scene` in `crates/wasm-api`, not the
headless JSON-RPC dispatcher in `crates/api`. This phase adds Scene methods;
it adds neither UI nor JSON-RPC verbs. All geometry and document mutations
remain in the Phase 1–3 kernel implementations.

## Surface

Rust signatures (wasm-bindgen maps `u64` to JavaScript `bigint`, `u32` to
`number`, and `Result` failures to thrown `"CODE: message"` strings):

```rust
prepare_chamfer(&mut self, object: u64, edge: u64, distance: f64)
    -> Result<u32, ApiError>
prepare_roundover(&mut self, object: u64, edge: u64, radius: f64,
                  max_chord_error: f64) -> Result<u32, ApiError>
edge_treatment_info_json(&self, handle: u32) -> Result<String, ApiError>
edge_treatment_mesh(&self, handle: u32) -> Result<MeshJs, ApiError>
commit_edge_treatment(&mut self, handle: u32) -> Result<DocChangeJs, ApiError>
discard_edge_treatment(&mut self, handle: u32) -> Result<(), ApiError>
```

Lengths are meters, volumes cubic meters. Object and edge arguments are the
existing runtime selection handles. Instance/definition-local selections
are unsupported. Kernel eligibility still restricts treatments to supported
straight convex edges; the binding does not reinterpret geometry.

## Ownership and validity

Each Scene owns at most one candidate. A successful prepare replaces and
frees the previous candidate; failed prepare preserves it. Handles are
process/module-wide monotonic, nonwrapping u32 tokens. They never identify
geometry and cannot be transferred to another Scene. Exhaustion refuses
`CandidateHandleExhausted` instead of reusing tokens.

Inspection is read-only. Commit consumes a matching handle on success **or
failure**, so a failed Apply requires regeneration. Foreign/invalid handles
never evict the live candidate. Discard releases the matching candidate.
Successful document load clears the slot; failed load preserves it. Scene
`free()`/Rust drop releases it automatically; there are no timers or global
candidate registry. Handles are never persisted or recorded.

Commit retains the native logical-document identity, history generation,
and exact canonical-state checks. The Scene additionally checks its hidden
sets, pick scope, active instance, frame-sketch scope, and document epoch.
Hidden targets and out-of-scope targets refuse preparation. Active editing
or drawing contexts and definition members are unsupported. Native caller
transactions and the existing Scene history bracket remain authoritative;
the adapter opens no internal bracket.

Inspection describes the frozen candidate; it does not certify that the
live source is still current. Commit is the source-compatibility authority.
Any canonical document change, even unrelated/view changes, may stale it.
A source changing and then returning to identical bytes through undo/redo
still stales it via native history generation. Matching native document
checkpoints retain Phase 3's logical identity behavior.

## Metadata and warnings

`edge_treatment_info_json` returns:

- `handle`: u32 token.
- `object`, `object_sid`, `edge`: decimal strings (no JS Number precision loss).
- `operation`: `{kind:"chamfer", distance}` or
  `{kind:"roundover", radius, max_chord_error}`.
- `removed_volume`, `expected_removed_volume`: native validation evidence.
- `warnings`: empty or an `AnnotationDetachment` record with `annotations`
  (`annotation` runtime handle and `sid`, both decimal strings),
  `recorded_points_preserved:true`, and `undo_restores_attachments:true`.

Parameters and geometry cannot be changed through these accessors. Generate
another candidate to change dimensions. Topology handles are not durable:
the committed edit preserves document ObjectId/SID, but the UI must discard
old face/edge selections and re-pick after applying the returned DocChange.
Owned sketches refuse rather than being silently reassigned. Arbitrary
client attribute payloads remain opaque; topology remapping is not provided.

## Preview bridge

`edge_treatment_mesh` tessellates the actual native candidate using the
existing tessellator, palette, and `MeshJs` buffer/accessor conventions.
It neither inserts document entities nor populates the source object's
face-range cache/inference scene. The preview has its own topology and must
stay separate from committed-object caches and selection identifiers.

Each call returns independent copied render buffers. The caller owns the
MeshJs wrapper and should call `.free()` after uploading/copying buffers.
Existing copies survive candidate replacement, discard, or commit. They
are render data only and cannot be committed as geometry. A later material
palette edit may affect a newly requested mesh; commit will reject the
stale source. Phase 4B must create and dispose its transient render node,
refresh on new parameters, show warnings, and apply returned DocChange.

## Errors

The existing `CODE: message` convention is preserved. Relevant codes include:

- `UnknownObject`, `UnknownEdge`, `HiddenTarget`.
- `UnsupportedDefinitionMember`, `UnsupportedSelectionContext`,
  `ActiveEditingSession`, `OwnedSketchesUnsupported`.
- `InvalidDistance`, `InvalidRadius`, `InvalidChordError`,
  `ApproximationTooFine`, `InsufficientClearance`, `UnresolvableCut`.
- `InvalidSource`, `UnsupportedSolid`, `NonPerpendicularFaces`,
  `UnsupportedTermination` and native construction/Boolean errors.
- `InvalidCandidate`, `InvalidResult`, `ContainmentFailed`, `VolumeMismatch`.
- `StaleCandidate` versus `InvalidCandidateHandle` (missing, consumed,
  replaced, foreign, or disposed token).
- Tessellation errors use their existing variant codes;
  `CandidateSerialization` reports metadata encoding failure.

Annotation detachment is a structured pre-commit warning, not an error.
A target becoming unsupported after preparation is a stale commit failure.

## Recording and replay

Existing Scene mutations record typed `RecordedCall`s and replay from a
fresh Scene (or a recorded Load). Undo/redo/brackets have existing recorded
variants. This adds one additive `edge_treatment` variant without changing
recording format v2, following the existing additive-variant convention.
Older recordings remain readable; older builds reject this new variant.

Only successful commits record a command: target SID, native source
state_hash, exact world-space edge endpoint pair, and frozen operation
parameters. Prepare, inspect, preview, discard, and failures record nothing.
A replay resolves the target SID, checks the source hash, and requires
exactly one edge with exactly those endpoints (either orientation). It
never guesses by index, nearest point, or tolerance. The source hash is an
additional replay diagnostic, **not** a replacement for the native exact
candidate-state guard. Replay uses the same prepare/commit implementation
under the existing recorder's capture suppression.

Runtime tests compare replayed canonical bytes and hashes for both modes,
load, undo/redo, and a second treatment after topology reconstruction.
Replay remains subject to the existing same-build deterministic-recording
contract; it is not cross-version topology tracking. Unmatched/ambiguous
edges refuse `ReplayEdgeMismatch`; differing sources refuse
`ReplaySourceMismatch`. Edited or partial recordings are not silently
repaired. Existing large-u64 recording literals must remain intact (the
Node replay runner already passes the original JSON to Rust).

No browser/renderer UI lifecycle is implemented here. The next step is a
small frontend adapter around this boundary, with GPU cleanup and real
wasm-bindgen/JavaScript exception/BigInt tests before building the wizard.
