# Pre-commit in a dedicated worktree

`git commit` in any worktree of this repo snapshots the index as a commit object, checks that commit out in a dedicated worktree, and runs the existing pre-commit hooks there. The invoking working tree, index, `HEAD`, and stash reflog are not modified. If the hooks pass and the invoking index still has the snapshot tree, git creates the commit in the invoking worktree.

## What the user does

```
$ git add crates/isograph_parser/src/chunk.rs
$ git commit -m "chunk: drop empty contents"
```

The invoking worktree stays as it was after `git add`. Other files left dirty stay dirty. `git stash list` is unchanged. Hook output is the same `pre-commit` output as today. On success git creates the commit from the invoking index. On failure git does not create the commit; the invoking worktree and index are still unchanged.

If a hook rewrites files in the dedicated worktree (`end-of-file-fixer`, `trailing-whitespace`, `mixed-line-ending`), those rewrites stay there. The invoking worktree does not receive them. The hook prints that diff and exits 1. Fix the files in the invoking worktree, `git add`, commit again.

`git commit --no-verify` skips this, as today.

The first run after the dedicated worktree is created compiles from a cold `target/`. Later runs reuse that `target/` and `node_modules/`.

## Snapshot

The snapshot is the index, which is what `git commit` would write. Plumbing, no checkout, no stash:

```bash
# from scripts/precommit-in-worktree.sh
tree=$(git write-tree)
parent=$(git rev-parse HEAD)
parents=(-p "$parent")
if [[ -f "$git_dir/MERGE_HEAD" ]]; then
  while read -r p; do
    parents+=(-p "$p")
  done <"$git_dir/MERGE_HEAD"
fi
snapshot=$(git commit-tree "$tree" "${parents[@]}" -m "precommit snapshot")
```

`git write-tree` fails on unmerged entries. The script checks `git ls-files --unmerged` first and exits 1 with `Unmerged files. Resolve before committing.`

Intent-to-add (`git add -N`) is not a committable index. The script checks `git ls-files -v` for a leading lowercase status and exits 1 with those paths.

The snapshot commit is not on a branch. The dedicated worktree's detached `HEAD` keeps it reachable until the next reset.

## Dedicated worktree

Path: sibling of the clone, derived from the common git dir, not from `PWD`.

```bash
# from scripts/precommit-in-worktree.sh
common=$(git rev-parse --path-format=absolute --git-common-dir)
wt="$(cd "$common/../.." && pwd)/i2-precommit"
```

For `/Users/rbalicki/code/i2/.git` that is `/Users/rbalicki/code/i2-precommit`. The same path is used when the hook fires from `i2-mise` or from a grok isolated worktree: those share `git-common-dir`.

Not `i2-mise`. That worktree is the `i2-mise` branch. Not a new worktree per commit: `target/` and `node_modules/` stay in this one directory so clippy and tests stay incremental.

Create once, locked so `git worktree prune` will not drop it:

```bash
# from scripts/precommit-in-worktree.sh
git --git-dir="$common" worktree add --detach --lock --reason "pre-commit checks" "$wt" "$snapshot"
```

On later runs:

```bash
# from scripts/precommit-in-worktree.sh
git -C "$wt" reset --hard "$snapshot"
git -C "$wt" clean -fd
```

`git clean -fd` without `-x` leaves gitignored `target/` and `node_modules/`.

If `$wt` exists and is not this repo's worktree, exit 1 and name the path. Do not reuse it.

If the hook's worktree is `$wt` (someone committed from the dedicated checkout), exit 0.

## Git env the hook is given

Git sets `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` for the invoking worktree. Snapshot commands use that env. Commands against `$wt` do not.

```bash
# from scripts/precommit-in-worktree.sh
local_env_vars=$(git rev-parse --local-env-vars)
tree=$(git write-tree)
# ... commit-tree ...
unset $local_env_vars
git -C "$wt" reset --hard "$snapshot"
```

After `unset`, the invoking index is read back as `git --git-dir="$git_dir" write-tree`. `$git_dir` was captured with `git rev-parse --path-format=absolute --git-dir` before `unset` (main tree: `.../i2/.git`; linked tree: `.../i2/.git/worktrees/<name>`).

## Lock

`flock` is not on macOS. The lock is a directory:

```bash
# from scripts/precommit-in-worktree.sh
lockdir="$common/precommit-worktree.lock"

acquire_lock() {
  while true; do
    if mkdir "$lockdir" 2>/dev/null; then
      echo "$$" >"$lockdir/pid"
      return
    fi
    local oldpid
    oldpid=$(cat "$lockdir/pid" 2>/dev/null || true)
    if [[ -n "${oldpid}" ]] && ! kill -0 "$oldpid" 2>/dev/null; then
      rm -rf "$lockdir"
      continue
    fi
    sleep 0.2
  done
}

acquire_lock
trap 'rm -rf "$lockdir"' EXIT
```

