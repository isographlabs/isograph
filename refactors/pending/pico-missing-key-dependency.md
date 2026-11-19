# pico records dependencies on missing keys

A memo that reads a key from a tracked map depends on the value held by that key, an `Option<V>`. A hit and a miss are the same kind of dependency. Inserting the key (`None` to `Some`), removing it (`Some` to `None`), or replacing its value with an unequal one re-invokes the readers of that key and no other memo. A key that stays absent stays equal, and its readers do not re-invoke. Iterating the map is a separate dependency on the map's entries.

pico provides this as a map type, `TrackedMap<K, V>`, on a `#[tracked_map]` database field. It replaces `#[tracked]` and its `View` / `MutView` / `Counter`. Source `get` / `set` / `remove` keep their upstream semantics, including a vacant `set` not advancing the epoch (upstream 21d671d7).

## Problem in the current code

- `View::tracked()` reads the field's counter with `get_singleton` (`crates/pico/src/view.rs:29-32`). `Storage::get_impl` returns `None` before registering a dependency when the key has no node (`crates/pico/src/database.rs:134-140`; the `?` is on line 135). A singleton is a source whose key is the hash of its `TypeId` (`crates/pico_macros/src/singleton_macro.rs:10-23`), so the same path applies.
- The counter does not exist until the first `MutView::tracked()` (`crates/pico/src/view.rs:67-74`), which writes `C::default()` when `get_singleton` returns `None`. That first write is a vacant `set_source`, which inserts with `time_updated: current_epoch` and does not advance the epoch (`crates/pico/src/database.rs:352-358`). A memo that read the map `tracked()` before that write recorded nothing, and `node_verified_in_current_epoch` (`database.rs:258-263`) or an empty dependency list makes the next call reuse the stale result.
- `MutView::tracked()` writes `counter + 1` on every call (`view.rs:67-74`), whether or not the caller then changes the map. An insert of an already-present path or a remove of an absent path re-invokes every memo that read the map `tracked()`.
- A keyed `untracked()` lookup that misses records nothing. pico.md and AGENTS.md make every memo author pair it with a `tracked()` read on the miss, which depends on every key of the map. `TypeScriptHostLanguage::extract_iso_literals` does this (`crates/isograph_extract_typescript/src/lib.rs:68-71`). `IsographDb::disk_file` does not (`crates/isograph_compiler/src/database.rs:67-70`), so a memo that calls it on a missing path is stale after the file appears.
- `missing_then_present_extracts` (`crates/isograph_extract_typescript/src/lib.rs:556-567`) inserts an unrelated `src/b.ts` first so the counter exists before the miss.

## Design

Each `TrackedMap` field owns pico source nodes in the database's existing source storage (`source_node_key_to_index: DashMap<Key, Index<SourceNode>>` and `source_nodes: BoxcarVec<Option<SourceNode>>`, `crates/pico/src/database.rs:72,76`):

- One slot per key, a `TrackedMapSlot<M, K, V>` whose value is `Option<V>`. Its `Key` is the hash of `TypeId::<TrackedMapSlot<M, K, V>>` and `K`, the same scheme `#[derive(Source)]` uses (`crates/pico_macros/src/source_macro.rs:49-58`). `M` is a marker type that `#[derive(Db)]` generates per field, so two maps with the same `K` and `V` have different slot keys.
- One entries node, a `TrackedMapEntries<M>` singleton holding a generation number.

Reads go through `&Db` and create a missing node. Writes go through `&mut Db` and the public `Database::set`.

- `get(&key)` reads K's slot. If the slot does not exist, `get` inserts it holding `None` and stamped with the current epoch, then registers the dependency. The slot is a present node with value `None` from then on.
- `iter()` reads the entries node the same way, inserting it if it does not exist, then iterates the field's `HashMap`.
- `insert(key, value)` writes the `HashMap`. If the entry changed (it was absent or held an unequal value), it `set`s K's slot to `value.wrap_some()` and `set`s the entries node to the next generation.
- `remove(&key)` removes from the `HashMap`. If the key was present, it `set`s K's slot to `None` and the entries node to the next generation.

### Why creation during a read is invisible

The reader registers `Dependency { node_to: NodeKind::Source(slot_key), time_verified_or_updated: current_epoch }` (`database.rs:109-121`). Verification asks `source_node_changed_since(slot_key, since)`, which is `time_updated > since` for a present node (`crates/pico/src/execute_memoized_function.rs:202-207`). The new slot's `time_updated` is the current epoch, which equals the reader's `since`, so the slot reads as unchanged. Creation does not advance the epoch, so `node_verified_in_current_epoch` of every other memo is unaffected. No other memo can depend on a node that did not exist. The same stamping is what `intern_value` does for a node it creates during a read (`database.rs:402-419`).

A missing slot means the key was never in the map: every change to an entry `set`s that key's slot (below), and the `HashMap` is a private field that only `TrackedMapMutView` writes. So a slot that a read creates always holds `None`, and that is the key's true value.

### Why a write re-invokes exactly the readers of K

- A slot that a reader created exists. `insert` of K `set`s it from `None` to `Some(value)`. That is the Occupied-unequal arm of `set_source` (`database.rs:340-347`): the epoch advances and the slot's `time_updated` is the new epoch, which is greater than every reader's `since`. Readers of K re-invoke. Readers of other keys hold dependencies on other slots, whose `time_updated` did not change.
- A slot nobody read and nobody wrote does not exist. `insert` of that key is a vacant `set` (`database.rs:352-358`): no epoch bump, which is correct because no memo depends on it.
- `remove` `set`s the slot to `None`: Occupied-unequal, the epoch advances. The slot is kept, so there are no tombstones and `source_node_changed_since`'s `None => true` arm is never reached for a slot.
- An `insert` of a value equal to the present one, or a `remove` of an absent key, does not `set` anything. The epoch does not advance and nothing re-invokes. The map never `set`s a slot to an equal value, so the Occupied-equal arm's `source_node.time_updated = self.current_epoch` (`database.rs:348-350`) is not reached for slots.
- A `SourceId` value whose source is re-`set` with new contents is the same `SourceId`. The map entry is unchanged, no slot is written, and readers that `db.get` the id re-invoke through the source's own node.

