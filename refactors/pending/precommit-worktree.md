# Pre-commit in a dedicated worktree

`git commit` in any worktree of this repo snapshots the index as a commit object, checks that commit out in a dedicated worktree, and runs the existing pre-commit hooks there. The invoking working tree, index, `HEAD`, and stash reflog are not modified. If the hooks pass and the invoking index still has the snapshot tree, git creates the commit in the invoking worktree.

## What the user does

```
$ git add crates/isograph_parser/src/chunk.rs
$ git commit -m "chunk: drop empty contents"
```

The invoking worktree stays as it was after `git add`. Other files left dirty stay dirty. `git stash list` is unchanged. Hook output is the same `pre-commit` output as today. On success git creates the commit from the invoking index. On failure git does not create the commit; the invoking worktree and index are still unchanged.

`git commit path/to/file` and `git commit -o` use git's temporary index (`GIT_INDEX_FILE`). The snapshot and the post-check both use that index, so the hook tree is the tree git will commit.

If a hook rewrites files in the dedicated worktree (`end-of-file-fixer`, `trailing-whitespace`, `mixed-line-ending`), those rewrites stay there. The invoking worktree does not receive them. The hook prints that diff and exits 1. Fix the files in the invoking worktree, `git add`, commit again.

`git add -N` paths: the hook exits 1 and prints them. Add them for real or reset them, then commit.

An unborn branch (first commit): the snapshot has no parent. `pre-commit` runs `--all-files`.

`git commit` from the dedicated worktree itself exits 1.

`git commit --no-verify` skips this, as today.

The first run after the dedicated worktree is created compiles from a cold `target/`. Later runs reuse that `target/` and `node_modules/`. This repo has `pnpm-lock.yaml`, so every commit runs `pnpm install --frozen-lockfile` in the dedicated worktree.

## Snapshot

The snapshot is the index `GIT_INDEX_FILE` names, which is what `git commit` would write. Plumbing, no checkout, no stash. Capture the index path before dropping git's hook env. Unborn `HEAD` omits `-p`. `--no-gpg-sign` so a configured `commit.gpgSign` cannot prompt for a dangling snapshot.

```bash
# from scripts/precommit-in-worktree.sh
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
snapshot=$(git commit-tree --no-gpg-sign "$tree" "${parents[@]}" -m "precommit snapshot")
```

`--files .` is present so pre-commit does not enter `staged_files_only` (`stash = not args.all_files and not args.files`). The file list still comes from `--from-ref` / `--to-ref` when both are set. On an unborn branch `--all-files` is the same: no `staged_files_only`.

`git write-tree` fails on unmerged entries. The script checks `git ls-files --unmerged` first and exits 1 with `Unmerged files. Resolve before committing.`

Intent-to-add (`git add -N`) is in the index as an empty blob with `CE_INTENT_TO_ADD`. `git write-tree` includes that empty blob; `git commit` omits the path. The snapshot tree would not match the commit tree. Detect the same way pre-commit does (`git diff` vs the index, added paths), before `write-tree`, and exit 1 with those paths. `git ls-files -v` lowercase is assume-unchanged, not intent-to-add; that is not a rejection.

```bash
# from scripts/precommit-in-worktree.sh
intent=$(git diff --no-ext-diff --ignore-submodules --diff-filter=A --name-only || true)
if [[ -n "$intent" ]]; then
  echo "Intent-to-add files in the index. git add them for real, or git reset them, before committing." >&2
  echo "$intent" >&2
  exit 1
fi
```

The snapshot commit is not on a branch. The dedicated worktree's detached `HEAD` keeps it reachable until the next reset.

## Dedicated worktree

Path: sibling of the clone directory, derived from the common git dir, not from `PWD`, not a hardcoded `i2-precommit` under the parent of the clone.

```bash
# from scripts/precommit-in-worktree.sh
common=$(cd "$(git rev-parse --path-format=absolute --git-common-dir)" && pwd -P)
wt="$(cd "$common/.." && pwd -P)-precommit"
```

For `/Users/rbalicki/code/i2/.git` that is `/Users/rbalicki/code/i2-precommit`. A second clone at `/Users/rbalicki/code/i2-copy` gets `/Users/rbalicki/code/i2-copy-precommit`. Linked worktrees (`i2-mise`, a grok isolated worktree) share `git-common-dir`, so they share this path.

Not `i2-mise`. That worktree is the `i2-mise` branch. Not a new worktree per commit: `target/` and `node_modules/` stay in this one directory so clippy and tests stay incremental.

