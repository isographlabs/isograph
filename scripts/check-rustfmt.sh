#!/usr/bin/env bash

set -euo pipefail

cargo fmt -- --check
cargo fmt --manifest-path crates/isograph_cli/Cargo.toml -- --check