### Entries node

`iter()` depends on the entries node. Every changed entry advances its generation: a new key, a removed key, or an unequal value at a present key (the iterator yields values, so a changed value is a changed result). An equal re-insert does not. The node is created by the first `iter()` or the first changed write, whichever is first, so a memo that iterates before any write records a dependency on an existing node, and the first insert after that is an Occupied-unequal `set` that re-invokes it. The never-written-counter hole cannot occur.

### Interior mutability

Creating a node during a read uses `DashMap::entry` on `source_node_key_to_index` and `boxcar::Vec::push` on `source_nodes`. Both are already written through `&self` during memo execution: `insert_source_node` takes `&self` (`database.rs:319-321`), and `intern_value` inserts into `derived_node_id_to_revision` through `&self` (`database.rs:396-419`). No new primitive. `Storage` is `!Sync` because `DependencyStack` is a `RefCell` (`crates/pico/src/dependency.rs:55`), so these are single-thread writes.

### Memory and garbage collection

Source nodes are not garbage collected (`crates/pico/src/garbage_collection.rs:21`; `add_dependencies_to_queue` skips `NodeKind::Source`, `garbage_collection.rs:136-137`). A slot exists for every key ever read through `get` or ever inserted, and lives as long as the database. A slot is one `DashMap` entry and one boxed `TrackedMapSlot { key: K, value: Option<V> }`. For `disk_file_map` that is an interned path plus a `SourceId` per path ever seen, including paths an LSP request asked about that never existed. A memo collected by GC loses its dependency list, not the slot. The next call re-creates the memo and reads the existing slot.

`V` is stored twice for a key that has a slot: in the field's `HashMap` (for `iter`) and in the slot (for `get` and the equality check). For a `SourceId` value that is a `Copy` key.

## End-user experience

A memo author calls `get` for a key and `iter` for the whole map. There is no tracked/untracked choice. A miss is safe.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographDb<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let contents = db.disk_file(path)?.contents.reference();
        // ... unchanged
    }
```

- `extract_iso_literals(db, path)` on a missing path returns `None`. Inserting a `DiskFile` at another path does not re-invoke it. Inserting a `DiskFile` at `path` re-invokes it, and it returns `Some`.
- Re-saving an unchanged file re-invokes nothing (`set` of an equal `DiskFile`, `database.rs:348-350`; the map entry is the same `SourceId`).
- A compile memo that iterates `get_disk_file_map().iter()` re-invokes when a file is added or removed, and not when one file's contents change (that is a dependency on that `DiskFile`, read through `db.get`).

## Changes

Three changes. Change 1 is pico only and changes nothing for existing users. Change 2 migrates i2. Change 3 deletes `#[tracked]`.

1. pico: `TrackedMap`, `TrackedMapView`, `TrackedMapMutView`, `Storage::get_or_insert`, and `#[tracked_map]`.
2. i2: `disk_file_map` becomes `#[tracked_map]`; every call site migrates.
3. pico: delete `#[tracked]`, `View`, `MutView`, `Counter`, and the `tracking_field` tests.

## Change 1: `TrackedMap` in pico

### `crates/pico/src/tracked_map.rs` (new)

```rust
// from crates/pico/src/tracked_map.rs
use std::{
    any::TypeId,
    collections::{HashMap, hash_map},
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
};

use prelude::Postfix;

use crate::{Database, Key, Projector, ProjectorMut, Singleton, Source};

/// A map on a `#[tracked_map]` database field.
#[derive(Debug)]
pub struct TrackedMap<K, V>(HashMap<K, V>);

impl<K, V> Default for TrackedMap<K, V> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

/// Read access to a `TrackedMap` field whose marker is `M`.
pub struct TrackedMapView<'a, Db: Database, M, T> {
    db: &'a Db,
    projector: Projector<Db, T>,
    phantom: PhantomData<M>,
}

impl<'a, Db: Database, M, T> TrackedMapView<'a, Db, M, T> {
    pub fn new(db: &'a Db, projector: Projector<Db, T>) -> Self {
        Self {
            db,
            projector,
            phantom: PhantomData,
        }
    }
}

impl<'a, Db: Database, M: 'static, K: Hash + Eq + Clone + 'static, V: PartialEq + 'static>
    TrackedMapView<'a, Db, M, TrackedMap<K, V>>
{
    /// The value held by `key`. The caller depends on that value, present or absent, and on no other key.
    pub fn get(&self, key: &K) -> Option<&'a V> {
        self.db
            .get_storage()
            .get_or_insert(TrackedMapSlot::<M, K, V>::source_key(key), || TrackedMapSlot {
                key: key.clone(),
                value: None,
                phantom: PhantomData,
            })
            .value
            .as_ref()
    }

    /// Every entry. The caller depends on the entries: a new key, a removed key, or an unequal value at a present key.
    pub fn iter(&self) -> hash_map::Iter<'a, K, V> {
        self.db.get_storage().get_or_insert(
            TrackedMapEntries::<M>::get_singleton_key(),
            TrackedMapEntries::<M>::initial,
        );
        (self.projector)(self.db).0.iter()
    }
}

/// Write access to a `TrackedMap` field whose marker is `M`.
pub struct TrackedMapMutView<'a, Db: Database, M, T> {
    db: &'a mut Db,
    projector: ProjectorMut<Db, T>,
    phantom: PhantomData<M>,
}

impl<'a, Db: Database, M, T> TrackedMapMutView<'a, Db, M, T> {
    pub fn new(db: &'a mut Db, projector: ProjectorMut<Db, T>) -> Self {
        Self {
            db,
            projector,
            phantom: PhantomData,
        }
    }
}

impl<Db: Database, M: 'static, K: Hash + Eq + Clone + 'static, V: Clone + PartialEq + 'static>
    TrackedMapMutView<'_, Db, M, TrackedMap<K, V>>
{
    /// Inserts `value` at `key` and returns the previous value.
    pub fn insert(self, key: K, value: V) -> Option<V> {
        let previous = (self.projector)(self.db)
            .0
            .insert(key.clone(), value.clone());
        if previous.as_ref() != value.reference().wrap_some() {
            set_slot::<Db, M, K, V>(self.db, key, value.wrap_some());
        }
        previous
    }

    /// Removes `key` and returns its value.
    pub fn remove(self, key: &K) -> Option<V> {
        let previous = (self.projector)(self.db).0.remove(key)?;
        set_slot::<Db, M, K, V>(self.db, key.clone(), None);
        previous.wrap_some()
    }
}

