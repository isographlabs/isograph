#!/usr/bin/env bash
# Restart the isograph daemon for one config, only if that daemon is running.
# The config defaults to demos/pet-demo/isograph.config.json.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
isograph="$repo_root/target/debug/isograph"
config="${1:-$repo_root/demos/pet-demo/isograph.config.json}"

if "$isograph" status --config "$config" >/dev/null 2>&1; then
  "$isograph" restart --config "$config"
fi
