# pico depends on missing keys through absent nodes

Every keyed read of a source or singleton inside a memo records a dependency on that key, whether a source is set there or not. A read of a missing key creates an absent node at that key. Setting the key later is a change to that node, and the reader re-runs. Removing a source turns its node into an absent node, which is also a change. `tracked()` on a `#[tracked]` map is for iteration only.

## How pico works, and where this fits

pico.md has the depth: sections "Source", "Memo", "Equality and backdating", "Tracked maps", and "Garbage collection". This section is the minimum needed to read the design.

A source is input data. `handle` writes it with `db.set`. Its identity is its `#[key]` field. `#[derive(Source)]` hashes the type's `TypeId` plus the key field into a `Key` (`crates/pico_macros/src/source_macro.rs:49-58`). `db.set` returns a `SourceId<T>`, which is that `Key` plus `PhantomData<T>` (`crates/pico/src/source.rs:16-20`, `crates/pico/src/database.rs:157-162`). Pass the `SourceId` to `db.get(id)` and you get `&T` back. A singleton is a source whose key is the hash of its `TypeId` alone, read with `db.get_singleton::<T>()`, which returns `Option<&T>` (`crates/pico_macros/src/singleton_macro.rs:10-23`, `database.rs:130-132`).

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: RelativePathToSourceFile,
    pub contents: String,
}

impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        let source_id = self.set(DiskFile { path, contents });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }
}
```

A memo is a `#[memo]` function of `&db` and parameters. pico caches its result per (function, parameters) as a derived node (`crates/pico/src/execute_memoized_function.rs:58-104`).

A memo records dependencies while it runs. Each `db.get` / `get_singleton` inside it calls `get_impl`, which registers `NodeKind::Source(key)` together with that node's `time_updated` (`database.rs:134-140`). Each nested memo call registers `NodeKind::Derived(id)` (`execute_memoized_function.rs:99-102`). The recorded `Dependency` stores the epoch the read happened in, `time_verified_or_updated: current_epoch` (`database.rs:109-121`).

An epoch is a counter in the database (`crates/pico/src/epoch.rs:5-20`, `database.rs:78`). It advances when `set` replaces a source with an unequal value (`database.rs:340-347`) and when `remove` removes one (`database.rs:362-373`). Every source node carries `time_updated`, the epoch it last changed in (`source.rs:80-84`).

A later call of a memo verifies before it reuses (`execute_memoized_function.rs:71-98`):

```text
already verified in the current epoch          -> reuse                     (database.rs:258-263)
else, for each dependency recorded in an older epoch:
  source:  node.time_updated > since ?           (execute_memoized_function.rs:202-207)
  derived: verify that memo first, then compare  (execute_memoized_function.rs:209-248)
any changed -> re-run the body; if the new value == the old value, keep the old time_updated (backdating, execute_memoized_function.rs:162-174)
none changed -> reuse
```

A `#[tracked]` field (`disk_file_map`, `open_file_map`) is a plain `HashMap<key, SourceId<_>>` on the database plus a generated counter singleton (`DiskFileMapCounter`, `crates/pico_macros/src/db_macro.rs:73-80`). `View::tracked()` reads the counter, which depends on every key of the map (`crates/pico/src/view.rs:29-32`). `MutView::tracked()` writes counter + 1 (`view.rs:67-74`). `untracked()` reads the field with no dependency (`view.rs:42-44`). The i2 rule today: on a hit, `untracked()` lookup then `db.get(source_id)`, which depends on that one source. On a miss, `tracked()`, which depends on the counter. Iteration uses `tracked()`. `extract_iso_literals` is the shape (`crates/isograph_extract_typescript/src/lib.rs:68-71`).

The gap: `get_impl` returns at `get_source_node(key)?` (`database.rs:135`) before it registers anything, so a read of a missing key records nothing. A tracked-on-miss read of a counter that has never been written records nothing either, because the counter is itself a missing key. Concretely in i2: on a database where no `DiskFile` was ever inserted, `lsp_semantic_tokens_for_file(path)` reaches `extract_iso_literals(path)`, which misses in the map and reads `DiskFileMapCounter`, which does not exist. The memo stores `None` with no dependencies. The first `insert_disk_file` is two vacant `set`s (the `DiskFile` and the counter), and neither advances the epoch (`database.rs:352-358`). The next semantic tokens request finds the memo verified in the current epoch and returns the stale `None`. The same happens to `file_contents` (`crates/isograph_compiler/src/database.rs:65-80`) before any client has opened any buffer: the first `set_open_file` is seen by nothing, and the memo keeps returning disk text. After an absent node exists for the missing key (or for the counter), the first `set` of that key is a change from absent to present, the epoch advances, and the reader re-runs.

## Design

### `SourceNode` can be absent

The node storage is `source_node_key_to_index: DashMap<Key, Index<SourceNode>>` plus `source_nodes: BoxcarVec<Option<SourceNode>>` (`database.rs:72,76`). Today `remove_source` drops the key from the map and `take`s the boxcar slot, leaving `None` there (`database.rs:362-373`). After this change nothing removes a key or empties a slot. Absence moves into the node, and the boxcar `Option` goes away.

Before:

```rust
// external isograph/crates/pico/src/source.rs
#[derive(Debug)]
pub struct SourceNode {
    pub time_updated: Epoch,
    pub value: Box<dyn DynEq>,
}
```

After:

```rust
// from crates/pico/src/source.rs
#[derive(Debug)]
pub struct SourceNode {
    pub time_updated: Epoch,
    /// The source set at this key, or `None` when no source is set at it.
    pub value: Option<Box<dyn DynEq>>,
}

impl SourceNode {
    pub(crate) fn present(time_updated: Epoch, value: Box<dyn DynEq>) -> Self {
        Self {
            time_updated,
            value: value.wrap_some(),
        }
    }

    pub(crate) fn absent(time_updated: Epoch) -> Self {
        Self {
            time_updated,
            value: None,
        }
    }
}
```

`source.rs` gains `use prelude::Postfix;` (open decision 2).

`InternalStorage` field, before (`database.rs:76`):

```rust
// external isograph/crates/pico/src/database.rs
    pub(crate) source_nodes: BoxcarVec<Option<SourceNode>>,
```

After:

```rust
// from crates/pico/src/database.rs
    pub(crate) source_nodes: BoxcarVec<SourceNode>,
```

`Storage::new_with_capacity` (`database.rs:96`) is unchanged: `BoxcarVec::new()` infers the new element type.