fn set_slot<Db: Database, M: 'static, K: Hash + PartialEq + 'static, V: PartialEq + 'static>(
    db: &mut Db,
    key: K,
    value: Option<V>,
) {
    db.set(TrackedMapSlot::<M, K, V> {
        key,
        value,
        phantom: PhantomData,
    });
    let entries = db
        .get_singleton::<TrackedMapEntries<M>>()
        .map_or(TrackedMapEntries::initial(), TrackedMapEntries::next);
    db.set(entries);
}

/// The value held by one key of the `TrackedMap` whose marker is `M`.
pub(crate) struct TrackedMapSlot<M, K, V> {
    key: K,
    value: Option<V>,
    phantom: PhantomData<M>,
}

impl<M: 'static, K: Hash + 'static, V: 'static> TrackedMapSlot<M, K, V> {
    fn source_key(key: &K) -> Key {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<Self>().hash(&mut hasher);
        key.hash(&mut hasher);
        Key::from(hasher.finish())
    }
}

impl<M: 'static, K: Hash + 'static, V: 'static> Source for TrackedMapSlot<M, K, V> {
    fn get_key(&self) -> Key {
        Self::source_key(self.key.reference())
    }
}

impl<M, K: PartialEq, V: PartialEq> PartialEq for TrackedMapSlot<M, K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value
    }
}

/// The generation of the entries of the `TrackedMap` whose marker is `M`.
pub(crate) struct TrackedMapEntries<M> {
    generation: u64,
    phantom: PhantomData<M>,
}

impl<M> TrackedMapEntries<M> {
    fn initial() -> Self {
        Self {
            generation: 0,
            phantom: PhantomData,
        }
    }

    fn next(&self) -> Self {
        Self {
            generation: self.generation.wrapping_add(1),
            phantom: PhantomData,
        }
    }
}

impl<M: 'static> Singleton for TrackedMapEntries<M> {
    fn get_singleton_key() -> Key {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<Self>().hash(&mut hasher);
        Key::from(hasher.finish())
    }
}

impl<M: 'static> Source for TrackedMapEntries<M> {
    fn get_key(&self) -> Key {
        Self::get_singleton_key()
    }
}

impl<M> PartialEq for TrackedMapEntries<M> {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
    }
}
```

`PartialEq` is written by hand on `TrackedMapSlot` and `TrackedMapEntries` because `#[derive(PartialEq)]` would bound `M: PartialEq`, and the marker carries no data.

`TrackedMapSlot` and `TrackedMapEntries` are `pub(crate)`. `Database::set` and `get_singleton` are generic, so pico instantiates them with crate-private types.

### `Storage::get_or_insert` (new, `crates/pico/src/database.rs`)

```rust
// from crates/pico/src/database.rs
impl<Db: Database> Storage<Db> {
    /// The source at `key`, after inserting `make_source()` there if `key` has no node. Registers a dependency either way. An inserted node's `time_updated` is the current epoch. `make_source` runs while `source_node_key_to_index` holds the entry's shard lock, so it must not read the database.
    pub(crate) fn get_or_insert<T: DynEq>(&self, key: Key, make_source: impl FnOnce() -> T) -> &T {
        if let Entry::Vacant(vacant_entry) = self.internal.source_node_key_to_index.entry(key) {
            vacant_entry.insert(self.internal.insert_source_node(SourceNode {
                time_updated: self.internal.current_epoch,
                value: make_source().boxed(),
            }));
        }
        self.get_impl(key).expect(
            "get_or_insert inserted a node at this key if it had none. \
            This is indicative of a bug in Pico.",
        )
    }
}
```

`get_impl` (`database.rs:134-152`) is unchanged; it registers `NodeKind::Source(key)` with the node's `time_updated` and downcasts.

### `crates/pico/src/lib.rs`

Before:

```rust
// from crates/pico/src/lib.rs
mod source;
mod view;
mod with_serialize;

pub use source::*;
pub use view::*;
pub use with_serialize::*;
```

After:

```rust
// from crates/pico/src/lib.rs
mod source;
mod tracked_map;
mod view;
mod with_serialize;

pub use source::*;
pub use tracked_map::*;
pub use view::*;
pub use with_serialize::*;
```

### `crates/pico/Cargo.toml`

Add `prelude = { path = "../prelude" }` under `[dependencies]`. `prelude` has no dependencies (`crates/prelude/Cargo.toml`). Open decision 3 covers this.

### `#[derive(Db)]`

`crates/pico_macros/src/lib.rs` before:

```rust
// from crates/pico_macros/src/lib.rs
#[proc_macro_derive(Db, attributes(tracked))]
pub fn db(input: TokenStream) -> TokenStream {
    db_macro::db_macro(input)
}
```

After:

```rust
// from crates/pico_macros/src/lib.rs
#[proc_macro_derive(Db, attributes(tracked, tracked_map))]
pub fn db(input: TokenStream) -> TokenStream {
    db_macro::db_macro(input)
}
```

`crates/pico_macros/src/db_macro.rs` before (lines 6-96):

