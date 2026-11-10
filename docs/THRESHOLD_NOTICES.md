# Threshold licensing and source provenance

Threshold is a modified development version of Hew, maintained for EZO Guitars Edge Treatment testing. Original copyright and attribution notices remain in their source files and the full upstream Git ancestry is retained. This is not an official Hew release or a claim of upstream endorsement.

The workspace and frontend declare **AGPL-3.0-only**. The actual upstream license text is [LICENSE](../LICENSE), with the additional [Hew Plugin API Exception](../LICENSE-EXCEPTION). The exception is not a replacement proprietary license for modifications to Hew's core. Copies in `crates/wasm-api/` are retained. No license text or copyright notice has been rewritten.

Hew's brand assets have separate **CC BY 4.0** terms in [brand/LICENSE](../brand/LICENSE). Attribution to Hew and its upstream authors, including Kurt Granroth, is retained. SketchUp is a trademark of Trimble; this fork is not affiliated with Trimble or an SDK-derived implementation.

Included third-party source/assets retain their existing notices:

- `crates/vendor/dae-parser/LICENSE-APACHE` and `LICENSE-MIT`.
- `crates/vendor/minidom-14/LICENSE`.
- `app/src/assets/fonts/onest/OFL.txt`.
- `app/src/assets/fonts/space-mono/OFL.txt`.
- `app/src/assets/fonts/lora/OFL.txt`.
- `app/src/assets/fonts/varela-round/OFL.txt`.
- `app/src/assets/fonts/alfa-slab-one/OFL.txt`.

This delivery distributes repository source and history, not a bundled desktop executable or compiled npm/Cargo dependency distribution. `Cargo.lock`, shell lockfiles, `pnpm-lock.yaml` and manifests preserve build dependency provenance; registry dependencies are fetched under their own package licenses. Their installed caches and credentials are excluded. Before separately redistributing a compiled binary/bundle, inventory its actual included dependencies and accompanying licenses/notices. No blanket legal compatibility or completed desktop redistribution audit is claimed.

The source archive must contain the complete tagged tracked tree, including these notices and build scripts. Providing a private repository does not remove applicable copyright, attribution or source obligations. Preserve AGPL source availability requirements when conveying modified builds or providing covered network access. The GitHub release and source-build instructions identify the exact corresponding source rather than linking a different branch tip.

Development provenance: `EZOGuitars/Hew3dEdging`, `feature/edge-treatment`. Release destination: `EZOGuitars/HewEdgingThreshold`. Official upstream: `hew3d/hew`. Preserve these relationships and original upstream history when preparing future contributions; no force-push or upstream PR is part of Threshold publication.