Before/after pico snippets are labeled `// external isograph/...` because `crates/pico` and `crates/pico_macros` are byte-identical to upstream isograph today (`diff -r` against `../isograph` differs only in `crates/pico/tests/tracking_field/correctness.rs`), and pico's sources use `Some(` and `Box::new`, which the pending-doc postfix lint rejects in `// from` snippets (`crates/prelude/src/postfix_constructors.rs:48-58,134-167`).

### Reads register a dependency on missing keys

Before (`database.rs:134-152`):

```rust
// external isograph/crates/pico/src/database.rs
    fn get_impl<T: 'static>(&self, key: Key) -> Option<&T> {
        let source_node = self.internal.get_source_node(key)?;

        self.register_dependency_in_parent_memoized_fn(
            NodeKind::Source(key),
            source_node.time_updated,
        );
        Some(
            source_node
                .value
                .as_ref()
                .as_any()
                .downcast_ref::<T>()
                .expect(
                    "unexpected struct type. \
            This is indicative of a bug in Pico.",
                ),
        )
    }
```

After:

```rust
// from crates/pico/src/database.rs
    fn get_impl<T: 'static>(&self, key: Key) -> Option<&T> {
        let source_node = if self.dependency_stack.is_empty() {
            self.internal.get_source_node(key)?
        } else {
            self.internal.get_or_insert_absent_source_node(key)
        };

        self.register_dependency_in_parent_memoized_fn(
            NodeKind::Source(key),
            source_node.time_updated,
        );
        source_node.value.as_ref().map(|value| {
            value.as_ref().as_any().downcast_ref::<T>().expect(
                "unexpected struct type. \
                This is indicative of a bug in Pico.",
            )
        })
    }
```

Callers are unchanged: `Storage::get` (`database.rs:123-128`) and `Storage::get_singleton` (`database.rs:130-132`). `Storage::get` on a `SourceId` whose node is absent still panics with its existing message, because `get_impl` returns `None` and `get` calls `expect` on it. A `SourceId` of a removed source is still a false proof (pico.md "Source"). `removing_params.rs` keeps passing: after the remove, `first_letter`'s dependency on the key is an absent node with a newer `time_updated`, the memo re-runs, and `db.get` panics.

`InternalStorage` lookup functions, before (`database.rs:308-321`):

```rust
// external isograph/crates/pico/src/database.rs
    pub fn get_source_node(&self, key: Key) -> Option<&SourceNode> {
        let index = self.source_node_key_to_index.get(&key)?;
        self.source_nodes
            .get(index.idx)
            .expect(
                "indexes should always be valid. \
                This is indicative of a bug in Pico.",
            )
            .as_ref()
    }

    pub(crate) fn insert_source_node(&self, source_node: SourceNode) -> Index<SourceNode> {
        Index::new(self.source_nodes.push(Some(source_node)))
    }
```

After:

```rust
// from crates/pico/src/database.rs
    pub fn get_source_node(&self, key: Key) -> Option<&SourceNode> {
        let index = self.source_node_key_to_index.get(&key)?;
        self.source_node_at(index.idx).wrap_some()
    }

    /// The node at `key`, after inserting an absent node stamped with the current epoch if `key` has none.
    pub(crate) fn get_or_insert_absent_source_node(&self, key: Key) -> &SourceNode {
        let idx = self
            .source_node_key_to_index
            .entry(key)
            .or_insert_with(|| self.insert_source_node(SourceNode::absent(self.current_epoch)))
            .idx;
        self.source_node_at(idx)
    }

    fn source_node_at(&self, idx: usize) -> &SourceNode {
        self.source_nodes.get(idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Pico.",
        )
    }

    pub(crate) fn insert_source_node(&self, source_node: SourceNode) -> Index<SourceNode> {
        Index::new(self.source_nodes.push(source_node))
    }
```

`.idx` is read out of the `RefMut` because `Index<SourceNode>` is not `Copy`: `#[derive(Clone, Copy)]` on `Index<T>` (`crates/pico/src/index.rs:3`) bounds `T: Copy`. The `RefMut` drops at the end of the `let`, before `source_node_at`. The closure touches only `source_nodes` and `current_epoch`, not `source_node_key_to_index`, so it cannot deadlock on the shard lock that `entry` holds.

Why creating the node is not a change for its reader: the reader registers `Dependency { node_to: NodeKind::Source(key), time_verified_or_updated: current_epoch }` (`database.rs:114-120`). The new node's `time_updated` is that same `current_epoch`. Verification asks `time_updated > since` (`execute_memoized_function.rs:204`), which is false. Creation does not advance the epoch, so `node_verified_in_current_epoch` (`database.rs:258-263`) of every other memo is unaffected. No other memo depends on a node that did not exist. `intern_value` stamps a node it creates during a read the same way (`database.rs:402-419`).

Interior mutability: creation goes through `&self` with `DashMap::entry` on `source_node_key_to_index` and `boxcar::Vec::push` on `source_nodes`. Both are already written through `&self` during memo execution: `insert_source_node` takes `&self` (`database.rs:319-321`), `intern_value` inserts into `derived_node_id_to_revision` through `&self` (`database.rs:396-419`), and `intern_borrowed_param` pushes to `params` through `&self` (`crates/pico/src/macro_fns.rs:20-27`). No new primitive. `Storage` is `!Sync` because `DependencyStack` is a `RefCell` (`crates/pico/src/dependency.rs:55`), so these are single-thread writes. A `&SourceNode` handed out earlier stays valid across the push, because `boxcar::Vec` does not move elements.

A read outside any memo does not create a node. With an empty dependency stack, `register_dependency_in_parent_memoized_fn` registers nothing (`dependency.rs:81-89`), so nothing could ever compare against the node. A node created there would only turn a later first `set` of that key from a vacant insert into an epoch advance that no memo observes. `relative_from_uri` reads `CurrentWorkingDirectory` this way, from a request handler with no memo on the stack (`crates/isograph_cli/src/adapter.rs:42-53`).

### `set` over an absent node is a change

Before (`database.rs:323-360`):