```rust
// from crates/pico_macros/src/db_macro.rs
pub(crate) fn db_macro(item: TokenStream) -> TokenStream {
    struct TrackedField {
        field_ident: syn::Ident,
        field_ty: Type,
    }

    let input = parse_macro_input!(item as ItemStruct);

    let fields = match &input.fields {
        syn::Fields::Named(named) => named,
        _ => {
            return syn::Error::new_spanned(&input.fields, "#[db] requires named fields")
                .to_compile_error()
                .into();
        }
    };

    let struct_ident = input.ident.clone();
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let mut tracked = Vec::new();
    let mut errors = Vec::new();

    for f in fields.named.iter() {
        let field_ident = f.ident.as_ref().unwrap();
        if has_attr(&f.attrs, "tracked") {
            if !matches!(f.vis, Visibility::Inherited) {
                errors.push(
                    syn::Error::new(
                        f.vis.span(),
                        "fields marked `#[tracked]` must be private (remove any `pub` visibility)",
                    )
                    .to_compile_error(),
                );
            }
            tracked.push(TrackedField {
                field_ident: field_ident.clone(),
                field_ty: f.ty.clone(),
            });
        }
    }

    if !errors.is_empty() {
        return quote! { #(#errors)* }.into();
    }

    let mut struct_impl = Vec::new();
    let mut aux_struct_defs = Vec::new();

    for TrackedField {
        field_ident,
        field_ty,
    } in tracked
    {
        let counter_ident =
            format_ident!("{}Counter", field_ident.to_string().to_case(Case::Pascal));
        let proj_ident = format_ident!(
            "{}_PROJECTOR",
            field_ident.to_string().to_case(Case::UpperSnake)
        );
        let proj_ident_mut = format_ident!(
            "{}_PROJECTOR_MUT",
            field_ident.to_string().to_case(Case::UpperSnake)
        );
        let get_ident = format_ident!("get_{field_ident}");
        let get_ident_mut = format_ident!("get_{field_ident}_mut");

        aux_struct_defs.push(quote! {
            #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
            pub struct #counter_ident(u64);
            impl ::pico::Counter for #counter_ident {
                #[inline]
                fn increment(self) -> Self { Self(self.0.wrapping_add(1)) }
            }
        });

        struct_impl.push(quote! {
            const #proj_ident: ::pico::Projector<#struct_ident #type_generics, #field_ty> = |db: &#struct_ident #type_generics| &db.#field_ident;
            const #proj_ident_mut: ::pico::ProjectorMut<#struct_ident #type_generics, #field_ty> = |db: &mut #struct_ident #type_generics| &mut db.#field_ident;

            #[inline]
            pub fn #get_ident(&self) -> ::pico::View<'_, #struct_ident #type_generics, #field_ty, #counter_ident> {
                ::pico::View::new(self, Self::#proj_ident)
            }

            #[inline]
            pub fn #get_ident_mut(&mut self) -> ::pico::MutView<'_, #struct_ident #type_generics, #field_ty, #counter_ident> {
                ::pico::MutView::new(self, Self::#proj_ident_mut)
            }
        });
    }
