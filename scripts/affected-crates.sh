#!/usr/bin/env bash
# Prints the cargo package selection for the changed files given as
# arguments (paths relative to the repository root), one cargo argument
# per line: `--workspace`, or `--package=<name>` for each workspace package
# that owns a changed file, reads a changed file through `include_str!`,
# `include_bytes!`, `include!`, or `#[path]`, or depends on one of those
# packages transitively. Prints nothing when no workspace package is affected.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

if [[ $# -eq 0 ]]; then
  exit 0
fi

metadata=$(cargo metadata --format-version 1)
root=$(jq -r '.workspace_root' <<<"$metadata")

clippy_target=""
if [[ -L clippy.toml ]]; then
  clippy_target=$(readlink clippy.toml)
fi

package_dirs=$(jq -r '
  .workspace_members as $members
  | .packages[]
  | select(.id as $id | $members | index($id))
  | .manifest_path
  | sub("/Cargo.toml$"; "")
' <<<"$metadata")

includes=$(
  while read -r dir; do
    grep -rlE --include='*.rs' 'include(_str|_bytes)?!|#\[path' "$dir" || true
  done <<<"$package_dirs" | while read -r file; do
    tr '\n' ' ' <"$file" |
      grep -oE 'include(_str|_bytes)?!\([[:space:]]*"[^"]*"|#\[path[[:space:]]*=[[:space:]]*"[^"]*"' |
      sed -E 's/^[^"]*"//; s/"$//' |
      while read -r literal; do
        jq -n --arg file "${file#"$root"/}" --arg literal "$literal" '{file: $file, literal: $literal}'
      done || true
  done | jq -s '.'
)

jq -r \
  --argjson includes "$includes" \
  --arg root "$root" \
  --arg clippy_target "$clippy_target" \
  '
  def normalize:
    split("/")
    | reduce .[] as $segment ([];
        if $segment == "" or $segment == "." then .
        elif $segment == ".." and length > 0 and .[-1] != ".." then .[:-1]
        else . + [$segment]
        end)
    | join("/");
  def dirname: split("/") | .[:-1] | join("/");
  def under($dir): . == $dir or startswith($dir + "/");

  . as $metadata
  | $metadata.workspace_members as $members
  | [ $metadata.packages[]
      | select(.id as $id | $members | index($id))
      | { id, name, dir: (.manifest_path | sub("/Cargo.toml$"; "") | ltrimstr($root + "/")) } ]
    as $packages
  | ($packages | map({ key: .id, value: .name }) | from_entries) as $names
  | [ $includes[]
      | { target: ((.file | dirname) + "/" + .literal | normalize),
          id: (.file as $file | $packages | map(.dir as $dir | select($file | under($dir))) | max_by(.dir | length) | .id) } ]
    as $included_by
  | $ARGS.positional as $files
  | [ $files[]
      | . as $file
      | if ($file == "Cargo.toml" or $file == "Cargo.lock"
            or $file == "rust-toolchain" or $file == "rust-toolchain.toml"
            or $file == "clippy.toml" or $file == ".clippy.toml"
            or ($clippy_target != "" and $file == $clippy_target)
            or ($file | under(".cargo")))
        then "workspace"
        elif ($file | under("crates/swc_isograph_plugin")) then empty
        else
          ($packages | map(.dir as $dir | select($file | under($dir))) | max_by(.dir | length)) as $owner
          | if $owner != null then $owner.id
            elif ($file | under("crates")) then "workspace"
            else empty
            end,
          ($included_by[] | select(.target == $file) | .id)
        end ]
    as $selected
  | if ($selected | index("workspace")) then "--workspace"
    else
      [ $metadata.resolve.nodes[]
        | select(.id as $id | $members | index($id))
        | { id, deps: [ .deps[].pkg ] } ]
        as $nodes
      | { set: ($selected | unique), done: false }
      | until(.done;
          .set as $set
          | ($set + [ $nodes[] | select(any(.deps[]; . as $dep | $set | index($dep))) | .id ] | unique) as $next
          | { set: $next, done: ($next == $set) })
      | .set[]
      | "--package=" + $names[.]
    end
  ' --args "$@" <<<"$metadata"
