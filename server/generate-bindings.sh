#!/usr/bin/env bash
set -euo pipefail
server_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$server_dir"
cargo build --locked --release --target wasm32-unknown-unknown
module_path="target/wasm32-unknown-unknown/release/stronghold_server.wasm"
spacetime generate --lang rust --out-dir ../game/src/module_bindings --bin-path "$module_path"
spacetime generate --lang typescript --out-dir ../game/web/src/module_bindings --bin-path "$module_path"
# Format generated Rust files without modifying handwritten game code in CI.
rustfmt --edition 2021 ../game/src/module_bindings/mod.rs