```

After (the rest of the function, from `let output = quote! {` on, is unchanged):

```rust
// from crates/pico_macros/src/db_macro.rs
pub(crate) fn db_macro(item: TokenStream) -> TokenStream {
    enum TrackedFieldKind {
        Tracked,
        TrackedMap,
    }

    struct TrackedField {
        field_ident: syn::Ident,
        field_ty: Type,
        kind: TrackedFieldKind,
    }

    let input = parse_macro_input!(item as ItemStruct);

    let fields = match &input.fields {
        syn::Fields::Named(named) => named,
        _ => {
            return syn::Error::new_spanned(&input.fields, "#[db] requires named fields")
                .to_compile_error()
                .into();
        }
    };

    let struct_ident = input.ident.clone();
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let mut tracked = Vec::new();
    let mut errors = Vec::new();

    for f in fields.named.iter() {
        let field_ident = f.ident.as_ref().unwrap();
        let kind = match (has_attr(&f.attrs, "tracked"), has_attr(&f.attrs, "tracked_map")) {
            (false, false) => continue,
            (true, false) => TrackedFieldKind::Tracked,
            (false, true) => TrackedFieldKind::TrackedMap,
            (true, true) => {
                errors.push(
                    syn::Error::new(
                        f.span(),
                        "a field is marked `#[tracked]` or `#[tracked_map]`, not both",
                    )
                    .to_compile_error(),
                );
                continue;
            }
        };
        if !matches!(f.vis, Visibility::Inherited) {
            errors.push(
                syn::Error::new(
                    f.vis.span(),
                    "fields marked `#[tracked]` or `#[tracked_map]` must be private (remove any `pub` visibility)",
                )
                .to_compile_error(),
            );
        }
        tracked.push(TrackedField {
            field_ident: field_ident.clone(),
            field_ty: f.ty.clone(),
            kind,
        });
    }

    if !errors.is_empty() {
        return quote! { #(#errors)* }.into();
    }

    let mut struct_impl = Vec::new();
    let mut aux_struct_defs = Vec::new();

    for TrackedField {
        field_ident,
        field_ty,
        kind,
    } in tracked
    {
        let proj_ident = format_ident!(
            "{}_PROJECTOR",
            field_ident.to_string().to_case(Case::UpperSnake)
        );
        let proj_ident_mut = format_ident!(
            "{}_PROJECTOR_MUT",
            field_ident.to_string().to_case(Case::UpperSnake)
        );
        let get_ident = format_ident!("get_{field_ident}");
        let get_ident_mut = format_ident!("get_{field_ident}_mut");

        let projectors = quote! {
            const #proj_ident: ::pico::Projector<#struct_ident #type_generics, #field_ty> = |db: &#struct_ident #type_generics| &db.#field_ident;
            const #proj_ident_mut: ::pico::ProjectorMut<#struct_ident #type_generics, #field_ty> = |db: &mut #struct_ident #type_generics| &mut db.#field_ident;
        };

        match kind {
            TrackedFieldKind::Tracked => {
                let counter_ident =
                    format_ident!("{}Counter", field_ident.to_string().to_case(Case::Pascal));

                aux_struct_defs.push(quote! {
                    #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
                    pub struct #counter_ident(u64);
                    impl ::pico::Counter for #counter_ident {
                        #[inline]
                        fn increment(self) -> Self { Self(self.0.wrapping_add(1)) }
                    }
                });

                struct_impl.push(quote! {
                    #projectors

                    #[inline]
                    pub fn #get_ident(&self) -> ::pico::View<'_, #struct_ident #type_generics, #field_ty, #counter_ident> {
                        ::pico::View::new(self, Self::#proj_ident)
                    }

                    #[inline]
                    pub fn #get_ident_mut(&mut self) -> ::pico::MutView<'_, #struct_ident #type_generics, #field_ty, #counter_ident> {
                        ::pico::MutView::new(self, Self::#proj_ident_mut)
                    }
                });
            }
            TrackedFieldKind::TrackedMap => {
                let marker_ident =
                    format_ident!("{}Marker", field_ident.to_string().to_case(Case::Pascal));

                aux_struct_defs.push(quote! {
                    pub struct #marker_ident;
                });

                struct_impl.push(quote! {
                    #projectors

                    #[inline]
                    pub fn #get_ident(&self) -> ::pico::TrackedMapView<'_, #struct_ident #type_generics, #marker_ident, #field_ty> {
                        ::pico::TrackedMapView::new(self, Self::#proj_ident)
                    }

                    #[inline]
                    pub fn #get_ident_mut(&mut self) -> ::pico::TrackedMapMutView<'_, #struct_ident #type_generics, #marker_ident, #field_ty> {
                        ::pico::TrackedMapMutView::new(self, Self::#proj_ident_mut)
                    }
                });
            }
        }
    }
```

`get` / `iter` / `insert` / `remove` exist only when the field type is `TrackedMap<K, V>`, because they are on `impl ... TrackedMapView<'a, Db, M, TrackedMap<K, V>>`. A `#[tracked_map]` field of another type compiles, and its view has no methods; the first call site fails with "no method named `get`". The macro does not parse the field type.

`f.span()` needs `syn::spanned::Spanned`, already imported (`db_macro.rs:4`).

### Change 1 tests

`crates/pico/tests/tests.rs` gains:

```rust
// from crates/pico/tests/tests.rs
mod tracked_map {
    mod entries;
    mod garbage_collection;
    mod keyed;
    mod source_id_values;
}
```

Every test owns its memos and its `static` `AtomicUsize` counters, so tests run in parallel without the `RUN_SERIALLY` mutex some existing pico tests use. Counting executions of a memo body with a `static AtomicUsize` is the existing pico test pattern (`crates/pico/tests/basic.rs:6`); open decision 2 covers it. `pico`'s `[dev-dependencies]` needs nothing new: `prelude` is a dependency after change 1.

Shared fixture shape (each file declares its own copy, as existing pico test files do):

```rust
// from crates/pico/tests/tracked_map/keyed.rs
use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage, TrackedMap};
use pico_macros::{Db, Source, memo};
use prelude::Postfix;

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
    #[tracked_map]
    map: TrackedMap<&'static str, String>,
    #[tracked_map]
    other_map: TrackedMap<&'static str, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    key: &'static str,
    value: String,
}

static MISSING_THEN_INSERTED: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn missing_then_inserted_reader(db: &TestDatabase, key: &'static str) -> Option<String> {
    MISSING_THEN_INSERTED.fetch_add(1, Ordering::SeqCst);
    db.get_map().get(key.reference()).cloned()
}

#[test]
fn missing_then_inserted_reruns() {
    let mut db = TestDatabase::default();
    assert_eq!(*missing_then_inserted_reader(&db, "a"), None);
    db.get_map_mut().insert("a", "x".to_string());
    assert_eq!(
        *missing_then_inserted_reader(&db, "a"),
        "x".to_string().wrap_some()
    );
    assert_eq!(MISSING_THEN_INSERTED.load(Ordering::SeqCst), 2);
}
```

An unrelated epoch advance in these tests is `db.set(Input { key: "unrelated", value })` with a value unequal to the previous one; the first `set` of that key is vacant and does not advance the epoch, so a test that needs an advance `set`s it twice with different values.

`tracked_map/keyed.rs` (one `reader` memo per test, shaped like `missing_then_inserted_reader`, with its own counter):

- `missing_then_inserted_reruns`: above.
- `first_read_creates_slot_without_rerun`: read `"a"` (1 run, `None`). Read again in the same epoch: 1 run. Advance the epoch with an unrelated `Input`; read: 1 run, `None`.
- `missing_and_still_missing_across_epochs_does_not_rerun`: read `"a"` (`None`). Advance the epoch three times with unrelated `Input` sets, reading after each: 1 run total.
- `unrelated_insert_does_not_rerun`: read `"a"` (`None`). Insert `"b"`; read `"a"`: 1 run, `None`.
- `insert_reruns_only_readers_of_that_key`: read `"a"`, `"b"`, `"c"` (3 runs, each `None`). Insert `"b"`. Read all three: 4 runs; `"b"` is `Some("x")`, `"a"` and `"c"` are `None`.
- `present_then_removed_reruns`: insert `"a"`; read (1 run, `Some`); remove `"a"`; read: 2 runs, `None`.
- `removed_then_reinserted_equal_reruns`: insert `"a"` = `"x"`; read; remove; read (`None`); insert `"a"` = `"x"`; read: 3 runs, `Some("x")`.
- `equal_reinsert_does_not_rerun`: insert `"a"` = `"x"`; read (1 run); insert `"a"` = `"x"`; read: 1 run. `insert` returns `"x".to_string().wrap_some()`.
- `unequal_reinsert_reruns`: insert `"a"` = `"x"`; read; insert `"a"` = `"y"`; read: 2 runs, `Some("y")`.
- `remove_of_absent_key_does_not_rerun`: read `"a"` (`None`); remove `"a"` (returns `None`); read: 1 run.
- `same_key_in_two_maps_does_not_interfere`: a reader of `map` at `"a"` and a reader of `other_map` at `"a"`, both `None`. Insert `"a"` into `other_map`. The `map` reader: 1 run, `None`. The `other_map` reader: 2 runs, `Some`.
- `read_outside_a_memo_then_insert`: call `db.get_map().get(&"a")` from the test (no memo on the stack; returns `None`). Then a memo reads `"a"` (1 run); insert `"a"`; read: 2 runs, `Some`.
- `same_missing_key_twice_in_one_memo`: the memo reads `"a"` twice and returns both. Insert `"a"`: 2 runs, both `Some`.
- `parent_of_a_missing_reader_reruns`: a parent memo returns `reader(db, "a").clone()`. Call the parent (1 parent run, `None`). Insert `"a"`. Call the parent: 2 parent runs, `Some`. Call a second parent of the same `reader` that was first called before the insert: it returns `Some` (the child's `time_updated` advanced with its changed value).

`tracked_map/entries.rs` (an `iterating` memo returns `db.get_map().iter().map(|(key, value)| (*key, value.clone())).collect::<Vec<_>>()` sorted by key, with its own counter per test):

- `iteration_before_any_write_then_insert_reruns`: iterate an empty map (1 run, empty). Insert `"a"`. Iterate: 2 runs, one entry. This is the never-written-counter case.
- `iteration_reruns_on_new_key`: insert `"a"`; iterate; insert `"b"`; iterate: 2 runs, two entries.
- `iteration_reruns_on_remove`: insert `"a"`, `"b"`; iterate; remove `"a"`; iterate: 2 runs, one entry.
- `iteration_reruns_on_unequal_value`: insert `"a"` = `"x"`; iterate; insert `"a"` = `"y"`; iterate: 2 runs.
- `iteration_does_not_rerun_on_equal_reinsert`: insert `"a"` = `"x"`; iterate; insert `"a"` = `"x"`; iterate: 1 run.
- `iteration_does_not_rerun_on_remove_of_absent_key`: insert `"a"`; iterate; remove `"b"`; iterate: 1 run.
- `keyed_read_does_not_rerun_on_iteration_change`: insert `"a"`; a keyed reader of `"a"` (1 run); insert `"b"`; read `"a"`: 1 run.

`tracked_map/source_id_values.rs` (the map is `TrackedMap<&'static str, SourceId<Input>>`):

- `source_reset_reruns_through_the_source`: `set` an `Input` `"a"` = `"x"` and insert its id at `"a"`. A `contents` memo does `db.get_map().get(&key).map(|id| db.get(*id).value.clone())`; a `has_id` memo returns `db.get_map().get(&key).is_some()`. Both run once. `set` `Input` `"a"` = `"y"` (same id). `contents`: 2 runs, `Some("y")`. `has_id`: 1 run.
- `source_reset_equal_does_not_rerun`: as above, then `set` `Input` `"a"` = `"x"` again. `contents`: 1 run.
- `insert_of_same_id_does_not_rerun`: `set` `Input` `"a"` and insert its id; read; `set` `Input` `"a"` = `"y"` and insert the same id again (the `insert_disk_file` shape). The `has_id` reader: 1 run. An iterating memo: 1 run.

`tracked_map/garbage_collection.rs` (the database uses `Storage::new_with_capacity(1.try_into().unwrap())`, as `crates/pico/tests/garbage_collection/basic_gc.rs:333` does):

- `collected_reader_of_missing_key_sees_insert`: reader of `"a"` (1 run, `None`); call an unrelated memo so the LRU holds only that; `run_garbage_collection`; insert `"a"`; reader of `"a"`: 2 runs, `Some`.
- `retained_reader_of_missing_key_sees_insert`: reader of `"a"` (1 run, `None`) is the last top-level call; `run_garbage_collection`; reader of `"a"` again: 1 run (retained, slot unchanged); insert `"a"`; reader: 2 runs, `Some`. The slot and the dependency survived collection.
- `remove_after_collection`: insert `"a"`; reader (1 run, `Some`); `run_garbage_collection` with the reader retained; remove `"a"`; reader: 2 runs, `None`.

## Change 2: migrate `disk_file_map`

`disk_file_map` is the only tracked field in i2 (`grep -rn '#\[tracked\]' crates` outside pico finds `crates/isograph_compiler/src/database.rs:14` only). Its values are `SourceId<DiskFile>`, and `DiskFile`'s `#[key]` is `path`, the map key (`database.rs:31-36`). Re-grep `get_disk_file_map` before implementing: `crates/isograph_cli` is being edited concurrently, so its line numbers below may have moved.

### `crates/isograph_compiler/src/database.rs`

Before (lines 11-70):

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    phantom_data: PhantomData<THostLanguage>,
}

impl<THostLanguage: HostLanguage> Default for IsographDb<THostLanguage> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<RelativePathToSourceFile, SourceId<DiskFile>>);

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

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| key.as_ref().starts_with(path.as_ref()))
            .map(|(_, source_id)| source_id)
            .collect();
        for source_id in ids {
            self.remove(source_id);
        }
    }

    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        let source_id = self.get_disk_file_map().untracked().0.get(&path).copied()?;
        self.get(source_id).wrap_some()
    }
}
```

After:

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked_map]
    disk_file_map: TrackedMap<RelativePathToSourceFile, SourceId<DiskFile>>,
    phantom_data: PhantomData<THostLanguage>,
}

impl<THostLanguage: HostLanguage> Default for IsographDb<THostLanguage> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: TrackedMap::default(),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: RelativePathToSourceFile,
    pub contents: String,
}

impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        let source_id = self.set(DiskFile { path, contents });
        self.get_disk_file_map_mut().insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().remove(path.reference()) {
            self.remove(source_id);
        }
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let descendants: Vec<_> = self
            .get_disk_file_map()
            .iter()
            .filter_map(|(key, _)| key.as_ref().starts_with(path.as_ref()).then_some(key.dereference()))
            .collect();
        descendants
            .into_iter()
            .for_each(|descendant| self.remove_disk_file(descendant));
    }

    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        let source_id = self.get_disk_file_map().get(path.reference()).copied()?;
        self.get(source_id).wrap_some()
    }
}
```

