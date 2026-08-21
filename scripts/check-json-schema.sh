#!/usr/bin/env bash

set -euo pipefail

cargo build --bin build_json_schema
./target/debug/build_json_schema
git diff --exit-code -- libs/isograph-compiler/isograph-config-schema.json