```rust
// external isograph/crates/pico/src/database.rs
    /// Sets a source in the database. If there is an existing item and it does not equal
    /// the new source, increment the current epoch.
    fn set_source<T: DynEq>(&mut self, source: T, source_id: SourceId<T>) {
        match self.source_node_key_to_index.entry(source_id.key) {
            Entry::Occupied(occupied_entry) => {
                let source_node = self
                    .source_nodes
                    .get_mut(occupied_entry.get().idx)
                    .expect(
                        "indexes should always be valid. \
                        This is indicative of a bug in Pico.",
                    )
                    .as_mut()
                    .expect(
                        "indexes should always point to a non-empty source node. \
                        This is indicative of a bug in Pico.",
                    );
                if !source_node.value.as_ref().dyn_eq(&source) {
                    // We cannot call self.increment_epoch() because that borrows
                    // the entire struct, but self.source_nodes is already borrowed
                    let next_epoch = self.current_epoch.increment();
                    *source_node = SourceNode {
                        time_updated: next_epoch,
                        value: Box::new(source),
                    };
                } else {
                    source_node.time_updated = self.current_epoch;
                }
            }
            Entry::Vacant(vacant_entry) => {
                let index = self.insert_source_node(SourceNode {
                    time_updated: self.current_epoch,
                    value: Box::new(source),
                });
                vacant_entry.insert(index);
            }
        }
    }
```

After:

```rust
// from crates/pico/src/database.rs
    /// Sets a source in the database. If the key has a node that is absent or holds an unequal
    /// source, increment the current epoch.
    fn set_source<T: DynEq>(&mut self, source: T, source_id: SourceId<T>) {
        match self.source_node_key_to_index.entry(source_id.key) {
            Entry::Occupied(occupied_entry) => {
                let source_node = self
                    .source_nodes
                    .get_mut(occupied_entry.get().idx)
                    .expect(
                        "indexes should always be valid. \
                        This is indicative of a bug in Pico.",
                    );
                match source_node.value.as_ref() {
                    Some(value) if value.as_ref().dyn_eq(&source) => {
                        source_node.time_updated = self.current_epoch;
                    }
                    Some(_) | None => {
                        // We cannot call self.increment_epoch() because that borrows
                        // the entire struct, but self.source_nodes is already borrowed
                        let next_epoch = self.current_epoch.increment();
                        *source_node = SourceNode::present(next_epoch, source.boxed());
                    }
                }
            }
            Entry::Vacant(vacant_entry) => {
                let index =
                    self.insert_source_node(SourceNode::present(self.current_epoch, source.boxed()));
                vacant_entry.insert(index);
            }
        }
    }
```

- Absent to present: the `None` arm. The epoch advances, the node's `time_updated` is the new epoch, and every reader's `since` is older, so readers re-run.
- Present and unequal: unchanged behavior.
- Present and equal: unchanged behavior (`source_node.time_updated = self.current_epoch`, no advance). See Follow-ups for what this arm does after an unrelated epoch advance.
- A key that has no node (no memo ever read it while missing, nobody ever set it): the `Vacant` arm, unchanged. Upstream 21d671d7 ("When setting a source in a Database, avoid incrementing the epoch if the source is identical to the previously set source") introduced this `Occupied` / `Vacant` split with a vacant insert at `current_epoch`. Its message states the intent as not advancing the epoch on an identical re-set. The vacant arm not advancing is kept because no memo can depend on a key that has no node, now that every in-memo read of a missing key creates one.
- `set_source` loses the "indexes should always point to a non-empty source node" `expect`: a slot is never empty.

### `remove` turns a present node into an absent node

Before (`database.rs:362-373`):

```rust
// external isograph/crates/pico/src/database.rs
    pub fn remove_source<T>(&mut self, id: SourceId<T>) {
        if let Some((_, index)) = self.source_node_key_to_index.remove(&id.key) {
            self.current_epoch.increment();
            self.source_nodes
                .get_mut(index.idx)
                .expect(
                    "indexes should always be valid. \
                    This is indicative of a bug in Pico.",
                )
                .take();
        }
    }
```

After:

```rust
// from crates/pico/src/database.rs
    pub fn remove_source<T>(&mut self, id: SourceId<T>) {
        let Some(index) = self.source_node_key_to_index.get(&id.key) else {
            return;
        };
        let source_node = self.source_nodes.get_mut(index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Pico.",
        );
        if source_node.value.is_some() {
            *source_node = SourceNode::absent(self.current_epoch.increment());
        }
    }
```

- Present: the node becomes absent, stamped with a new epoch. Readers re-run and see `None` (or, through `db.get`, panic as today).
- Absent already: no change, no epoch advance.
- No node: no change, no epoch advance (today the same).
- The key stays in `source_node_key_to_index`. There is no tombstone side structure; the absent node is the record.

`Storage::remove` and `Storage::remove_singleton` (`database.rs:164-173`) are unchanged.

### Verification compares absent nodes like any node

Before (`execute_memoized_function.rs:202-207`):

```rust
// external isograph/crates/pico/src/execute_memoized_function.rs
fn source_node_changed_since<Db: Database>(db: &Db, key: Key, since: Epoch) -> bool {
    match db.get_storage().internal.get_source_node(key) {
        Some(source) => source.time_updated > since,
        None => true,
    }
}
```

After:

```rust
// from crates/pico/src/execute_memoized_function.rs
fn source_node_changed_since<Db: Database>(db: &Db, key: Key, since: Epoch) -> bool {
    db.get_storage()
        .internal
        .get_source_node(key)
        .expect(
            "a source dependency is recorded only for a key that has a node, and source nodes \
            are never removed. This is indicative of a bug in Pico.",
        )
        .time_updated
        > since
}
```

An absent node that is still absent compares `time_updated > since` like a present one, so a missing key that stays missing across any number of unrelated epochs does not re-run its reader. A key with no node cannot be a recorded dependency: `NodeKind::Source` is registered only by `get_impl` (`grep -rn "NodeKind::Source" crates/pico/src` finds `database.rs:138`, the match at `execute_memoized_function.rs:193`, and `garbage_collection.rs:137`), `get_impl` with a memo on the stack always finds or creates the node, and after this change no code removes a key from `source_node_key_to_index`. The `None` case is that broken invariant; open decision 3 is whether it is an `expect` or stays `true`.

### Garbage collection

Unchanged code. Source nodes are not collected (`crates/pico/src/garbage_collection.rs:21`; `add_dependencies_to_queue` skips `NodeKind::Source`, `garbage_collection.rs:136-137`). An absent node is a source node, so it lives as long as the database. Memory cost: one node per key that a memo ever read while missing, plus one per removed source. A node is one `DashMap` entry (`Key` and `Index`, 16 bytes plus map overhead) and one `SourceNode` (`Epoch` plus `Option<Box<dyn DynEq>>`, 24 bytes) with no heap value. Removed sources already kept their boxcar slot today (`take` leaves `None` in the `BoxcarVec`), so the new cost for them is only the retained map entry. For `disk_file_map` this is one node per path an LSP request or a memo asked about, including paths that never existed. The earlier `TrackedMap` slot design had the same cost (a slot per key ever read or inserted, see Follow-ups). A memo collected by GC loses its dependency list, not the node; the next call re-creates the memo and reads the existing node.