The `collect` in `remove_disk_files_from_path` ends the shared borrow of `self` before `remove_disk_file` takes `&mut self`. `insert_disk_file` discards `insert`'s previous value; the source `set` already replaced the contents and the id is the same. `use std::collections::HashMap;` is removed and `use pico::{Database, SourceId, Storage, TrackedMap};` replaces the `pico` import.

Behavior deltas: `insert_disk_file` of an already-present path no longer re-invokes iterating memos (the entry is the same id). `remove_disk_file` of an absent path no longer re-invokes anything. `disk_file` on a miss now records a dependency on that path's slot.

Test call sites in the same file: `state.get_disk_file_map().untracked().0.len()` (lines 139, 177, 253) becomes `state.get_disk_file_map().iter().len()` (`hash_map::Iter` is `ExactSizeIterator`). `assert!(state.get_disk_file_map().untracked().0.is_empty())` (line 195) becomes `assert_eq!(state.get_disk_file_map().iter().len(), 0)`.

### `crates/isograph_compiler/src/lib.rs`

Before:

```rust
// from crates/isograph_compiler/src/lib.rs
pub use database::{DiskFile, DiskFileMap, IsographDb};
```

After:

```rust
// from crates/isograph_compiler/src/lib.rs
pub use database::{DiskFile, IsographDb};
```