Concurrent `git commit` from any worktree of this repo queues on that mkdir. One dedicated worktree, one lock.

## Checks in the dedicated worktree

Working tree equals `HEAD` equals the snapshot. `pre-commit` therefore has nothing to hide.

```bash
# from scripts/precommit-in-worktree.sh
if [[ -f "$wt/pnpm-lock.yaml" ]]; then
  (cd "$wt" && pnpm install --frozen-lockfile)
fi

set +e
(cd "$wt" && pre-commit run --from-ref "$parent" --to-ref HEAD --hook-stage pre-commit)
status=$?
set -e
if [[ "$status" -ne 0 ]]; then
  if ! git -C "$wt" diff --quiet; then
    echo "Hooks modified the snapshot. Those fixes were not copied back." >&2
    git -C "$wt" --no-pager diff >&2
  fi
  exit "$status"
fi
```

`--from-ref "$parent" --to-ref HEAD` is the files this commit adds, which is what file-filtered hooks already receive. `pass_filenames: false` hooks (`cargo fmt`, `cargo clippy`, `postfix-constructors`, `prettier`, `oxlint`, `cargo test`) still run when their `types` / `files` match that list. `.pre-commit-config.yaml` is unchanged.

`CARGO_TARGET_DIR` is not set. Cargo writes `$wt/target`, not the invoking tree's `target/`.

## Index after the checks

```bash
# from scripts/precommit-in-worktree.sh
tree_now=$(git --git-dir="$git_dir" write-tree)
if [[ "$tree_now" != "$tree" ]]; then
  echo "index changed while pre-commit ran; aborting commit" >&2
  exit 1
fi
```

Exit 0. Git then creates the real commit from the invoking index. That tree matches `$snapshot`. The snapshot commit stays detached and is dropped from reachability on the next reset.

## Hook install

Tracked hook, relative `core.hooksPath` resolved against the invoking worktree root (githooks(5)).

```bash
# from githooks/pre-commit
#!/usr/bin/env bash
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
exec "$here/../scripts/precommit-in-worktree.sh" "$@"
```

```
# local, not a tracked file
git config core.hooksPath githooks
```

`pre-commit install` writes `.git/hooks`, which is unused while `core.hooksPath` is set.

## Change 1: `scripts/precommit-in-worktree.sh`

New file, executable. Full script:

```bash
# from scripts/precommit-in-worktree.sh
#!/usr/bin/env bash
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "precommit-in-worktree: not a git worktree" >&2
  exit 1
fi

common=$(git rev-parse --path-format=absolute --git-common-dir)
git_dir=$(git rev-parse --path-format=absolute --git-dir)
toplevel=$(git rev-parse --show-toplevel)
wt="$(cd "$common/../.." && pwd)/i2-precommit"

if [[ "$toplevel" == "$wt" ]]; then
  exit 0
fi

if [[ -n "$(git ls-files --unmerged)" ]]; then
  echo "Unmerged files. Resolve before committing." >&2
  exit 1
fi

intent=$(git ls-files -v | grep '^[[:lower:]]' || true)
if [[ -n "$intent" ]]; then
  echo "Intent-to-add files in the index. git add them for real, or git reset them, before committing." >&2
  echo "$intent" >&2
  exit 1
fi

lockdir="$common/precommit-worktree.lock"

acquire_lock() {
  while true; do
    if mkdir "$lockdir" 2>/dev/null; then
      echo "$$" >"$lockdir/pid"
      return
    fi
    local oldpid
    oldpid=$(cat "$lockdir/pid" 2>/dev/null || true)
    if [[ -n "${oldpid}" ]] && ! kill -0 "$oldpid" 2>/dev/null; then
      rm -rf "$lockdir"
      continue
    fi
    sleep 0.2
  done
}

acquire_lock
trap 'rm -rf "$lockdir"' EXIT

local_env_vars=$(git rev-parse --local-env-vars)
tree=$(git write-tree)
parent=$(git rev-parse HEAD)
parents=(-p "$parent")
if [[ -f "$git_dir/MERGE_HEAD" ]]; then
  while read -r p; do
    parents+=(-p "$p")
  done <"$git_dir/MERGE_HEAD"
fi
snapshot=$(git commit-tree "$tree" "${parents[@]}" -m "precommit snapshot")

unset $local_env_vars

if [[ -e "$wt/.git" ]]; then
  git -C "$wt" reset --hard "$snapshot"
  git -C "$wt" clean -fd
elif git --git-dir="$common" worktree list --porcelain | grep -F -x "worktree $wt" >/dev/null; then
  git --git-dir="$common" worktree prune
  git --git-dir="$common" worktree add --detach --lock --reason "pre-commit checks" "$wt" "$snapshot"
elif [[ -e "$wt" ]]; then
  echo "precommit-in-worktree: $wt exists and is not this repo's precommit worktree" >&2
  exit 1
else
  git --git-dir="$common" worktree add --detach --lock --reason "pre-commit checks" "$wt" "$snapshot"
fi

if [[ -f "$wt/pnpm-lock.yaml" ]]; then
  (cd "$wt" && pnpm install --frozen-lockfile)
fi

set +e
(cd "$wt" && pre-commit run --from-ref "$parent" --to-ref HEAD --hook-stage pre-commit)
status=$?
set -e
if [[ "$status" -ne 0 ]]; then
  if ! git -C "$wt" diff --quiet; then
    echo "Hooks modified the snapshot. Those fixes were not copied back." >&2
    git -C "$wt" --no-pager diff >&2
  fi
  exit "$status"
fi

tree_now=$(git --git-dir="$git_dir" write-tree)
if [[ "$tree_now" != "$tree" ]]; then
  echo "index changed while pre-commit ran; aborting commit" >&2
  exit 1
fi
```

