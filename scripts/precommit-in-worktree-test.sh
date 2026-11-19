#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
under_test="$script_dir/precommit-in-worktree.sh"
unset $(git rev-parse --local-env-vars)

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
write_precommit_mock "$bindir" "echo patched > file.txt
exit 1"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test") >"$root/fixer/out" 2>&1
status=$?
set -e
assert_eq "$status" "1" "fixer exit"
assert_contains "$(cat "$root/fixer/out")" "Hooks modified the snapshot. Those fixes were not copied back." "fixer message"
assert_eq "$(cat "$repo/file.txt")" "base" "invoking tree file.txt"

# index changed while hooks ran
repo="$root/idx/repo"
bindir="$root/idx/bin"
init_with_head "$repo"
stage_and_dirty "$repo"
echo extra > "$repo/extra.txt"
make_mocks "$bindir"
idx=$(git -C "$repo" rev-parse --path-format=absolute --git-path index)
gdir=$(git -C "$repo" rev-parse --path-format=absolute --git-dir)
write_precommit_mock "$bindir" "GIT_INDEX_FILE='$idx' git --git-dir='$gdir' --work-tree='$repo' add extra.txt
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

# ignored residue of a crate absent from the snapshot is removed; warm target/ of a crate that exists is kept
repo="$root/stale/repo"
bindir="$root/stale/bin"
init_with_head "$repo"
printf '%s\n' '**/target/**' '**/node_modules/**' > "$repo/.gitignore"
mkdir -p "$repo/crates/kept" "$repo/crates/notes" "$repo/libs/kept"
echo '[package]' > "$repo/crates/kept/Cargo.toml"
echo notes > "$repo/crates/notes/README"
echo '{}' > "$repo/libs/kept/package.json"
git -C "$repo" add .gitignore crates libs
git -C "$repo" commit -q -m crates
make_mocks "$bindir"
write_precommit_mock "$bindir" "exit 0"
stage_and_dirty "$repo"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "stale setup exit"
wt=$(expected_wt "$repo")
mkdir -p "$wt/crates/kept/target" "$wt/crates/gone/target" "$wt/crates/notes/target" \
  "$wt/libs/kept/node_modules" "$wt/libs/gone/node_modules"
echo warm > "$wt/crates/kept/target/artifact"
echo stale > "$wt/crates/gone/target/artifact"
echo stale > "$wt/crates/notes/target/artifact"
echo warm > "$wt/libs/kept/node_modules/artifact"
echo stale > "$wt/libs/gone/node_modules/artifact"
set +e
(cd "$repo" && PATH="$bindir:$PATH" "$under_test")
status=$?
set -e
assert_eq "$status" "0" "stale exit"
assert_eq "$(cat "$wt/crates/kept/target/artifact")" "warm" "kept crate target"
assert_eq "$(cat "$wt/libs/kept/node_modules/artifact")" "warm" "kept lib node_modules"
assert_eq "$(cat "$wt/crates/notes/README")" "notes" "tracked file without a manifest"
if [[ -e "$wt/crates/gone" ]]; then
  fail "stale crate directory $wt/crates/gone remains"
fi
if [[ -e "$wt/libs/gone" ]]; then
  fail "stale lib directory $wt/libs/gone remains"
fi
if [[ -e "$wt/crates/notes/target" ]]; then
  fail "ignored target of a directory without a manifest remains"
fi