`DiskFileMap` has no other users (`grep -rn DiskFileMap crates` finds only `isograph_compiler`).

### `crates/isograph_extract_typescript/src/lib.rs`

Before (lines 63-72):

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

`missing_then_present_extracts` (lines 556-567) before:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[test]
    fn missing_then_present_extracts() {
        let mut db = IsographDb::<TypeScriptHostLanguage>::default();
        let other = intern_path("src/b.ts");
        intern_file(&mut db, other, "export const x = 1;");
        let path = intern_path("src/a.ts");
        assert!(TypeScriptHostLanguage::extract_iso_literals(&db, path).is_none());
        intern_file(&mut db, path, "iso(`entrypoint Query.HomeRoute`)");
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }
```

After (the unrelated `src/b.ts` was there only so the counter existed before the miss):

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[test]
    fn missing_then_present_extracts() {
        let mut db = IsographDb::<TypeScriptHostLanguage>::default();
        let path = intern_path("src/a.ts");
        assert!(TypeScriptHostLanguage::extract_iso_literals(&db, path).is_none());
        intern_file(&mut db, path, "iso(`entrypoint Query.HomeRoute`)");
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }
```

Unchanged and still passing: `missing_disk_file_is_none` (line 549), `missing_disk_file_has_no_text_or_tree` (716), `missing_disk_file_has_no_literal_id` (922), `missing_disk_file_has_no_file_literals` (1235), `missing_disk_file_has_no_parsed_iso_literals_in_file` (1311), `missing_disk_file_has_no_text_through_last_iso_literal` (1620). Each reads a missing path once and asserts `None`; the read now creates a `None` slot, which none of them observes.

### `crates/isograph_cli`

`crates/isograph_cli/src/state.rs` test helper `contents` (around line 308) before:

```rust
// from crates/isograph_cli/src/state.rs
    fn contents(
        state: &IsographState<TypeScriptHostLanguage>,
        path: common_lang_types::RelativePathToSourceFile,
    ) -> Option<&str> {
        state
            .db
            .get_disk_file_map()
            .untracked()
            .0
            .get(&path)
            .map(|id| state.db.get(*id).contents.as_str())
    }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
    fn contents(
        state: &IsographState<TypeScriptHostLanguage>,
        path: common_lang_types::RelativePathToSourceFile,
    ) -> Option<&str> {
        state
            .db
            .disk_file(path)
            .map(|disk_file| disk_file.contents.as_str())
    }
```

`crates/isograph_cli/src/watch.rs` (around line 808): `assert!(h.state.db.get_disk_file_map().untracked().0.is_empty());` becomes `assert_eq!(h.state.db.get_disk_file_map().iter().len(), 0);`.

## Change 3: delete `#[tracked]`

- `crates/pico/src/view.rs`: delete `Counter`, `View`, `MutView`. Move `Projector` and `ProjectorMut` into `crates/pico/src/tracked_map.rs` and delete `view.rs`; `lib.rs` drops `mod view;` and `pub use view::*;`.
- `crates/pico_macros/src/lib.rs`: `#[proc_macro_derive(Db, attributes(tracked_map))]`.
- `crates/pico_macros/src/db_macro.rs`: delete `TrackedFieldKind` and the `Tracked` arm. The attribute check is `if !has_attr(&f.attrs, "tracked_map") { continue; }`, the visibility error names `#[tracked_map]` only, and the loop body is change 1's `TrackedMap` arm with `#projectors` inlined.
- `crates/pico/tests/tracking_field/correctness.rs` and `efficiency.rs` are deleted, and `mod tracking_field` leaves `tests.rs`. Their facts are covered by `tracked_map/entries.rs` (`iteration_reruns_on_new_key`) and `tracked_map/keyed.rs` (`unrelated_insert_does_not_rerun`). `correctness.rs` pins that iterating `untracked()` misses an insert; that API no longer exists.

After change 3:

```rust
// from crates/pico/src/tracked_map.rs
pub type Projector<Db, T> = for<'a> fn(&'a Db) -> &'a T;

pub type ProjectorMut<Db, T> = for<'a> fn(&'a mut Db) -> &'a mut T;
```

## Panics

Change 1 adds one `expect`, in `Storage::get_or_insert`: after a vacant key gets a node, `get_impl` of that key finds it. The invariant is that the code two lines above inserted a node at that key and nothing between them removes it. The type system cannot express that a `DashMap` lookup after an insert hits. It is the same family as the existing pico invariant panics (`database.rs:124-127`, `147-150`, `312-315`, `336-339`), all "indicative of a bug in Pico". `UnexpectedError` is an LSP type that pico does not depend on. `TrackedMapView::get` on a key whose slot is a removed source cannot occur: slot keys are crate-private, and if a user forged a `SourceId` from the key and `remove`d it, `remove_source` drops the key from `source_node_key_to_index` (`database.rs:363`), so the next `get` takes the vacant arm and re-creates the slot.

## Alternatives

### Absent reads at the source layer

`get_impl` registers `NodeKind::Source(key)` before its `?`. A vacant `set_source` advances the epoch. `remove_source` keeps the key and replaces the slot with a tombstone holding the removal epoch, so `source_nodes` becomes `BoxcarVec<SourceSlot>`:

```rust
// from crates/pico/src/source.rs
pub(crate) enum SourceSlot {
    Present(SourceNode),
    Removed(RemovedSourceNode),
}

pub(crate) struct RemovedSourceNode(pub Epoch);
```

