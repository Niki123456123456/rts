#!/usr/bin/env bash
set -euo pipefail
server_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec spacetime start --data-dir "$server_dir/.spacetime" --listen-addr "${SPACETIMEDB_LISTEN:-0.0.0.0:3000}"
