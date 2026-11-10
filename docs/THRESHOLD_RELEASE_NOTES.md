# Threshold

First named source-build alpha of EZO Guitars' modular Hew Edge Treatment foundation.

- Complete convex polygonal cap chamfers and roundovers, including nonuniform polygon corner angles.
- Circular cylinder upper/lower chamfers and faceted roundovers, with compatible opposite-cap sequential operations and independent dimensions.
- Geometry-derived conservative sliders, exact metric/imperial entry, parameter help and safe refusal diagnostics.
- Cancellable worker previews with source identity/generation checks, rapid-edit coalescing, no preview history mutations and stale-result rejection.
- Topology, containment, expected volume and actual-STL approximation checks; independent mesh measurements, undo/redo, persistence and replay tests.

Validation: 45 focused native kernel/document tests, 259 native API tests, 56 mesh-export tests, 6,712 frontend tests across 310 files, 14 browser tests, four Node smoke tests and a production-worker check passed. Production/PWA build, formatting, Clippy, type checks, lint and localization checks passed. These are focused native suites, not a claim that every native workspace executable or Fedora desktop was tested.

This is **source-build only**. No tested Fedora desktop binary is attached: this host lacks GTK3/WebKitGTK 4.1. Keep the existing isolated alpha identity/XDG state and use the no-bundle instructions in `docs/THRESHOLD.md`. Normal Hew is not replaced.

Known limits: mixed/irregular curved boundaries, arbitrary nonconvex bodies and arbitrary sequential/overlapping blends remain unsupported. Circle source resolution and segment/work caps can prevent a requested tolerance. Apply remains synchronous; expensive commits can pause. Worker previews keep JavaScript input responsive, but headless software-rendered frame jitter remains visible in measurements. Native Fedora hardware rendering is unverified. No exact analytic/G2/NURBS/variable-radius capability is claimed.

The repository preserves upstream Hew history, licensing and notices. No upstream merge/rebase or pull request was performed, and the original development repository was not pushed. See the verification report and notices for detailed scope and evidence.