`source_node_changed_since` is `Present` `time_updated > since`, `Removed` epoch `> since`, never-set `false`. A keyed lookup would go by the source key directly, which needs a new `#[derive(Source)]` output that computes the `Key` from the `#[key]` field without a value, plus an `Option`-returning `get`. The tracked map would be needed only for iteration.

Not chosen. It changes semantics for every source, not only map lookups: every vacant `set` advances the global epoch, so a boot scan of N files is N bumps, and every bump defeats `node_verified_in_current_epoch` for every memo, which then walks its dependencies on the next call. Every `remove` leaves a tombstone. It reverses 21d671d7's vacant arm. It needs a second public API on `Source` for key derivation, and the iteration dependency still needs the counter fix.

### Entries node created at database construction

`#[derive(Db)]` does not generate `Default` (`IsographDb` writes its own, `database.rs:19-27`), and `TrackedMap::default()` has no `Storage` to write into. Creating the entries node at construction needs each database's constructor to call a pico function per field, which is the discipline that lsp-open-files.md open decision 2 option (a) relies on. Creating it on the first `iter()` has the same effect with no call site. Open decision 1.

## Docs this change amends

Proposed text only. These files are not edited by this doc.

AGENTS.md, replace the whole "## pico tracked maps" section body with:

```text
`#[tracked_map]` on a database field of type `TrackedMap<K, V>` (`disk_file_map`) gives `get_disk_file_map()` / `get_disk_file_map_mut()`. `handle` writes through `get_disk_file_map_mut()` with `insert` and `remove`.

`get(&key)` records a dependency on the value held by that key, present or absent. Inserting that key, replacing its value with an unequal one, or removing it re-invokes the reader. A change to another key does not. A miss is an ordinary dependency: a later insert of that key is seen. There is no tracked or untracked read.

`iter()` records a dependency on the entries. A memo that iterates (compile, every file) uses `iter()`. Inserting a new key, replacing a value with an unequal one, or removing a key re-invokes it. Inserting an equal value at a present key, or removing an absent key, re-invokes nothing.

A `SourceId` value is the same id after its source is re-`set`. Readers that `db.get` the id re-invoke through the source; the map entry is unchanged.

pico `tracked_map` tests pin this.
```

pico.md, replace "## Tracked maps" (lines 334-348) with:

```text
## Tracked maps

    struct IsographDb<THostLanguage: HostLanguage> {
        storage: Storage<Self>,
        #[tracked_map]
        disk_file_map: TrackedMap<RelativePathToSourceFile, SourceId<DiskFile>>,
        phantom_data: PhantomData<THostLanguage>,
    }

The map is which paths currently have a `DiskFile`. `db.set` of a `DiskFile` does not update the map. `handle` does both: intern the source, then `insert` into the map. Remove is the reverse.

`#[tracked_map]` on the field gives `get_disk_file_map()` / `get_disk_file_map_mut()`.

`get(&path)` records a dependency on the value held by `path`, an `Option<SourceId<DiskFile>>`. A hit and a miss are the same dependency. Inserting or removing that path re-invokes the reader; inserting or removing another path does not. `IsographDb::disk_file` is that lookup followed by `db.get`, and extract calls it.

`iter()` records a dependency on the entries. Inserting a new path or removing a path re-invokes every memo that iterated. A memo that walks every file (compile, "all client declarations") uses `iter()`.

`insert` of the same `SourceId` at a present path, and `remove` of an absent path, re-invoke nothing. Re-setting a `DiskFile` with new contents keeps its `SourceId`; memos that `db.get` it re-invoke, and the map entry does not change.

pico keeps one node per path ever read or inserted. Those nodes are sources and are not garbage collected.
```

## Upstream divergence

AGENTS.md says pico and pico_macros are kept unchanged from upstream isograph. `diff -r` of `crates/pico` and `crates/pico_macros` against `../isograph` today differs only in `crates/pico/tests/tracking_field/correctness.rs` (`.values()` for `.iter()`). This doc adds `crates/pico/src/tracked_map.rs`, `Storage::get_or_insert`, a `prelude` dependency, and `#[tracked_map]` (change 1), and deletes `View`, `MutView`, `Counter`, `#[tracked]`, and the `tracking_field` tests (change 3). The doc needs the user's approval for that divergence, and AGENTS.md's first paragraph changes to say pico and pico_macros are kept from upstream with `TrackedMap` added and `#[tracked]` removed.

## Follow-ups this unlocks

- lsp-open-files.md open decision 2 resolves to neither (a) nor (b): the never-written counter does not exist.
- lsp-open-files.md change 2 shrinks: `IsographDb::default` writes no counter, map writes bump only on a changed entry without call-site checks, and `disk_file` tracks its miss with no change to `disk_file`.
- `IsographDb.clients` in lsp-open-files.md can be a `#[tracked_map]` `TrackedMap<ClientId, V>`. A non-`SourceId` `V` is stored twice (field and slot) and compared with `==` on every write.
- split-pico.md: `TrackedMap` is engine (`pico`); `#[tracked_map]` is in whichever crate holds `#[derive(Db)]`.
- An upstream isograph PR with the same three changes.

## Open decisions

1. Entries node creation. (a) On the first `iter()` or first changed write, as this doc specifies. (b) At database construction, with each database constructor calling a pico function per `#[tracked_map]` field (see Alternatives).
2. Test counters. The new pico tests count memo-body executions with a `static AtomicUsize` per test, the existing pico test pattern (`crates/pico/tests/basic.rs:6`). A memo body is a `fn` with no state of its own, so the count has to live outside it; a per-test static avoids the `LazyLock<Mutex<()>>` serialization lock that `basic_multi_function_chain.rs:67` uses. (a) Approve the statics. (b) Name another counting mechanism.
3. Postfix in pico. The pending-doc postfix lint (`crates/prelude/src/postfix_constructors.rs:49-58`) rejects `Some(`, `Box::new`, and `.into()` in this doc's snippets, and pico's own sources are outside `RUST_ROOTS` (`postfix_constructors.rs:17-32`). (a) pico depends on `prelude` and the new code uses `Postfix`, as written above. (b) pico takes no `prelude` dependency; the new code uses `Some(` / `Box::new` like the rest of pico, and the snippets are rewritten as `// external` or the lint gains a pico exemption.
