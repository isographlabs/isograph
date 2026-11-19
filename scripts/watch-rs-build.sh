#!/usr/bin/env bash
# The command of bacon's `build` job: build the workspace, then restart a running
# pet-demo daemon so it runs the new binary. A failed build exits before the restart.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)

cd "$repo_root"
cargo build --color always
"$repo_root/scripts/restart-daemon-if-running.sh" "$@"
