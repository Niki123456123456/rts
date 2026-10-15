#!/usr/bin/env bash
set -euo pipefail
server_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$server_dir"
cargo build --locked --release --target wasm32-unknown-unknown
spacetime publish --server "${SPACETIMEDB_URI:-http://127.0.0.1:3000}" \
    --bin-path target/wasm32-unknown-unknown/release/stronghold_server.wasm \
    "${SPACETIMEDB_DATABASE:-stronghold}" "$@"