Create once, locked so `git worktree prune` will not drop it. If `$wt/.git` exists, compare common-dir to `$common` before `reset --hard`; a foreign repo at that path is an error, not a reset. If the worktree is registered and the directory is gone, `worktree remove --force` (locked worktrees are not pruned) then add. Paths are `pwd -P` so they match `worktree list --porcelain`.

```bash
# from scripts/precommit-in-worktree.sh
git --git-dir="$common" worktree add --detach --lock --reason "pre-commit checks" "$wt" "$snapshot"
```

On later runs, when `$wt` is this repo's worktree:

```bash
# from scripts/precommit-in-worktree.sh
git -C "$wt" reset --hard "$snapshot"
git -C "$wt" clean -fd
```

`git clean -fd` without `-x` leaves gitignored `target/` and `node_modules/`.

If `$wt` exists and is not this repo's worktree, exit 1 and name the path. Do not reuse it.

If the hook's worktree is `$wt`, exit 1. `pre-commit run` does not invoke `githooks/pre-commit`, so this is not a recursion guard. It stops `git commit` from the dedicated checkout, and it stops a test that forgets to `cd` into its throwaway repo from exiting 0.

## Git env the hook is given

Git sets `GIT_INDEX_FILE` for every `git commit`. In a linked worktree it also sets `GIT_DIR` to `.git/worktrees/<name>`. `GIT_WORK_TREE` is not set. Snapshot commands use that env. Commands against `$wt` do not.

`git commit path` / `git commit -o` point `GIT_INDEX_FILE` at a temporary `next-index-*.lock`. `git rev-parse --git-path index` follows that. After `unset`, the post-check must pass that saved path; `git --git-dir="$git_dir" write-tree` uses the real index and, during a pathspec commit, fails because git already holds `.git/index.lock`.

```bash
# from scripts/precommit-in-worktree.sh
local_env_vars=$(git rev-parse --local-env-vars)
index_file=$(git rev-parse --path-format=absolute --git-path index)
tree=$(git write-tree)
# ... commit-tree ...
unset $local_env_vars
git -C "$wt" reset --hard "$snapshot"
```

After `unset`, the invoking index is read back as `GIT_INDEX_FILE="$index_file" git --git-dir="$git_dir" write-tree`. `$git_dir` was captured with `git rev-parse --path-format=absolute --git-dir` before `unset` (main tree: `.../i2/.git`; linked tree: `.../i2/.git/worktrees/<name>`).

## Lock

macOS `/bin/bash` is 3.2: no `coproc`, no `flock(1)`. `python3` `fcntl.flock` blocks in the kernel. Waiters park there. The script re-execs itself under python once python holds the lock; the lock fd is inherited (`os.set_inheritable`) so it stays open until the hook process exits.

```bash
# from scripts/precommit-in-worktree.sh
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
```

Concurrent `git commit` from any worktree of this repo queues on that flock. One dedicated worktree, one lock file.

## Checks in the dedicated worktree

Working tree equals `HEAD` equals the snapshot. `pre-commit` therefore has nothing to hide. `--files .` / `--all-files` keep it out of `staged_files_only` even if a smudge filter dirties `$wt`.

`PATH` appends Homebrew and mise shims so `pre-commit` and `pnpm` resolve from a git hook env that is only `/usr/bin:/bin:...`, without shadowing a mock the tests put first.

```bash
# from scripts/precommit-in-worktree.sh
PATH="${PATH}:/opt/homebrew/bin:${HOME}/.local/share/mise/shims"

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
```

`--from-ref "$parent" --to-ref HEAD` is the files this commit adds, which is what file-filtered hooks already receive. `pass_filenames: false` hooks (`cargo fmt`, `cargo clippy`, `postfix-constructors`, `prettier`, `oxlint`, `cargo test`) still run when their `types` / `files` match that list. `.pre-commit-config.yaml` is unchanged except for the test hook in Change 3.

`CARGO_TARGET_DIR` is not set. Cargo writes `$wt/target`, not the invoking tree's `target/`.

## Index after the checks

```bash
# from scripts/precommit-in-worktree.sh
tree_now=$(GIT_INDEX_FILE="$index_file" git --git-dir="$git_dir" write-tree)
if [[ "$tree_now" != "$tree" ]]; then
  echo "index changed while pre-commit ran; aborting commit" >&2
  exit 1
fi
```

Exit 0. Git then creates the real commit from the invoking index. That tree matches `$snapshot`. The snapshot commit stays detached and is dropped from reachability on the next reset.

## Hook install

Tracked hook. `core.hooksPath` is an absolute path to `githooks` in the checkout that contains `githooks/pre-commit` (the `i2` worktree). Relative `githooks` is resolved against the invoking worktree root (githooks(5)), so `i2-mise` and a grok worktree on a branch without that directory would skip the hook. Absolute path is local config, shared by every worktree of this clone.