Who calls it: `githooks/pre-commit`, and the test script below. Exit 0 allows the invoking `git commit` to proceed. Exit 1 aborts it. No other callers.

## Change 2: `githooks/pre-commit`

New file, executable.

```bash
# from githooks/pre-commit
#!/usr/bin/env bash
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
exec "$here/../scripts/precommit-in-worktree.sh" "$@"
```

Before: `.git/hooks/pre-commit` is the file generated by `pre-commit install`, which runs `python -mpre_commit hook-impl` in the invoking worktree (and hides unstaged files by writing a patch then `git checkout -- .`).

After: git runs `githooks/pre-commit` because `core.hooksPath` is `githooks`.

## Change 3: `scripts/precommit-in-worktree-test.sh`

New file, executable. Creates a throwaway git repo, does not use this clone. Asserts facts about the invoking tree. `pre-commit` on `PATH` is a mock.

Degenerate cases the test establishes:

- staged file plus a dirty unstaged file: after a successful mock, the unstaged contents are the pre-hook contents, `git stash list` is empty, the index still has the staged blob
- mock exits 1: the script exits 1, same two invariants
- mock exits 0 then the test `git add`s another file in the invoking tree before the script's index check would run: covered by invoking `git --git-dir` write-tree mismatch through a mock that writes the invoking index (`GIT_INDEX_FILE` passed to the mock). The script exits 1, message `index changed while pre-commit ran; aborting commit`
- unmerged path: script exits 1 without creating `$wt`
- `git add -N` file: script exits 1 without creating `$wt`
- `$wt` already a directory that is not a gitdir: script exits 1, directory left as it was

The mock is a shell script on `PATH` named `pre-commit` that records its cwd (must be the dedicated worktree) and `--from-ref` / `--to-ref`, then exits as the test asks. `pnpm` is also mocked (`exit 0`) so the test does not install anything. The throwaway repo has a `pnpm-lock.yaml` so that branch is taken, and one staged file plus one unstaged file.

Run by a new local hook so every commit exercises the invariants. The hook must not recurse into this repo's dedicated worktree for the throwaway repo: the script derives `$wt` from that repo's common dir, so the test's worktree is a sibling of the temp clone, not `/Users/rbalicki/code/i2-precommit`.

```yaml
# from .pre-commit-config.yaml (add)
      - id: precommit-in-worktree-test
        name: precommit-in-worktree-test
        entry: ./scripts/precommit-in-worktree-test.sh
        language: system
        pass_filenames: false
```

Place it last in the `local` repo list. `fail_fast: true` still applies.

## Change 4: AGENTS.md Commits

Before:

```
# from AGENTS.md
Commit after every change, small and atomically, without being asked. Each logical change is its own commit.
```

After:

```
# from AGENTS.md
Commit after every change, small and atomically, without being asked. Each logical change is its own commit.

`git commit` runs `.pre-commit-config.yaml` in the dedicated worktree `../i2-precommit`. The invoking working tree, index, and stash list are not modified. `git commit --no-verify` skips the hooks. This clone needs `git config core.hooksPath githooks`.
```

## Change 5: this clone's `core.hooksPath`

```
git config core.hooksPath githooks
```

Not a tracked file. After this, `.git/hooks/pre-commit` is unused. Leave it. A later `pre-commit install` cannot put the stash path back while `core.hooksPath` is set.
