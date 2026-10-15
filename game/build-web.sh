#!/usr/bin/env bash
set -euo pipefail

game_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
output_dir="${1:-$game_dir/../public}"
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"
cd "$game_dir"

bash "$game_dir/../server/generate-bindings.sh"

npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
cp web/dist/network.js "$output_dir/network.js"

# The CLI must match the version linked into the game, including after lockfile updates.
bindgen_version="$(awk '/name = "wasm-bindgen"/ { found = 1; next } found && /^version = / { gsub(/"/, "", $3); print $3; exit }' Cargo.lock)"
if [[ -z "$bindgen_version" ]]; then
    echo "wasm-bindgen is missing from Cargo.lock" >&2
    exit 1
fi
if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version)" != "wasm-bindgen $bindgen_version" ]]; then
    cargo install wasm-bindgen-cli --version "$bindgen_version" --locked
fi

rustup target add wasm32-unknown-unknown
cargo build --locked --profile web-release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-name stronghold_game --out-dir "$output_dir" \
    "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/web-release/stronghold-game.wasm"
cp web/index.html "$output_dir/index.html"
touch "$output_dir/.nojekyll"
echo "Web build: $output_dir"