```bash
# from githooks/pre-commit
#!/usr/bin/env bash
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
exec "$here/../scripts/precommit-in-worktree.sh" "$@"
```

```
# local, not a tracked file. run from /Users/rbalicki/code/i2
git config core.hooksPath "$(git rev-parse --show-toplevel)/githooks"
```

That stores `/Users/rbalicki/code/i2/githooks`. Commits from `i2-mise` and from a grok isolated worktree run that file. The wrapper then snapshots the invoking worktree's index.

`pre-commit install` writes `.git/hooks`, which is unused while `core.hooksPath` is set. Leave `.git/hooks/pre-commit` in place. A clone that has not set `core.hooksPath` still runs that generated hook (`python -mpre_commit hook-impl` in the invoking worktree, `git checkout -- .` to hide unstaged files).

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
snapshot=$(git commit-tree --no-gpg-sign "$tree" "${parents[@]}" -m "precommit snapshot")

unset $local_env_vars

if [[ -e "$wt/.git" ]]; then
  got=$(cd "$(git -C "$wt" rev-parse --path-format=absolute --git-common-dir)" && pwd -P)
  if [[ "$got" != "$common" ]]; then
    echo "precommit-in-worktree: $wt exists and is not this repo's precommit worktree" >&2
    exit 1
  fi
  git -C "$wt" reset --hard "$snapshot"
  git -C "$wt" clean -fd
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

After: git runs `/Users/rbalicki/code/i2/githooks/pre-commit` because `core.hooksPath` is that absolute directory.

## Change 3: `scripts/precommit-in-worktree-test.sh`

New file, executable. Creates throwaway git repos under `mktemp -d`, does not use this clone. Asserts facts about the invoking tree. `pre-commit` and `pnpm` on `PATH` are mocks in a bindir the test prepends; the script appends Homebrew and mise shims, so the mocks stay first.

The test `cd`s into each throwaway repo before invoking the script. `$wt` is derived from that repo's common dir: if the repo is `$root/ok/repo`, `$wt` is `$root/ok/repo-precommit`. That is not `/Users/rbalicki/code/i2-precommit`. `trap` `rm -rf "$root"` removes both.

Each case that expects the script to create `$wt` starts with an initial commit unless the case is unborn `HEAD`.

Degenerate cases the test establishes:

- staged file plus a dirty unstaged file: after a successful mock, the unstaged contents are the pre-hook contents, `git stash list` is empty, the index still has the staged blob; mock cwd is `$wt`; mock args contain `--from-ref`, `--to-ref HEAD`, `--files .`
- mock exits 1: the script exits 1, same three invoking-tree invariants
- mock writes a file in its cwd and exits 1: script stderr contains `Hooks modified the snapshot. Those fixes were not copied back.`; the invoking tree does not have that file
- mock `git add`s another file in the invoking index (path captured when writing the mock, not `GIT_INDEX_FILE` from the environment, which the script unsets before running `pre-commit`): the script exits 1, message `index changed while pre-commit ran; aborting commit`
- `GIT_INDEX_FILE` pointing at a temp index whose tree is not the default index: the script exits 0 (post-check uses the temp index)
- unmerged path: script exits 1 without creating `$wt`
- `git add -N` file: script exits 1 without creating `$wt`, stderr names the path
- `git update-index --assume-unchanged` on a tracked file plus a real staged add: script exits 0 (`ls-files -v` lowercase is not treated as intent-to-add)
- unborn `HEAD`: script exits 0; mock args contain `--all-files` and do not contain `--from-ref`
- `$wt` already a directory that is not a gitdir: script exits 1, directory left as it was (marker file unchanged)
- `$wt` already a different git repo: script exits 1, that repo's file is unchanged
- after a successful run, invoke the script with cwd `$wt`: exit 1, stderr contains `refusing to commit from the dedicated precommit worktree`

