# DiskChanged Absent of a missing path, and path keys as given

Requires filesystem-events.md (landed). `handle_disk_changed` interns and removes `DiskFile`. `MutView::tracked` always increments the map counter. `Absent` of a path that is not in the map still calls `tracked`, so a later memo that walked the map re-executes. `handle` does not canonicalize `path`; that is untested.

One shippable change.

## What the user does

No user-facing change. `isograph send` of `DiskChanged` is the same.

## Change 1: untracked lookup on Absent, tests for path keys

Origin of the Absent arm: landed `crates/isograph_cli/src/state.rs` `handle_disk_changed`. Delta: untracked `get` of the `SourceId`; `tracked` `remove` plus `db.remove` only when the key is present.

`MutView` has no untracked mutate. `Entry` on `tracked()` increments even for a vacant key. The read is untracked so a miss does not increment.

```rust
// from crates/isograph_cli/src/state.rs (before)
            Presence::Absent => {
                if let Some(source_id) = self
                    .get_disk_file_map_mut()
                    .tracked()
                    .0
                    .remove(&change.path)
                {
                    self.remove(source_id);
                }
            }
```

```rust
// from crates/isograph_cli/src/state.rs (after)
            Presence::Absent => {
                if let Some(&source_id) = self
                    .get_disk_file_map()
                    .untracked()
                    .0
                    .get(change.path.reference())
                {
                    self.get_disk_file_map_mut()
                        .tracked()
                        .0
                        .remove(change.path.reference());
                    self.remove(source_id);
                }
            }
```

`handle` is the only writer, so the untracked `get` and the `remove` see the same map. `HashMap::remove` return is unused: the `SourceId` came from `get`.

`extract-iso-literals-from-file.md` copies this arm as a free function on `state`. Same delta there (`self` becomes `state`).

### Tests

`crates/isograph_cli/src/state.rs` tests module. `disk_file` stays. Delete `present_of_an_empty_string_is_present_not_absent`; `an_empty_string_is_stored` already asserts `Some` and `contents == ""`.

```rust
// from crates/isograph_cli/src/state.rs (tests)
use std::sync::atomic::{AtomicUsize, Ordering};

use pico_macros::memo;

static DISK_FILE_COUNT_RUNS: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn disk_file_count(db: &IsographState) -> usize {
    DISK_FILE_COUNT_RUNS.fetch_add(1, Ordering::SeqCst);
    db.get_disk_file_map().tracked().0.len()
}
```

Origin of the memo: pico `crates/pico/tests/tracking_field/efficiency.rs`. Delta: `IsographState` / `disk_file_map`; the run counter is reset at the start of the one test that reads it.

```rust
// from crates/isograph_cli/src/state.rs (tests)
    #[test]
    fn a_relative_path_is_stored_as_given() {
        let mut state = IsographState::default();
        let path = PathBuf::from("src/a.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present("export const a = 1;\n".to_owned()),
        }));
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
        assert!(disk_file(state.reference(), Path::new("/src/a.ts")).is_none());
    }

    #[test]
    fn a_path_and_dot_slash_of_it_are_two_entries() {
        let mut state = IsographState::default();
        let a = PathBuf::from("a.ts");
        let dotted = PathBuf::from("./a.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: a.clone(),
            presence: Presence::Present("a".to_owned()),
        }));
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: dotted.clone(),
            presence: Presence::Present("dotted".to_owned()),
        }));
        assert_eq!(
            disk_file(state.reference(), a.reference())
                .expect("the test inserted this path")
                .contents,
            "a"
        );
        assert_eq!(
            disk_file(state.reference(), dotted.reference())
                .expect("the test inserted this path")
                .contents,
            "dotted"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 2);
    }

    #[test]
    fn absent_of_a_missing_path_does_not_reexecute_a_tracked_map_walk() {
        DISK_FILE_COUNT_RUNS.store(0, Ordering::SeqCst);
        let mut state = IsographState::default();
        let a = PathBuf::from("/tmp/proj/src/a.ts");
        let b = PathBuf::from("/tmp/proj/src/b.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: a.clone(),
            presence: Presence::Present("a".to_owned()),
        }));
        assert_eq!(*disk_file_count(state.reference()), 1);
        assert_eq!(DISK_FILE_COUNT_RUNS.load(Ordering::SeqCst), 1);
        let effects = state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: b,
            presence: Presence::Absent,
        }));
        assert_eq!(effects, Vec::new());
        assert_eq!(*disk_file_count(state.reference()), 1);
        assert_eq!(DISK_FILE_COUNT_RUNS.load(Ordering::SeqCst), 1);
        assert_eq!(
            disk_file(state.reference(), a.reference())
                .expect("the test inserted this path")
                .contents,
            "a"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 1);
    }
```

`absent_of_a_never_present_path_is_a_noop` stays: empty map, `file_count` is not involved, `handle` returns `Vec::new()`.

`expect` names the fixture the test inserted.

## Call sites

- `IsographEvent::DiskChanged` / `Presence::Absent` -> `handle_disk_changed`.
- `extract-iso-literals-from-file.md` copies the same arm.
