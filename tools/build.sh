#!/usr/bin/env bash
# Canonical build for the sandboxed Horizon .hmod runtime (same path as the
# working Zoom .hmod): Rust -> wasm32-unknown-unknown core module ->
# wit_component::ComponentEncoder -> ZIP with horizon-mod.toml + mod.wasm.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

rustup target add wasm32-unknown-unknown

echo "== unit tests: pure free-look logic =="
cargo test -p free-look-logic

echo "== build WASM core =="
cargo build -p free_look --lib --target wasm32-unknown-unknown \
  --message-format=json-render-diagnostics --release
CORE="$ROOT/target/wasm32-unknown-unknown/release/free_look.wasm"
test -f "$CORE" || { echo "missing $CORE" >&2; exit 1; }

echo "== componentize =="
cargo run --release -p componentize -- "$CORE" "$ROOT/mod.wasm"

echo "== verify component contract =="
node "$ROOT/tools/verify.mjs" "$ROOT/mod.wasm"

echo "== mock-host behaviour checks =="
node "$ROOT/tools/test-host.mjs"

echo "== package .hmod =="
python3 "$ROOT/tools/package.py"
