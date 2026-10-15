#!/usr/bin/env bash
set -euo pipefail
server_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
set -a
source "$server_dir/maincloud.env"
set +a
export SPACETIMEDB_CONFIG_PATH="$server_dir/.spacetime/maincloud-cli.toml"
exec bash "$server_dir/publish.sh" "$@"