## Memo author experience

A memo reads a singleton or source key and gets `None` when nothing is set. That read is a dependency. Nothing else is required.

```rust
// from crates/pico/tests/absent_source/missing_key.rs
#[memo]
fn config_name(db: &TestDatabase) -> Option<String> {
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}
```

- `config_name(&db)` before any `Config` is set returns `None`.
- `db.set(Config { name: "a".to_string() })`, then `config_name(&db)` re-runs and returns `Some("a")`. Today it returns the stale `None`.
- `db.remove_singleton::<Config>()`, then `config_name(&db)` re-runs and returns `None`.
- Setting or removing any other source does not re-run it.

After change 3, a per-key source lookup has the same shape with no map:

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        self.get_if_present(DiskFile::source_id(path.reference()))
    }
```

`tracked()` on a `#[tracked]` map stays the way to depend on the set of keys (iteration). It is no longer needed for a keyed miss.

## Changes

Each change ships on its own and leaves the tree green.

1. pico: absent source nodes. `SourceNode.value` is `Option`, `source_nodes` loses its `Option`, `get_impl` creates absent nodes in memos, `set_source` treats absent as a change, `remove_source` makes nodes absent, `source_node_changed_since` compares every node. Tests. This alone fixes the never-written-counter case in i2 with no i2 code change.
2. pico / pico_macros: the `#[tracked]` counter has no `Default`. `Counter::next(previous: Option<&Self>)` replaces `Counter::increment` and the `C::default()` branch.
3. pico / pico_macros: keyed source reads without a value. `#[derive(Source)]` generates `T::source_id(&key) -> SourceId<T>`; `Database::get_if_present(id) -> Option<&T>`.
4. i2: keyed lookups read the source key directly; tracked-on-miss call sites go away; AGENTS.md and pico.md rule text changes. Later; not implemented with this doc.

## Change 1: absent source nodes

Code: the Design section above, in full. Files: `crates/pico/src/source.rs`, `crates/pico/src/database.rs`, `crates/pico/src/execute_memoized_function.rs`, `crates/pico/Cargo.toml`, `crates/pico/src/lib.rs`, `crates/pico/tests`.

`crates/pico/Cargo.toml` gains `prelude = { path = "../prelude" }` under `[dependencies]` (open decision 2). `prelude` has no dependencies of its own.

`crates/pico/src/lib.rs` gains, for the unit tests below (`#[derive(Db)]` and `#[memo]` expand to `::pico::` paths, `crates/pico_macros/src/db_macro.rs:105`, `crates/pico_macros/src/memo_macro.rs:65`):

```rust
// from crates/pico/src/lib.rs
#[cfg(test)]
extern crate self as pico;
```

### Change 1 tests

Re-invocation counts are pico's own behavior and are asserted in pico's tests. Each test owns its memo and its `static AtomicUsize` counter (the existing pico pattern, `crates/pico/tests/basic.rs:6`; open decision 1), so tests run in parallel without the `RUN_SERIALLY` lock (`crates/pico/tests/basic_multi_function_chain.rs:13`). An unrelated epoch advance is two `set`s of an `Unrelated` source with different values: the first is a vacant insert and does not advance.

`crates/pico/tests/tests.rs` gains:

```rust
// from crates/pico/tests/tests.rs
mod absent_source {
    mod garbage_collection;
    mod missing_key;
    mod remove;
}
```

and `mod never_written;` inside the existing `mod tracking_field`.

Fixture shape (each file declares its own copy, as existing pico test files do):

```rust
// from crates/pico/tests/absent_source/missing_key.rs
use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, Source, memo};
use prelude::Postfix;

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct Config {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct OtherConfig {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Unrelated {
    #[key]
    key: &'static str,
    value: String,
}

fn advance_epoch(db: &mut TestDatabase) {
    db.set(Unrelated {
        key: "unrelated",
        value: "first".to_string(),
    });
    db.set(Unrelated {
        key: "unrelated",
        value: "second".to_string(),
    });
}

static MISSING_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn missing_then_set_reader(db: &TestDatabase) -> Option<String> {
    MISSING_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn missing_then_set_reruns() {
    let mut db = TestDatabase::default();
    assert_eq!(*missing_then_set_reader(&db), None);
    db.set(Config {
        name: "a".to_string(),
    });
    assert_eq!(
        *missing_then_set_reader(&db),
        "a".to_string().wrap_some()
    );
    assert_eq!(MISSING_THEN_SET.load(Ordering::SeqCst), 2);
}
```

`advance_epoch` is called only from tests in the same file, so no `expect(dead_code)` is needed. Each test below has its own reader memo shaped like `missing_then_set_reader` and its own counter.

`absent_source/missing_key.rs`:

- `missing_then_set_reruns`: above.
- `missing_and_still_missing_across_epochs_does_not_rerun`: read `Config` (1 run, `None`). `advance_epoch` three times, reading after each: 1 run total, `None`.
- `unrelated_set_does_not_rerun_reader_of_missing_key`: read `Config` (`None`). Set `OtherConfig` twice with different values (the second is an advance); read `Config`: 1 run, `None`.
- `set_reruns_only_readers_of_that_key`: one reader of `Config`, one of `OtherConfig`, both `None`. Set `Config`. `Config` reader: 2 runs, `Some`. `OtherConfig` reader: 1 run, `None`.
- `same_missing_key_twice_in_one_memo`: the memo reads `Config` twice and returns both as a tuple. Set `Config`: 2 runs, both `Some`.
- `parent_of_missing_reader_reruns`: a parent memo returns `missing_reader(db).clone()`. Parent: 1 run, `None`. Set `Config`. Parent: 2 runs, `Some`.
- `read_outside_a_memo_then_set`: `db.get_singleton::<Config>()` from the test returns `None`. Set `Config`. A memo reads it: 1 run, `Some`. `db.get_singleton::<Config>()` from the test returns `Some`.
- `read_outside_a_memo_after_absent_node_exists`: a memo reads `Config` (`None`, which creates the absent node). `db.get_singleton::<Config>()` from the test returns `None`.

`absent_source/remove.rs` (`remove_singleton` is the keyed remove that change 1 can observe; a removed `SourceId` read with `db.get` still panics, covered by `crates/pico/tests/removing_params.rs`):