```bash
# from scripts/precommit-in-worktree-test.sh
#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
under_test="$script_dir/precommit-in-worktree.sh"

fail() {
  echo "precommit-in-worktree-test: $*" >&2
  exit 1
}

assert_eq() {
  if [[ "$1" != "$2" ]]; then
    fail "$3: got $(printf %q "$1") want $(printf %q "$2")"
  fi
}

assert_contains() {
  case "$1" in
    *"$2"*) ;;
    *) fail "$3: $(printf %q "$1") does not contain $(printf %q "$2")" ;;
  esac
}

expected_wt() {
  local common
  common=$(cd "$(git -C "$1" rev-parse --path-format=absolute --git-common-dir)" && pwd -P)
  echo "$(cd "$common/.." && pwd -P)-precommit"
}

make_mocks() {
  local bindir="$1"
  mkdir -p "$bindir"
  cat > "$bindir/pnpm" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$bindir/pnpm"
}

write_precommit_mock() {
  local bindir="$1"
  local body="$2"
  cat > "$bindir/pre-commit" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$PWD" > "$bindir/cwd"
printf '%s\n' "\$*" > "$bindir/args"
$body
EOF
  chmod +x "$bindir/pre-commit"
}

git_ident() {
  git -C "$1" config user.email t@t
  git -C "$1" config user.name t
}

init_with_head() {
  local repo="$1"
  mkdir -p "$repo"
  git -C "$repo" init -q
  git_ident "$repo"
  echo base > "$repo/file.txt"
  echo lock > "$repo/pnpm-lock.yaml"
  git -C "$repo" add file.txt pnpm-lock.yaml
  git -C "$repo" commit -q -m init
}

stage_and_dirty() {
  local repo="$1"
  echo staged > "$repo/staged.txt"
  git -C "$repo" add staged.txt
  echo dirty > "$repo/staged.txt"
}

invoking_invariants() {
  local repo="$1"
  assert_eq "$(cat "$repo/staged.txt")" "dirty" "unstaged contents"
  assert_eq "$(git -C "$repo" stash list)" "" "stash list"
  assert_eq "$(git -C "$repo" show :staged.txt)" "staged" "index blob"
}

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT

# success: staged plus dirty, mock cwd and args
repo="$root/ok/repo"
bindir="$root/ok/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "success exit"
invoking_invariants "$repo"
wt=$(expected_wt "$repo")
assert_eq "$(cd "$(cat "$bindir/cwd")" && pwd -P)" "$(cd "$wt" && pwd -P)" "mock cwd"
assert_contains "$(cat "$bindir/args")" "--from-ref" "mock args from-ref"
assert_contains "$(cat "$bindir/args")" "--to-ref HEAD" "mock args to-ref"
assert_contains "$(cat "$bindir/args")" "--files ." "mock args files"

# mock exits 1
repo="$root/fail/repo"
bindir="$root/fail/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 1"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/fail/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "mock 1 exit"
invoking_invariants "$repo"

# fixer in dedicated worktree is not copied back
repo="$root/fixer/repo"
bindir="$root/fixer/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
make_mocks "$bindir"
write_precommit_mock "$bindir" "echo patched > patched.txt
exit 1"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/fixer/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "fixer exit"
assert_contains "$(cat "$root/fixer/out")" "Hooks modified the snapshot. Those fixes were not copied back." "fixer message"
if [[ -e "$repo/patched.txt" ]]; then
  fail "patched.txt was copied to the invoking tree"
fi

# index changed while hooks ran
repo="$root/idx/repo"
bindir="$root/idx/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
echo extra > "$repo/extra.txt"
make_mocks "$bindir"
idx=$(git -C "$repo" rev-parse --path-format=absolute --git-path index)
gdir=$(git -C "$repo" rev-parse --path-format=absolute --git-dir)
write_precommit_mock "$bindir" "GIT_INDEX_FILE='$idx' git --git-dir='$gdir' add extra.txt
exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/idx/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "index changed exit"
assert_contains "$(cat "$root/idx/out")" "index changed while pre-commit ran; aborting commit" "index changed message"

# GIT_INDEX_FILE is a temp index that differs from the default index
repo="$root/pathspec/repo"
bindir="$root/pathspec/bin"
init_with_head "$repo"
echo a > "$repo/a.txt"
echo b > "$repo/b.txt"
git -C "$repo" add a.txt b.txt
temp_index="$repo/.git/test-index"
GIT_INDEX_FILE="$temp_index" git -C "$repo" read-tree HEAD
GIT_INDEX_FILE="$temp_index" git -C "$repo" add a.txt
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" GIT_INDEX_FILE="$temp_index" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "temp GIT_INDEX_FILE exit"

# unmerged, no $wt
repo="$root/unmerged/repo"
bindir="$root/unmerged/bin"
init_with_head "$repo"
git -C "$repo" checkout -q -b side
echo side > "$repo/file.txt"
git -C "$repo" add file.txt
git -C "$repo" commit -q -m side
git -C "$repo" checkout -q main 2>/dev/null || git -C "$repo" checkout -q master
echo main > "$repo/file.txt"
git -C "$repo" add file.txt
git -C "$repo" commit -q -m main
git -C "$repo" merge --no-edit side >/dev/null 2>&1 || true
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
wt=$(expected_wt "$repo")
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/unmerged/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "unmerged exit"
if [[ -e "$wt" ]]; then
  fail "unmerged created $wt"
fi
assert_contains "$(cat "$root/unmerged/out")" "Unmerged files. Resolve before committing." "unmerged message"

# intent-to-add, no $wt
repo="$root/intent/repo"
bindir="$root/intent/bin"
init_with_head "$repo"
echo n > "$repo/n.txt"
git -C "$repo" add -N n.txt
echo staged > "$repo/staged.txt"
git -C "$repo" add staged.txt
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
wt=$(expected_wt "$repo")
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/intent/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "intent-to-add exit"
if [[ -e "$wt" ]]; then
  fail "intent-to-add created $wt"
fi
assert_contains "$(cat "$root/intent/out")" "n.txt" "intent-to-add path"

# assume-unchanged is not intent-to-add
repo="$root/assume/repo"
bindir="$root/assume/bin"
init_with_head "$repo"
git -C "$repo" update-index --assume-unchanged file.txt
echo staged > "$repo/staged.txt"
git -C "$repo" add staged.txt
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "assume-unchanged exit"

# unborn HEAD
repo="$root/unborn/repo"
bindir="$root/unborn/bin"
mkdir -p "$repo"
git -C "$repo" init -q
git_ident "$repo"
echo x > "$repo/f.txt"
echo lock > "$repo/pnpm-lock.yaml"
git -C "$repo" add f.txt pnpm-lock.yaml
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "unborn exit"
assert_contains "$(cat "$bindir/args")" "--all-files" "unborn all-files"
case "$(cat "$bindir/args")" in
  *--from-ref*) fail "unborn passed --from-ref" ;;
esac

# $wt exists as a plain directory
repo="$root/plain/repo"
bindir="$root/plain/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
wt=$(expected_wt "$repo")
mkdir -p "$wt"
echo keep > "$wt/marker"
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/plain/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "plain dir exit"
assert_eq "$(cat "$wt/marker")" "keep" "plain dir marker"
assert_contains "$(cat "$root/plain/out")" "exists and is not this repo's precommit worktree" "plain dir message"

# $wt exists as a foreign git repo
repo="$root/foreign/repo"
bindir="$root/foreign/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
wt=$(expected_wt "$repo")
mkdir -p "$wt"
git -C "$wt" init -q
git_ident "$wt"
echo precious > "$wt/file"
git -C "$wt" add file
git -C "$wt" commit -q -m foreign
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/foreign/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "foreign repo exit"
assert_eq "$(cat "$wt/file")" "precious" "foreign repo file"
assert_contains "$(cat "$root/foreign/out")" "exists and is not this repo's precommit worktree" "foreign repo message"

# refuse commit from the dedicated worktree
repo="$root/refuse/repo"
bindir="$root/refuse/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "refuse setup exit"
wt=$(expected_wt "$repo")
set +e
(cd "$wt" && PATH="$bindir:$PATH" "$under_test") >"$root/refuse/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "refuse from wt exit"
assert_contains "$(cat "$root/refuse/out")" "refusing to commit from the dedicated precommit worktree" "refuse message"
```

