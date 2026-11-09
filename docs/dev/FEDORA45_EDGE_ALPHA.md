# Fedora 45: isolated Edge Treatment alpha

This is an experimental **source launch**, not an installation. Fedora 45 and
WebKitGTK have not been executed in the managed Debian QA environment. The
commands below follow the actual repository scripts and the current [Tauri
Fedora prerequisites](https://v2.tauri.app/start/prerequisites/#linux).

## 1. System prerequisites

On conventional Fedora (not an immutable rpm-ostree variant):

```bash
sudo dnf install git gh pkgconf-pkg-config curl wget file \
  webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel \
  librsvg2-devel libxdo-devel
sudo dnf group install c-development
pkg-config --modversion webkit2gtk-4.1 gtk+-3.0
```

These are upstream Tauri's Fedora package names; the commands have not been run
on Fedora 45 here. Stop and record the error if your Fedora repositories cannot
provide the indicated WebKitGTK **4.1** ABI. Do not substitute a different ABI
merely to make package installation finish.

Install [rustup](https://rustup.rs/) and [fnm](https://github.com/Schniz/fnm#installation)
using their official user-level instructions. The required pins are:

| Tool | Repository source | Value |
|---|---|---|
| Rust | `rust-toolchain.toml` | 1.97.1 |
| Rust components | same file | clippy, rustfmt |
| WASM target | same file | wasm32-unknown-unknown |
| Node | `.node-version` | 26.7.0 |
| pnpm | root `package.json` | 11.20.0 |
| wasm-pack | `scripts/install-wasm-pack.sh` | 0.15.0 |
| wasm-bindgen CLI | `Cargo.lock` | 0.2.127 |

## 2. Clone the private repository into a new directory

Authenticate as an account that can read the private repository. No cloud
credentials need to be copied to Fedora:

```bash
gh auth login --hostname github.com --git-protocol https
gh auth setup-git
mkdir -p "$HOME/src"
gh repo clone EZOGuitars/hew-edge-treatment "$HOME/src/hew-edge-treatment-alpha" \
  -- --branch feature/edge-treatment
cd "$HOME/src/hew-edge-treatment-alpha"
git checkout feature/edge-treatment
git remote add upstream https://github.com/hew3d/hew.git
git status --short
git rev-parse HEAD
```

Use a different empty destination if that directory already exists; do not
reset an existing checkout. Compare HEAD with the final preservation receipt.
A fresh Git clone does not copy another machine's remote configuration, hence
the explicit `upstream` command above. The private repository is a standalone
copy retaining upstream history, not a public GitHub fork.

## 3. Activate the pinned tools

```bash
rustup toolchain install 1.97.1 --component clippy --component rustfmt \
  --target wasm32-unknown-unknown
eval "$(fnm env --shell bash)"
fnm install "$(cat .node-version)"
fnm use "$(cat .node-version)"
npm install --global pnpm@11.20.0
scripts/install-wasm-pack.sh
rustc --version
node --version
pnpm --version
wasm-pack --version
pnpm install --frozen-lockfile
```

If matching wasm-bindgen is not already available, wasm-pack normally installs
it. For an offline/no-install build, provision version 0.2.127 explicitly first.
Do not edit lockfiles or dependency versions to bypass an installation error.

## 4. Build and test the real WASM and frontend

```bash
wasm-pack build crates/wasm-api --target nodejs \
  --out-dir ../../tools/replay-runner/pkg-node
node --test tools/replay-runner/edge-treatment-smoke.mjs
wasm-pack build crates/wasm-api --target web \
  --out-dir ../../app/src/wasm/pkg
pnpm --dir app typecheck
pnpm --dir app typecheck:test
pnpm --dir app lint
pnpm --dir app test --maxWorkers=2
cargo test -p kernel --locked --test edge_treatment_specs \
  --test document_edge_treatment_specs
cargo test -p wasm-api -p mesh-export -p api --locked
```

Keep the broad frontend run free of concurrent Rust/optimizer builds. Its normal
five-second test timeout is unchanged. Review failures; an isolated pass alone
is not proof of their cause.

Normal wasm-pack uses wasm-opt for release sizing. If you deliberately build
with `--mode no-install --no-opt`, name/debug metadata may exceed the existing
PWA precache limit. Provision Binaryen and strip metadata before building the
frontend rather than raising the cache limit:

```bash
wasm-opt app/src/wasm/pkg/wasm_api_bg.wasm --strip-debug --strip-producers \
  -o app/src/wasm/pkg/wasm_api_bg.stripped.wasm
mv app/src/wasm/pkg/wasm_api_bg.stripped.wasm app/src/wasm/pkg/wasm_api_bg.wasm
```

This removes metadata, not geometry operations. Re-run the smoke after changing
build tooling. Do not silently skip a real WASM runtime failure.

## 5. Build a separate desktop executable; do not install it

The normal config uses `com.hew3d.Hew`, an updater endpoint for the official
application, and `.hew` file associations. Running that config alongside an
installed Hew can share application state or single-instance routing. Use the
provided overlay, which gives the alpha its own app ID/product name, disables
update endpoints, and disables bundling. Original application configuration is
unchanged. The launch below additionally isolates all XDG state directories.

```bash
# Use the default target locations inside this isolated checkout. The existing
# staging script copies repo-root target/release/hew-cli; it does not honor a
# custom CARGO_TARGET_DIR when locating the built sidecar.
unset CARGO_TARGET_DIR
pnpm --dir shells/tauri exec node scripts/stage-cli-sidecar.mjs
export HEW_BETA_SHA="$(git rev-parse --short=12 HEAD)"
pnpm --dir app build
pnpm --dir shells/tauri exec tauri build \
  --config src-tauri/tauri.edge-alpha.conf.json --no-bundle
```

The Tauri CLI command and flags are from its actual `build --help`; it runs the
normal `beforeBuildCommand` (`pnpm --dir ../../app build`). The sidecar staging
command must run first, as declared by the existing desktop package scripts.

Expected outputs, with the default targets:

- CLI: `target/release/hew-cli`.
- Staged sidecar: `shells/tauri/src-tauri/binaries/hew-cli-<host-triple>`.
- Desktop: `shells/tauri/src-tauri/target/release/hew`.

Do not run an RPM installer, copy these binaries over an installed Hew, or
register a desktop/file association. The overlay plus `--no-bundle` avoids
creating an installer. Launch the absolute path from the new checkout.

## 6. Launch with separate state and capture logs

```bash
ALPHA_ROOT="$PWD"
ALPHA_STATE="$HOME/.local/state/hew-edge-treatment-alpha"
mkdir -p "$ALPHA_STATE"/{config,data,cache,state,logs,models}
set -o pipefail
env XDG_CONFIG_HOME="$ALPHA_STATE/config" \
    XDG_DATA_HOME="$ALPHA_STATE/data" \
    XDG_CACHE_HOME="$ALPHA_STATE/cache" \
    XDG_STATE_HOME="$ALPHA_STATE/state" \
    RUST_LOG=info \
    "$ALPHA_ROOT/shells/tauri/src-tauri/target/release/hew" \
    2>&1 | tee "$ALPHA_STATE/logs/console.log"
```

Keep the installed Hew closed for the first alpha test. On later coexistence
tests, confirm launching the alpha does not focus or forward a file to the
installed application. App ID is `com.hew3d.Hew.EdgeTreatmentAlpha`. XDG paths
above also protect WebKit local storage, preferences, recents, and recovery data
from accidental reuse of normal application directories. Actual WebKitGTK and
single-instance coexistence still need to be verified on Fedora.

The UI's beta/version indicator or Welcome screen should show
`1.4.0+beta.<12-character-SHA>`, using the existing `HEW_BETA_SHA` build stamp.
Record both that stamp and the full `git rev-parse HEAD`. The native package's
base version remains 1.4.0; it alone does not identify the experimental commit.

The backend writes `diagnostic.log` (and `diagnostic.1.log`) under Tauri's app-log
directory; reproducers live below `reproducers/`. Locate them within the isolated
state roots without assuming a WebKitGTK/Tauri platform-specific layout:

```bash
find "$ALPHA_STATE" -type f \
  \( -name 'diagnostic*.log' -o -path '*/reproducers/*' \) -print
```

Record Fedora release, session type (Wayland/X11), GPU/driver, WebKitGTK version,
commit/stamp, input dimensions, and exact UI error. Use model **copies** in
`$ALPHA_STATE/models`; isolation does not protect a model file you explicitly
open and overwrite. Help → Report Bug can prepare a local diagnostic bundle;
review it before sending, and do not automatically upload private models/logs.

Proceed through [the manual checklist](EDGE_TREATMENT_ALPHA_CHECKLIST.md).
