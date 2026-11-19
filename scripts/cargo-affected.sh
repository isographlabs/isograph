#!/usr/bin/env bash
# Runs `cargo <subcommand> <selection> <args...>`, where the selection is what
# scripts/affected-crates.sh prints for the files changed between
# PRE_COMMIT_FROM_REF and PRE_COMMIT_TO_REF (deletions included). Without that
# range (`pre-commit run --all-files`, or a run over staged files), the
# selection is the full workspace.
set -euo pipefail

if [[ $# -eq 0 ]]; then
  echo "usage: cargo-affected.sh <cargo subcommand> [args...]" >&2
  exit 2
fi
subcommand=$1
shift

here=$(cd "$(dirname "$0")" && pwd)
cd "$(git rev-parse --show-toplevel)"

selection=()
if [[ -n "${PRE_COMMIT_FROM_REF:-}" && -n "${PRE_COMMIT_TO_REF:-}" ]]; then
  for ref in "$PRE_COMMIT_FROM_REF" "$PRE_COMMIT_TO_REF"; do
    git rev-parse --verify --quiet "$ref^{commit}" >/dev/null || {
      echo "cargo-affected.sh: $ref is not a commit" >&2
      exit 1
    }
  done
  files=()
  while IFS= read -r -d '' file; do
    files+=("$file")
  done < <(git diff --name-only --no-renames --no-ext-diff -z "$PRE_COMMIT_FROM_REF...$PRE_COMMIT_TO_REF")
  if [[ ${#files[@]} -gt 0 ]]; then
    affected=$("$here/affected-crates.sh" "${files[@]}")
    if [[ -n "$affected" ]]; then
      while IFS= read -r arg; do
        selection+=("$arg")
      done <<<"$affected"
    fi
  fi
else
  selection=(--workspace)
fi

if [[ ${#selection[@]} -eq 0 ]]; then
  echo "cargo $subcommand: no affected workspace crates"
  exit 0
fi

echo "cargo $subcommand ${selection[*]} $*"
exec cargo "$subcommand" "${selection[@]}" "$@"