Run by a new local hook. `fail_fast: true` skips it when an earlier hook fails; it is a regression test of the wrapper, not a check that runs on a red fmt. `always_run: true` so a commit whose file list is empty still runs it.

```yaml
# from .pre-commit-config.yaml (add)
      - id: precommit-in-worktree-test
        name: precommit-in-worktree-test
        entry: ./scripts/precommit-in-worktree-test.sh
        language: system
        pass_filenames: false
        always_run: true
```

Place it last in the `local` repo list.

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

`git commit` runs `.pre-commit-config.yaml` in the dedicated worktree `<clone>-precommit` (for this clone, `/Users/rbalicki/code/i2-precommit`). The invoking working tree, index, and stash list are not modified. `git commit --no-verify` skips the hooks. This clone needs `git config core.hooksPath "$(git rev-parse --show-toplevel)/githooks"` run from the `i2` worktree so the stored path is absolute and every worktree of this clone runs that hook.
```

## Change 5: this clone's `core.hooksPath`

From `/Users/rbalicki/code/i2`:

```
git config core.hooksPath "$(git rev-parse --show-toplevel)/githooks"
```

Stores `/Users/rbalicki/code/i2/githooks`. Not a tracked file. Shared local config, so `i2-mise` and grok isolated worktrees use it. After this, `.git/hooks/pre-commit` is unused. Leave it. A later `pre-commit install` cannot put the stash path back while `core.hooksPath` is set.
