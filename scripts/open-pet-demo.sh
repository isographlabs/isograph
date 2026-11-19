#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
extension_dir="$repo_root/vscode-extension"
pet_demo_dir="$repo_root/demos/pet-demo"
mac_code="/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"

if command -v code >/dev/null 2>&1; then
  code_cli=$(command -v code)
elif [[ -x "$mac_code" ]]; then
  code_cli="$mac_code"
else
  echo "open-pet-demo: VS Code CLI not found. Looked for 'code' on PATH and $mac_code." >&2
  exit 1
fi

if [[ ! -x "$extension_dir/node_modules/.bin/esbuild" ]]; then
  echo "open-pet-demo: $extension_dir/node_modules/.bin/esbuild is missing. Run 'npm install' in $extension_dir." >&2
  exit 1
fi

(cd "$repo_root" && cargo build -p ts_graphql_react_isograph_cli)

(cd "$extension_dir" && ./node_modules/.bin/esbuild \
  ./src/extension.ts \
  --bundle \
  --outfile=out/extension.js \
  --external:vscode \
  --format=cjs \
  --platform=node \
  --sourcemap)

"$code_cli" --extensionDevelopmentPath="$extension_dir" "$pet_demo_dir"