- `present_then_removed_reruns_and_returns_none`: set `Config`; read (1 run, `Some`); `remove_singleton::<Config>()`; read: 2 runs, `None`.
- `removed_then_set_reruns`: set `Config` = `"a"`; read; remove; read (`None`); set `Config` = `"a"` (equal to the value before the remove); read: 3 runs, `Some("a")`.
- `removed_and_still_removed_does_not_rerun`: set; read; remove; read (2 runs, `None`); remove again; `advance_epoch`; read: 2 runs, `None`.
- `remove_of_missing_key_does_not_rerun`: read `Config` (1 run, `None`); `remove_singleton::<Config>()`; read: 1 run.
- `remove_of_never_read_key_then_read`: `remove_singleton::<Config>()` on a fresh database (no panic); a memo reads `Config`: `None`.
- Equal re-set of a present node in the same epoch does not re-run: the existing `crates/pico/tests/overriding_same_source.rs`, unchanged.

`absent_source/garbage_collection.rs` (the database uses `Storage::new_with_capacity(1.try_into().unwrap())`, as `crates/pico/tests/garbage_collection/outer_retained.rs:17` does):

- `collected_reader_of_missing_key_sees_set`: reader of `Config` (1 run, `None`); call an unrelated memo so the LRU holds only that; `run_garbage_collection`; set `Config`; reader: 2 runs, `Some`.
- `retained_reader_of_missing_key_sees_set`: reader of `Config` (1 run, `None`) is the last top-level call; `run_garbage_collection`; reader: 1 run; set `Config`; reader: 2 runs, `Some`. The absent node and the dependency survived collection.
- `remove_after_collection`: set `Config`; reader (1 run, `Some`) is the last top-level call; `run_garbage_collection`; `remove_singleton`; reader: 2 runs, `None`.

`tracking_field/never_written.rs` (the i2 shape: `#[tracked] map: TestMap(HashMap<String, SourceId<Input>>)` as in `crates/pico/tests/tracking_field/correctness.rs:6-14`):

- `iteration_before_first_write_then_insert_reruns`: a memo collects `db.get_map().tracked().0.values()` on a fresh database (1 run, empty). `set` an `Input` and insert its id with `get_map_mut().tracked()`. The memo: 2 runs, one value.
- `untracked_miss_then_tracked_then_insert_reruns`: a memo does the `extract_iso_literals` lookup (`untracked()` get, on miss `tracked()` get) for `"a"` on a fresh database (1 run, `None`). Insert `"a"`. The memo: 2 runs, `Some`.

Unit tests in `crates/pico/src/database.rs` (`#[cfg(test)] mod tests`). The epoch is not observable through pico's public API; these tests assert the epoch directly because "a vacant insert does not advance the epoch" is the behavior 21d671d7 established and this change keeps.

```rust
// from crates/pico/src/database.rs
#[cfg(test)]
mod tests {
    use pico_macros::{Db, Singleton, memo};

    use crate::{Database, Storage};

    #[derive(Db, Default)]
    struct TestDatabase {
        storage: Storage<Self>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Singleton)]
    struct Config(u64);

    #[memo]
    fn read_config(db: &TestDatabase) -> Option<u64> {
        db.get_singleton::<Config>().map(|config| config.0)
    }

    fn epoch(db: &TestDatabase) -> usize {
        usize::from(db.storage.internal.current_epoch)
    }

    #[test]
    fn set_of_never_read_key_does_not_advance_epoch() {
        let mut db = TestDatabase::default();
        let before = epoch(&db);
        db.set(Config(1));
        assert_eq!(epoch(&db), before);
    }

    #[test]
    fn set_over_absent_node_advances_epoch() {
        let mut db = TestDatabase::default();
        read_config(&db);
        let before = epoch(&db);
        db.set(Config(1));
        assert_eq!(epoch(&db), before + 1);
    }
}
```

The same module also has:

- `read_in_a_memo_does_not_advance_epoch`: `read_config(&db)` on a fresh database; the epoch is unchanged.
- `read_outside_a_memo_does_not_create_a_node`: `db.get_singleton::<Config>()` from the test, then `db.set(Config(1))`: the epoch is unchanged (a vacant insert).
- `remove_of_present_advances_epoch`: set, then `remove_singleton`: one advance.
- `remove_of_absent_node_does_not_advance_epoch`: `read_config` (absent node), `remove_singleton`: unchanged.
- `remove_of_key_with_no_node_does_not_advance_epoch`: `remove_singleton` on a fresh database: unchanged.
- `equal_set_does_not_advance_epoch`: set `Config(1)` twice: unchanged after the second.

## Change 2: the `#[tracked]` counter has no `Default`

No values are inserted on reads, and no `Default` stands in for "not written yet". The counter's first value comes from `next(None)`.

`crates/pico/src/view.rs`, before (lines 5-7 and 67-74):

```rust
// external isograph/crates/pico/src/view.rs
pub trait Counter: Singleton + Default + Copy + Eq + 'static {
    fn increment(self) -> Self;
}

    pub fn tracked(&'a mut self) -> &'a mut T {
        let next = self
            .db
            .get_singleton::<C>()
            .map_or(C::default(), |c| c.increment());
        self.db.set(next);
        (self.projector)(self.db)
    }
```

After:

```rust
// from crates/pico/src/view.rs
pub trait Counter: Singleton + Eq + 'static {
    /// The value written after `previous`, or the first value when `previous` is `None`.
    fn next(previous: Option<&Self>) -> Self;
}

    pub fn tracked(&'a mut self) -> &'a mut T {
        let next = C::next(self.db.get_singleton::<C>());
        self.db.set(next);
        (self.projector)(self.db)
    }
```

The `Copy` bound goes too: `increment(self)` from `&C` was its only use. `View::tracked` (`view.rs:29-32`) is unchanged; under change 1 its `get_singleton` inside a memo creates the counter's absent node.

`crates/pico_macros/src/db_macro.rs`, before (lines 73-80):

```rust
// external isograph/crates/pico_macros/src/db_macro.rs
        aux_struct_defs.push(quote! {
            #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
            pub struct #counter_ident(u64);
            impl ::pico::Counter for #counter_ident {
                #[inline]
                fn increment(self) -> Self { Self(self.0.wrapping_add(1)) }
            }
        });
```

After:

