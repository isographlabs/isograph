#!/usr/bin/env bash
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "precommit-in-worktree: not a git worktree" >&2
  exit 1
fi

PATH="${PATH}:/opt/homebrew/bin:${HOME}/.local/share/mise/shims"

common=$(cd "$(git rev-parse --path-format=absolute --git-common-dir)" && pwd -P)
git_dir=$(git rev-parse --path-format=absolute --git-dir)
toplevel=$(cd "$(git rev-parse --show-toplevel)" && pwd -P)
wt="$(cd "$common/.." && pwd -P)-precommit"

if [[ "$toplevel" == "$wt" ]]; then
  echo "precommit-in-worktree: refusing to commit from the dedicated precommit worktree $wt" >&2
  exit 1
fi

if [[ -n "$(git ls-files --unmerged)" ]]; then
  echo "Unmerged files. Resolve before committing." >&2
  exit 1
fi

intent=$(git diff --no-ext-diff --ignore-submodules --diff-filter=A --name-only || true)
if [[ -n "$intent" ]]; then
  echo "Intent-to-add files in the index. git add them for real, or git reset them, before committing." >&2
  echo "$intent" >&2
  exit 1
fi

lockfile="$common/precommit-worktree.lock"

if [[ -z "${PRECOMMIT_WORKTREE_LOCK_FD:-}" ]]; then
  if ! command -v python3 >/dev/null; then
    echo "precommit-in-worktree: python3 not on PATH" >&2
    exit 1
  fi
  exec python3 -c '
import fcntl, os, sys
lock_path = sys.argv[1]
script = sys.argv[2]
rest = sys.argv[3:]
fd = os.open(lock_path, os.O_CREAT | os.O_RDWR, 0o644)
os.set_inheritable(fd, True)
fcntl.flock(fd, fcntl.LOCK_EX)
os.environ["PRECOMMIT_WORKTREE_LOCK_FD"] = str(fd)
os.execvp("bash", ["bash", script] + rest)
' "$lockfile" "$0" "$@"
fi

local_env_vars=$(git rev-parse --local-env-vars)
index_file=$(git rev-parse --path-format=absolute --git-path index)
tree=$(git write-tree)
parents=()
from_ref_args=(--all-files)
if git rev-parse --verify --quiet HEAD >/dev/null; then
  parent=$(git rev-parse HEAD)
  parents=(-p "$parent")
  from_ref_args=(--from-ref "$parent" --to-ref HEAD --files .)
fi
if [[ -f "$git_dir/MERGE_HEAD" ]]; then
  while read -r p; do
    parents+=(-p "$p")
  done <"$git_dir/MERGE_HEAD"
fi
if [[ ${#parents[@]} -gt 0 ]]; then
  snapshot=$(git commit-tree --no-gpg-sign "$tree" "${parents[@]}" -m "precommit snapshot")
else
  snapshot=$(git commit-tree --no-gpg-sign "$tree" -m "precommit snapshot")
fi

unset $local_env_vars

if [[ -e "$wt/.git" ]]; then
  got=$(cd "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" && pwd -P)
  if [[ "$got" != "$common" ]]; then
    echo "precommit-in-worktree: $wt exists and is not this repo's precommit worktree" >&2
    exit 1
  fi
  git -C "$wt" reset --hard "$snapshot"
  git -C "$wt" clean -fd
  for dir in "$wt"/crates/*/ "$wt"/libs/*/; do
    [[ -d "$dir" ]] || continue
    rel=${dir#"$wt"/}
    rel=${rel%/}
    if [[ -z "$(git -C "$wt" ls-files -- "$rel/Cargo.toml" "$rel/package.json")" ]]; then
      git -C "$wt" clean -ffdXq -- "$rel"
      if [[ -d "$dir" && -z "$(ls -A "$dir")" ]]; then
        rmdir "$dir"
      fi
    fi
  done
elif git --git-dir="$common" worktree list --porcelain | grep -F -x "worktree $wt" >/dev/null; then
  git --git-dir="$common" worktree remove --force "$wt"
  git --git-dir="$common" worktree add --detach --lock --reason "pre-commit checks" "$wt" "$snapshot"
elif [[ -e "$wt" ]]; then
  echo "precommit-in-worktree: $wt exists and is not this repo's precommit worktree" >&2
  exit 1
else
  git --git-dir="$common" worktree add --detach --lock --reason "pre-commit checks" "$wt" "$snapshot"
fi

if [[ -f "$wt/pnpm-lock.yaml" ]]; then
  if ! command -v pnpm >/dev/null; then
    echo "precommit-in-worktree: pnpm not on PATH" >&2
    exit 1
  fi
  (cd "$wt" && pnpm install --frozen-lockfile)
fi

if ! command -v pre-commit >/dev/null; then
  echo "precommit-in-worktree: pre-commit not on PATH" >&2
  exit 1
fi

set +e
(cd "$wt" && pre-commit run "${from_ref_args[@]}" --hook-stage pre-commit)
status=$?
set -e
if [[ "$status" -ne 0 ]]; then
  if ! git -C "$wt" diff --quiet; then
    echo "Hooks modified the snapshot. Those fixes were not copied back." >&2
    git -C "$wt" --no-pager diff >&2
  fi
  exit "$status"
fi

tree_now=$(GIT_INDEX_FILE="$index_file" git --git-dir="$git_dir" write-tree)
if [[ "$tree_now" != "$tree" ]]; then
  echo "index changed while pre-commit ran; aborting commit" >&2
  exit 1
fi