```rust
// from crates/pico_macros/src/db_macro.rs
        aux_struct_defs.push(quote! {
            #[derive(Clone, Copy, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
            pub struct #counter_ident(u64);
            impl ::pico::Counter for #counter_ident {
                #[inline]
                fn next(previous: ::core::option::Option<&Self>) -> Self {
                    previous.map_or(Self(0), |previous| Self(previous.0.wrapping_add(1)))
                }
            }
        });
```

Generated for `disk_file_map`, before:

```rust
// external isograph/crates/pico_macros/src/db_macro.rs
// generated by crates/pico_macros/src/db_macro.rs
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
pub struct DiskFileMapCounter(u64);
impl ::pico::Counter for DiskFileMapCounter {
    #[inline]
    fn increment(self) -> Self { Self(self.0.wrapping_add(1)) }
}
```

After:

```rust
// generated by crates/pico_macros/src/db_macro.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
pub struct DiskFileMapCounter(u64);
impl ::pico::Counter for DiskFileMapCounter {
    #[inline]
    fn next(previous: ::core::option::Option<&Self>) -> Self {
        previous.map_or(Self(0), |previous| Self(previous.0.wrapping_add(1)))
    }
}
```

The value sequence is unchanged (0, then +1). `grep -rn "Counter\b\|increment()" crates` outside pico finds no other implementor or caller. No new tests: `tracking_field/correctness.rs`, `efficiency.rs`, and change 1's `never_written.rs` cover `MutView::tracked`.

## Change 3: keyed source reads without a value

A keyed lookup needs the `SourceId` of a key without holding a source value, and a read that returns `Option`.

`crates/pico_macros/src/source_macro.rs`, before (lines 25-61):

```rust
// external isograph/crates/pico_macros/src/source_macro.rs
    let key_field_name = fields
        .iter()
        .find(|field| {
            field.attrs.iter().any(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "key")
            })
        })
        .and_then(|field| field.ident.clone());

    let field_name = match key_field_name {
        Some(field_name) => field_name,
        None => {
            return Error::new_spanned(
                &struct_name,
                "#[key] attribute must be set on a struct field",
            )
            .to_compile_error()
            .into();
        }
    };

    let output = quote! {
        impl ::pico::Source for #struct_name {
            fn get_key(&self) -> ::pico::Key {
                use ::std::hash::{Hash, Hasher, DefaultHasher};
                let mut s = DefaultHasher::new();
                ::core::any::TypeId::of::<#struct_name>().hash(&mut s);
                self.#field_name.hash(&mut s);
                s.finish().into()
            }
        }
    };

    output.into()
}
```

After:

```rust
// from crates/pico_macros/src/source_macro.rs
    let key_field = fields.iter().find(|field| {
        field.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "key")
        })
    });

    let (field_name, field_ty) = match key_field.and_then(|field| {
        field
            .ident
            .clone()
            .map(|field_name| (field_name, field.ty.clone()))
    }) {
        Some(name_and_ty) => name_and_ty,
        None => {
            return Error::new_spanned(
                &struct_name,
                "#[key] attribute must be set on a struct field",
            )
            .to_compile_error()
            .to();
        }
    };

    let output = quote! {
        impl #struct_name {
            /// The `SourceId` of the source whose key field is `key`.
            pub fn source_id(key: &#field_ty) -> ::pico::SourceId<Self> {
                use ::std::hash::{Hash, Hasher, DefaultHasher};
                let mut s = DefaultHasher::new();
                ::core::any::TypeId::of::<#struct_name>().hash(&mut s);
                key.hash(&mut s);
                ::pico::SourceId::from(::pico::Key::from(s.finish()))
            }
        }

        impl ::pico::Source for #struct_name {
            fn get_key(&self) -> ::pico::Key {
                Self::source_id(&self.#field_name).key
            }
        }
    };

    output.to()
}
```

`&T: Hash` hashes as `T` does, so `source_id(&value.path).key == value.get_key()`: the same `Key` as today for every existing source. `Some(name_and_ty) =>` is a pattern. `.to()` requires `pico_macros` to depend on `prelude` (open decision 2); under option (b) it stays `.into()`.

Generated for `DiskFile`:

```rust
// generated by crates/pico_macros/src/source_macro.rs
impl DiskFile {
    /// The `SourceId` of the source whose key field is `key`.
    pub fn source_id(key: &RelativePathToSourceFile) -> ::pico::SourceId<Self> {
        use ::std::hash::{Hash, Hasher, DefaultHasher};
        let mut s = DefaultHasher::new();
        ::core::any::TypeId::of::<DiskFile>().hash(&mut s);
        key.hash(&mut s);
        ::pico::SourceId::from(::pico::Key::from(s.finish()))
    }
}

impl ::pico::Source for DiskFile {
    fn get_key(&self) -> ::pico::Key {
        Self::source_id(&self.path).key
    }
}
```

`Database` trait (`database.rs:22-33`), after (one method added; the rest unchanged):

```rust
// from crates/pico/src/database.rs
pub trait Database: DatabaseDyn + Sized {
    fn get_storage(&self) -> &Storage<Self>;
    fn get<T: 'static>(&self, id: SourceId<T>) -> &T;
    /// The source at `id`, or `None` when no source is set there. The caller depends on that key either way.
    fn get_if_present<T: 'static>(&self, id: SourceId<T>) -> Option<&T>;
    /// Because `T` is a `Singleton`, unlike `get` this does not require `id: SourceId<T>`
    fn get_singleton<T: 'static + Singleton>(&self) -> Option<&T>;
    fn intern_value<T: Clone + Hash + DynEq + 'static>(&self, value: T) -> MemoRef<T>;
    fn intern_ref<T: Clone + Hash + DynEq + 'static>(&self, value: &T) -> MemoRef<T>;
    fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T>;
    fn remove<T>(&mut self, id: SourceId<T>);
    fn remove_singleton<T: Singleton + 'static>(&mut self);
    fn run_garbage_collection(&mut self);
}
```

`Storage`, added next to `get` (`database.rs:123`):

```rust
// from crates/pico/src/database.rs
    pub fn get_if_present<T: 'static>(&self, id: SourceId<T>) -> Option<&T> {
        self.get_impl(id.key)
    }
```

`crates/pico_macros/src/db_macro.rs`, added after the generated `get` (line 114):

```rust
// from crates/pico_macros/src/db_macro.rs
            #[inline]
            fn get_if_present<T: 'static>(&self, id: ::pico::SourceId<T>) -> Option<&T> {
                self.storage.get_if_present(id)
            }
```

### Change 3 tests

`crates/pico/tests/get_if_present.rs` (own fixture with `Input { #[key] key: &'static str, value: String }`):

- `source_id_matches_set`: `Input::source_id(&"a") == db.set(Input { key: "a", .. })`.
- `missing_then_set_reruns`: a memo returns `db.get_if_present(Input::source_id(&"a")).map(|input| input.value.clone())`. 1 run, `None`. Set `"a"`. 2 runs, `Some`.
- `unrelated_set_does_not_rerun`: same reader; set `"b"` twice with different values; 1 run, `None`.
- `present_then_removed_reruns_and_returns_none`: set `"a"`; read; `db.remove(id)`; read: 2 runs, `None`.
- `get_if_present_outside_a_memo`: `None` before the set, `Some` after.

## Change 4 (later): i2 keyed lookups read the source key

Not implemented with this doc. `crates/isograph_*` is being edited concurrently; re-grep `untracked()` / `tracked()` before implementing. The call sites below are as of `0ecc7e2f`.

Tracked-on-miss call sites today (`grep -rn "tracked()" crates` outside pico):

- `TypeScriptHostLanguage::extract_iso_literals`, `crates/isograph_extract_typescript/src/lib.rs:68-71`.
- `IsographDb::open_file_source_id`, `crates/isograph_compiler/src/database.rs:82-87`.
- `IsographDb::disk_file_source_id`, `crates/isograph_compiler/src/database.rs:89-94`.
- `IsographDb::disk_file`, `crates/isograph_compiler/src/database.rs:166-169`, does an `untracked()` lookup with no tracked miss. Its memo callers (`text_through_last_iso_literal`, `crates/isograph_compiler/src/iso_literals.rs:53`) call `extract_iso_literals` first, which records the miss.

Not keyed reads in a memo, unchanged: the `untracked()` checks in `set_open_file`, `remove_open_file`, `remove_open_files_of_client` (`database.rs:104,115,124`, called from `handle`), and test assertions (`database.rs:238,276,294,352,608`, `crates/isograph_cli/src/state.rs:338,915,933`, `crates/isograph_cli/src/watch.rs:808`).

`crates/isograph_compiler/src/database.rs`, before (lines 64-94 and 166-169):

```rust
// from crates/isograph_compiler/src/database.rs
/// The text of `path` as `client` reads it: that client's open buffer, otherwise disk.
#[memo]
pub fn file_contents<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    match db.open_file_source_id(OpenFileKey { client, path }) {
        Some(open_file_id) => db.get(open_file_id).contents.clone().wrap_some(),
        None => db
            .get(db.disk_file_source_id(path)?)
            .contents
            .clone()
            .wrap_some(),
    }
}

impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    fn open_file_source_id(&self, key: OpenFileKey) -> Option<SourceId<OpenFile>> {
        match self.get_open_file_map().untracked().0.get(&key).copied() {
            Some(open_file_id) => open_file_id.wrap_some(),
            None => self.get_open_file_map().tracked().0.get(&key).copied(),
        }
    }

    fn disk_file_source_id(&self, path: RelativePathToSourceFile) -> Option<SourceId<DiskFile>> {
        match self.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(disk_file_id) => disk_file_id.wrap_some(),
            None => self.get_disk_file_map().tracked().0.get(&path).copied(),
        }
    }

    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        let source_id = self.get_disk_file_map().untracked().0.get(&path).copied()?;
        self.get(source_id).wrap_some()
    }
}
```

After (`open_file_source_id` and `disk_file_source_id` are deleted):

```rust
// from crates/isograph_compiler/src/database.rs
/// The text of `path` as `client` reads it: that client's open buffer, otherwise disk.
#[memo]
pub fn file_contents<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    match db.get_if_present(OpenFile::source_id(OpenFileKey { client, path }.reference())) {
        Some(open_file) => open_file.contents.clone().wrap_some(),
        None => db
            .disk_file(path)
            .map(|disk_file| disk_file.contents.clone()),
    }
}

impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        self.get_if_present(DiskFile::source_id(path.reference()))
    }
}
```

`crates/isograph_extract_typescript/src/lib.rs`, before (lines 63-72):

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographDb<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let source_id = match db.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(source_id) => source_id,
            None => db.get_disk_file_map().tracked().0.get(&path).copied()?,
        };
        let contents = db.get(source_id).contents.reference();
```

After:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographDb<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let contents = db.disk_file(path)?.contents.reference();
```

`missing_then_present_extracts` (`lib.rs:556-568`) drops its first two lines (`let other = intern_path("src/b.ts");` and `intern_file(&mut db, other, ...)`), which existed only so the counter existed before the miss. Change 1 already makes the test pass without them.

Behavior delta: inserting, opening, or removing one path no longer re-runs memos that missed a different path. A miss depends on its own key. The maps stay for iteration (`remove_disk_files_from_path`, `remove_open_files_of_client`, and a future compile memo that walks every file with `tracked()`).

### Docs amended by change 4

Proposed text only; these files are not edited by this doc.

AGENTS.md, replace the body of "## pico tracked maps" with:

```text
A keyed read depends on its key, present or absent. `db.get_if_present(DiskFile::source_id(path.reference()))` (what `IsographDb::disk_file` does), `db.get(source_id)`, and `db.get_singleton::<T>()` inside a memo record a dependency on that key. Setting a missing key, removing a present one, or replacing it with an unequal value re-invokes the reader. A change to another key does not. A miss needs nothing extra.

`#[tracked]` on a database field (`disk_file_map`, `open_file_map`) gives `get_disk_file_map()` / `get_disk_file_map_mut()`. `handle` writes the map with `tracked()`; `MutView` has no `untracked()`.

`tracked()` is for iteration. A memo that iterates the map (compile, every file) uses `tracked()`. That records a dependency on the whole map. Inserting or removing any key re-invokes it. Do not look up one key through the map in a memo; read the source key. Do not iterate `untracked()`: a newly inserted key is not seen. pico `tracking_field::correctness` pins this.

pico `absent_source` tests pin the keyed rules.
```

pico.md, "## Source": replace the paragraph at line 70 with:

```text
The type system does not stop you from holding a `SourceId` after `remove`. `remove` keeps the key and replaces its node with an absent node stamped with a new epoch. Dependents that read that source see it as changed. `get_if_present` of that id returns `None`; `get` panics. A memo that returns a `SourceId` after the source was removed is handing out a false proof.
```

and the paragraph at line 72 with:

```text
A path is not a proof. `DiskFile::source_id(&path)` is the id that path would have; `db.get_if_present(id)` returns `None` when no `DiskFile` is set there. That miss is a dependency on the key: a memo that read it inside its body re-invokes when the path is later set. `IsographDb::disk_file(path)` is that read.
```

pico.md, replace the body of "## Tracked maps" (lines 336-348) with:

```text
    struct DiskFileMap(pub HashMap<RelativePathToSourceFile, SourceId<DiskFile>>);

The map is which paths currently have a `DiskFile`. `db.set` of a `DiskFile` does not update the map. `handle` does both: intern the source, then insert into the map. Remove is the reverse.

`#[tracked]` on the field gives `get_disk_file_map()` / `get_disk_file_map_mut()`.

`tracked()` records a dependency on the whole map. Inserting or removing any path invalidates every memo that used `tracked()`. A memo that iterates every file (compile, "all client declarations") uses `tracked()`. Iterating `untracked()` is wrong: a newly inserted path is not seen.

A memo keyed by `path` does not go through the map. It reads the source key with `db.disk_file(path)`, which depends on that path whether or not a `DiskFile` is set there. Adding an unrelated file does not re-invoke a per-file extract, and a later insert of this path does. Before this rule, a miss through the map recorded nothing unless the memo also read the map `tracked()`; isograph hit this when an autofix created a file and `get_iso_literal` reused a stale `None`.
```

and in "## Iso literals", the `source_id` match and `db.get` in the `extract_iso_literals` snippet (pico.md:206-210) become `let contents = db.disk_file(path)?.contents.reference();`.

## Panics

Change 1 adds one panic for approval and removes one.

- Added (open decision 3): the `expect` in `source_node_changed_since`. Invariant: a `NodeKind::Source(key)` dependency is recorded only by `get_impl` with a memo on the stack, which always finds or creates the node at `key`, and no code removes a key from `source_node_key_to_index`. The type system cannot express that a `DashMap` lookup of a key recorded earlier hits. `UnexpectedError` is an LSP type that pico does not depend on; pico's existing invariant panics are all `expect("... This is indicative of a bug in Pico.")` (`database.rs:124-127,147-150,312-315,336-339`).
- Removed: "indexes should always point to a non-empty source node" in `set_source` (`database.rs:336-339`). A boxcar slot is never empty after change 1.
- Moved, not new: the "indexes should always be valid" `expect` of `get_source_node` moves into `source_node_at`, shared with `get_or_insert_absent_source_node`.

## Upstream divergence

AGENTS.md says pico and pico_macros are kept unchanged from upstream isograph. `diff -r` of `crates/pico` and `crates/pico_macros` against `/Users/rbalicki/code/isograph` differs today only in `crates/pico/tests/tracking_field/correctness.rs` (`.values()` for `.iter()`). Changes 1-3 diverge: absent source nodes (change 1), `Counter::next` (change 2), `T::source_id` and `Database::get_if_present` (change 3), and under open decision 2 (a), a `prelude` dependency. This needs the user's approval. Proposed AGENTS.md first paragraph, first sentence, landing with change 1:

```text
From upstream isograph we keep pico and pico_macros (unchanged except that a read of a missing source key inside a memo records a dependency through an absent node, the `#[tracked]` counter has no `Default`, and sources can be read by key with `T::source_id` and `get_if_present`; see refactors/pending/pico-missing-key-dependency.md), the swc plugin and its dependency chain (isograph_config, common_lang_types, string_key_newtype, prelude), resolve_position, the relay crates, the docs website, and the build process.
```

An upstream isograph PR with changes 1-3 would remove the divergence.

## Follow-ups and alternatives

- `TrackedMap` (the previous design of this doc). A `#[tracked_map]` field type that owns one pico node per map key plus an entries node, with `get` / `iter` / `insert` / `remove` on its views. Over this design it would add per-key dependencies for a map whose keys are not the source keys of its values (for example a `HashMap<ClientId, V>` with non-source values), and it would encapsulate writes so an `insert` of an equal entry or a `remove` of an absent key advances nothing. It is not part of this change. For `disk_file_map` and `open_file_map` the map key is the source key (`DiskFile.path`, `OpenFile.key`), so change 3 covers keyed reads without it.
- `MutView::tracked()` writes counter + 1 on every call (`view.rs:67-74`), whether or not the caller then changes the map. `set_open_file` reads the map `untracked()` first to avoid that (`database.rs:104`). Unchanged here.
- The equal arm of `set_source` (`database.rs:348-350`, kept by this change, present upstream since 21d671d7) sets `time_updated = current_epoch`. A dependency's `since` is the epoch its reader last ran (`database.rs:117`), and verification without a re-run does not rewrite it (`execute_memoized_function.rs:182-200`). So after any unrelated epoch advance, an equal re-set of a source makes its readers re-run (and then backdate). pico.md "Equality and backdating" line 332 ("Re-saving an unchanged file does not re-invoke anything") holds only when no epoch advance happened since the reader last ran. Keeping the node's previous `time_updated` in that arm would make the statement hold. Separate change.
- lsp-open-files.md change 3 ("a tracked counter that was never written") and its open decision are resolved by change 1 of this doc without either of its options (a) or (b): `IsographDb::default` writes no counter, and `#[derive(Db)]` generates no constructor.

## Open decisions

1. Test counters. The new pico tests count memo-body executions with a `static AtomicUsize` per test, the existing pico test pattern (`crates/pico/tests/basic.rs:6`). AGENTS.md bans statics and atomics for ambient program state; these are test-only. A memo body is a `fn` with no state of its own, so the count has to live outside it; a per-test static avoids the `LazyLock<Mutex<()>>` serialization lock that `basic_multi_function_chain.rs:13` uses. (a) Approve the statics. (b) Name another counting mechanism.
2. Postfix in pico. The pending-doc postfix lint (`crates/prelude/src/postfix_constructors.rs:48-58`) rejects `Some(`, `Box::new`, and `.into()` in this doc's `// from` snippets, and pico's own sources are outside `RUST_ROOTS` (`postfix_constructors.rs:17-32`). (a) pico and pico_macros depend on `prelude` and the new code uses `Postfix` (`wrap_some`, `boxed`, `.to()`), as written above. (b) pico takes no `prelude` dependency; the new code uses `Some(` / `Box::new` / `.into()` like the rest of pico, and the after snippets are rewritten as `// external` or the lint gains a pico exemption.
3. `source_node_changed_since` on a key with no node. (a) `expect`, as written above (see Panics). (b) Keep `None => true`, so a broken invariant re-runs the reader instead of panicking.
