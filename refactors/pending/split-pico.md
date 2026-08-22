# Split pico by capability

isograph crates all depend on `pico`. A crate that stores an `Index<FileContent>` or impls `Singleton` pulls dashmap, lru, boxcar, tracing, bincode, and `#[memo]`. A crate that writes `#[memo]` can `#[derive(Source)]` and intern sources. Those are different jobs.

Three capabilities:

- The type: name `SourceId`, `Index`, `Key`, `Source`, `Singleton` in a struct, an impl, or a signature.
- Memoize: write `#[memo]`, call `db.get`, look up a `MemoRef`, intern a param.
- Sources: `#[derive(Source)]`, `#[derive(Singleton)]`, `#[derive(Db)]`, `db.set`, `db.remove`.

Not every crate that does one does the other two.

Four crates. The engine is shared by memoize and sources, so it is a crate of its own.

```
pico_types          identifiers and the Source / Singleton traits
pico                the engine: Database, Storage, execute, intern, get, set, MemoRef
pico_macros         #[memo]
pico_source         #[derive(Source)], #[derive(Singleton)], #[derive(Db)]
```

`pico` re-exports `pico_types`. A crate that depends on `pico` writes `use pico::SourceId` as today. A crate that only names the identifiers depends on `pico_types` and does not take the engine.

`Database` stays one trait, `get` and `set` both on it. A `#[memo]` body already takes `&IsographState` and cannot call `set`. The crate split is what stops a memo crate from defining a source type or a database.

## What the user does

No user-facing change. Tests of pico keep the same assertions. `common_lang_types` still exposes `Index` and `CurrentWorkingDirectory` as a singleton source.

## Who depends on what

isograph is the map. i2's graph after extract-iso-literals-from-file.md and the later seam crates is the same shape.

`pico_types` only:

- `common_lang_types`: `Index<FileContent>`, `CurrentWorkingDirectory` impls `Source` and `Singleton` by hand.

`pico_types` and `pico_source`, not `pico`:

- `isograph_config` when `CompilerConfig` is a singleton. i2's config crate does not yet. The derive expands to `impl pico_types::Singleton`, so the crate does not construct `Storage` or call `set`.

`pico` and `pico_macros`, not `pico_source`:

- `isograph_extract_typescript`: `#[memo] fn extract_iso_literals`. Reads a `DiskFile` with `db.get`. Does not define a source type.
- `isograph_lsp`: `#[memo]` hover, tokens, goto. Calls `insert_open_file` on `IsographState` (a method the compiler crate owns). Does not `#[derive(Source)]`.
- `isograph_protocol_graphql`: `#[memo]` parse of the schema. Takes `SourceId<SchemaSource>`. Does not intern that source.

`pico` only (no macros):

- generation (`artifact_content` in isograph, `isograph_generate_typescript` in i2): names `MemoRef`, calls `lookup`. Does not write `#[memo]`. Does not intern sources.

`pico`, `pico_macros`, and `pico_source`:

- `isograph_compiler`: owns `IsographState`, `DiskFile`, `#[derive(Db)]`, `#[derive(Source)]`, and compiler memos.
- pico's own tests.

Nothing:

- `isograph_parser`
- `isograph_cli` production code after extract-iso-literals-from-file.md (dev-dependency on `pico` for the `disk_file` helper)

`MemoRef` stays in `pico`. `lookup` reads storage. A crate that names `MemoRef` is using the engine.

`set` stays on `Database`. A crate that depends on `pico` to memoize can name `set`. It cannot `#[derive(Source)]` or `#[derive(Db)]` without `pico_source`. That is the line.

## Types

Most important first. Origin of each moved item is the pico file named in the snippet. Delta is the crate it lives in, and the `::pico::` / `::pico_types::` / `::pico_source::` path the macros emit.

### `pico_types`

```toml
# from crates/pico_types/Cargo.toml
[package]
name = "pico_types"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
intern = { path = "../../relay-crates/intern" }
u64_newtypes = { path = "../u64_newtypes" }
serde = { workspace = true }
serde_derive = { workspace = true }

[lints]
workspace = true
```

```rust
// from crates/pico_types/src/lib.rs
mod index;
mod intern;
mod source;

pub use index::*;
pub use intern::*;
pub use source::*;
```

`Index` is origin `crates/pico/src/index.rs`, verbatim:

```rust
// from crates/pico_types/src/index.rs
use std::{any::type_name, fmt, marker::PhantomData};

#[derive(Clone, Copy)]
pub struct Index<T> {
    pub idx: usize,
    phantom: PhantomData<T>,
}

impl<T> Index<T> {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            phantom: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Index<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Index<{}>[{:?}]", type_name::<T>(), self.idx)
    }
}
```

`Key`, `ParamId`, `HashId` are origin `crates/pico/src/intern.rs`. Delta: `use crate::SourceId` resolves in this crate.

```rust
// from crates/pico_types/src/intern.rs
use intern::InternSerdes;
use intern::{InternId, intern_struct};
use serde::{Deserialize, Serialize};
use u64_newtypes::u64_newtype;

use crate::SourceId;

u64_newtype!(HashKey);

intern_struct! {
    pub struct HashId = Intern<HashKey> {
      serdes("InternSerdes<HashId>");
      const EMPTY = HashKey(0);
    }
}

impl Default for HashId {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ParamId(HashId);

impl ParamId {
    pub fn inner(&self) -> HashId {
        self.0
    }
}

impl From<u64> for ParamId {
    fn from(value: u64) -> Self {
        Self(HashId::intern(HashKey(value)))
    }
}

impl<T> From<SourceId<T>> for ParamId {
    fn from(value: SourceId<T>) -> Self {
        Self(value.key.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Key(pub HashId);

impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Self(HashId::intern(HashKey(value)))
    }
}

impl From<HashId> for Key {
    fn from(value: HashId) -> Self {
        Self(value)
    }
}
```

`Source`, `Singleton`, `SourceId` are origin `crates/pico/src/source.rs` minus `SourceNode`. Delta: `HashId`, `ParamId`, `Key` come from this crate. `SourceNode` stays in `pico` (it holds `Epoch` and `Box<dyn DynEq>`). The moved file keeps origin `value.inner().into()`; the snippet below writes `.to()` because pending docs cannot contain `.into()`.

```rust
// from crates/pico_types/src/source.rs
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{HashId, Key, ParamId};

pub trait Source {
    fn get_key(&self) -> Key;
}

pub trait Singleton: Source {
    fn get_singleton_key() -> Key;
}

#[derive(Debug, PartialEq, Eq)]
pub struct SourceId<T> {
    pub key: Key,
    phantom: PhantomData<T>,
}

// We have to implement Clone and Copy ourselves. Otherwise,
// Clone and Copy would only be implemented if T: Clone or T: Copy,
// which is not correct! T only appears as part of PhantomData,
// so a SourceId is cloneable/copiable, regardless of what T is.
impl<T> Clone for SourceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SourceId<T> {}

impl<T> Hash for SourceId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T> Default for SourceId<T> {
    fn default() -> Self {
        Self {
            key: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<T> SourceId<T> {
    pub fn new(source: &impl Source) -> Self {
        Self {
            key: source.get_key(),
            phantom: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.key.0 == HashId::EMPTY
    }
}

impl<T> From<Key> for SourceId<T> {
    fn from(key: Key) -> Self {
        Self {
            key,
            phantom: PhantomData,
        }
    }
}

impl<T> From<ParamId> for SourceId<T> {
    fn from(value: ParamId) -> Self {
        Self {
            key: value.inner().to(),
            phantom: PhantomData,
        }
    }
}
```

Who calls it: `common_lang_types` (`Index`, `Source`, `Singleton`, `Key`). `pico` (re-export, `SourceNode` next to `SourceId`, `Database` methods). `pico_source` expansions. Every later crate that names a `SourceId`.

### `pico`

The engine crate. Origin `crates/pico`. Delta after change 1: depends on `pico_types`, re-exports it, `source.rs` is only `SourceNode`, `intern.rs` and `index.rs` are gone. `pico_macros` stays a `[dependencies]` entry until change 2.

```toml
# from crates/pico/Cargo.toml
[package]
name = "pico"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
intern = { path = "../../relay-crates/intern" }
pico_macros = { path = "../pico_macros" }
pico_types = { path = "../pico_types" }
u64_newtypes = { path = "../u64_newtypes" }
bincode = { workspace = true }
boxcar = { workspace = true }
dashmap = { workspace = true }
lru = { workspace = true }
serde = { workspace = true }
serde_derive = { workspace = true }
thiserror = { workspace = true }
tinyvec = { workspace = true, features = ["serde"] }
tracing = { workspace = true }

[lints]
workspace = true
```

`intern` stays: `DerivedNodeId` and `MemoRef` intern in this crate.

```rust
// from crates/pico/src/lib.rs
mod database;
mod dependency;
mod derived_node;
mod dyn_eq;
mod epoch;
mod execute_memoized_function;
mod garbage_collection;
pub mod macro_fns;
mod memo_ref;
mod raw_ptr;
mod retained_query;
mod source;
mod view;
mod with_serialize;

pub use pico_types::*;

pub use database::*;
pub use derived_node::*;
pub use dyn_eq::*;
pub use execute_memoized_function::*;
pub use memo_ref::*;
pub use raw_ptr::*;
pub use retained_query::*;
pub use source::*;
pub use view::*;
pub use with_serialize::*;
```

Origin: `crates/pico/src/lib.rs`. Delta: `mod index` and `mod intern` are gone; `pub use pico_types::*` replaces `pub use index::*` and `pub use intern::*`. `intern_value` and `intern_ref` stay in `database.rs` and still come through `pub use database::*`.

`Database` is origin `crates/pico/src/database.rs`. No delta to the trait.

```rust
// from crates/pico/src/database.rs
pub trait DatabaseDyn {
    fn get_storage_dyn(&self) -> &dyn StorageDyn;
}
pub trait Database: DatabaseDyn + Sized {
    fn get_storage(&self) -> &Storage<Self>;
    fn get<T: 'static>(&self, id: SourceId<T>) -> &T;
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

`SourceId`, `Source`, `Singleton` in those signatures are `pico_types` via `pub use pico_types::*`.

```rust
// from crates/pico/src/source.rs
use crate::{DynEq, Epoch};

#[derive(Debug)]
pub struct SourceNode {
    pub time_updated: Epoch,
    pub value: Box<dyn DynEq>,
}
```

Who calls it: every `#[memo]` expansion (`execute_memoized_function`, `Database`, `MemoRef`, `macro_fns`). `#[derive(Db)]` expansions. Tests. `IsographState` methods. Generation `lookup`.

`#[memo]` still emits `::pico::`. A memo crate depends on `pico` and `pico_macros`.

### `pico_macros`

`#[memo]` only. Origin `crates/pico_macros`. Delta: `db_macro.rs`, `source_macro.rs`, `singleton_macro.rs` are gone. `convert_case` is gone (only `db_macro` used it). Expansions still `::pico::`.

```toml
# from crates/pico_macros/Cargo.toml
[package]
name = "pico_macros"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
deluxe = { workspace = true }
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true }

[lib]
proc-macro = true

[lints]
workspace = true
```

```rust
// from crates/pico_macros/src/lib.rs
mod memo_macro;

extern crate proc_macro2;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn memo(args: TokenStream, input: TokenStream) -> TokenStream {
    memo_macro::memo_macro(args, input)
}
```

`memo_macro.rs` is verbatim. Who calls it: every `#[memo]` function. memo-on-trait-methods.md still rewrites this file and still emits `::pico::`.

### `pico_source`

Origin of the three files: `crates/pico_macros/src/{source,singleton,db}_macro.rs`. Delta: the `::pico::` paths on `Source`, `Singleton`, and `Key` become `::pico_types::`. The counter's `#[derive(Singleton)]` becomes `::pico_source::Singleton`. Engine paths stay `::pico::`.

```toml
# from crates/pico_source/Cargo.toml
[package]
name = "pico_source"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
convert_case = { workspace = true }
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true }

[lib]
proc-macro = true

[lints]
workspace = true
```

```rust
// from crates/pico_source/src/lib.rs
mod db_macro;
mod singleton_macro;
mod source_macro;

extern crate proc_macro2;

use proc_macro::TokenStream;

#[proc_macro_derive(Source, attributes(key))]
pub fn source(input: TokenStream) -> TokenStream {
    source_macro::source_macro(input)
}

#[proc_macro_derive(Singleton)]
pub fn singleton(input: TokenStream) -> TokenStream {
    singleton_macro::singleton_macro(input)
}

#[proc_macro_derive(Db, attributes(tracked))]
pub fn db(input: TokenStream) -> TokenStream {
    db_macro::db_macro(input)
}
```

```rust
// generated by crates/pico_source/src/source_macro.rs
impl ::pico_types::Source for DiskFile {
    fn get_key(&self) -> ::pico_types::Key {
        use ::std::hash::{Hash, Hasher, DefaultHasher};
        let mut s = DefaultHasher::new();
        ::core::any::TypeId::of::<DiskFile>().hash(&mut s);
        self.path.hash(&mut s);
        s.finish().to()
    }
}
```

Origin of that expansion: `crates/pico_macros/src/source_macro.rs` `quote!` block. Delta: `::pico::Source` is `::pico_types::Source`, `::pico::Key` is `::pico_types::Key`. The quote keeps origin `s.finish().into()`; the snippet below writes `.to()` because pending docs cannot contain `.into()`. The rest of `source_macro.rs` is verbatim.

```rust
// generated by crates/pico_source/src/singleton_macro.rs
impl ::pico_types::Singleton for CompilerConfig {
    fn get_singleton_key() -> ::pico_types::Key {
        use ::std::hash::{Hash, Hasher, DefaultHasher};
        let mut s = DefaultHasher::new();
        ::core::any::TypeId::of::<CompilerConfig>().hash(&mut s);
        s.finish().to()
    }
}

impl ::pico_types::Source for CompilerConfig {
    fn get_key(&self) -> ::pico_types::Key {
        <CompilerConfig as ::pico_types::Singleton>::get_singleton_key()
    }
}
```

Origin: `crates/pico_macros/src/singleton_macro.rs`. Delta: `::pico::` on `Singleton`, `Source`, `Key` is `::pico_types::`. The quote keeps origin `s.finish().into()`; the snippet writes `.to()` because pending docs cannot contain `.into()`.

`db_macro.rs` is origin `crates/pico_macros/src/db_macro.rs`. Delta: the tracked-field counter derive.

Before:

```rust
// from crates/pico_macros/src/db_macro.rs
            #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
            pub struct #counter_ident(u64);
            impl ::pico::Counter for #counter_ident {
```

After:

```rust
// from crates/pico_source/src/db_macro.rs
            #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_source::Singleton)]
            pub struct #counter_ident(u64);
            impl ::pico::Counter for #counter_ident {
```

The `impl Database` / `impl DatabaseDyn` quote is unchanged and still `::pico::`. `#[derive(Db)]` requires a dependency on `pico`.

Who calls it: `IsographState` (`Db`), `DiskFile` (`Source`), `CompilerConfig` (`Singleton`), pico tests (`Db`, `Source`, `Singleton`). `CurrentWorkingDirectory` does not: it impls the traits by hand.

## Change 1: `pico_types`

Add the crate. Move `index.rs`, `intern.rs`, and the `Source` / `Singleton` / `SourceId` half of `source.rs` as above. `pico` re-exports `pico_types`. `pico/src/intern.rs` and `pico/src/index.rs` are deleted. `pico/src/source.rs` is `SourceNode` only.

`common_lang_types` depends on `pico_types`, not `pico`.

```toml
# from crates/common_lang_types/Cargo.toml
intern = { path = "../../relay-crates/intern" }
span = { path = "../span" }
string_key_newtype = { path = "../string_key_newtype" }
pico_types = { path = "../pico_types" }
prelude = { path = "../prelude" }
```

Drop `pico = { path = "../pico" }`.

```rust
// from crates/common_lang_types/src/file_system_operation.rs
use std::path::PathBuf;

use crate::FileContent;

use pico_types::Index;
```

```rust
// from crates/common_lang_types/src/string_key_types.rs
use pico_types::{Key, Singleton, Source};
```

The `CurrentWorkingDirectory` impls are verbatim. `s.finish().to()` still needs `prelude::Postfix`.

`isograph_cli` still `use pico::{Database, SourceId, Storage}` and `use pico_macros::{Db, Source}`. Those names resolve through the re-export. No CLI edit in this change.

Pico modules that path through `crate::index`, `crate::intern`, or `crate::source::{Source, SourceId}` use the re-export instead. Origin of each `use` is the current file. Delta: `index::Index` is `Index`, `intern::{Key, ParamId}` is `{Key, ParamId}`, `intern::Key` is `Key`, `source::{Source, SourceId, SourceNode}` is `{Source, SourceId, SourceNode}` (`SourceNode` is still `crate::source::SourceNode`; pulling it from the crate root is the same after `pub use source::*`).

```rust
// from crates/pico/src/database.rs
use crate::{
    InnerFn, MemoRef, MemoRefKind, RawPtr, Singleton, Source, SourceId, SourceNode,
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    Index, Key, ParamId,
    macro_fns::{hash, init_param_vec},
};
```

```rust
// from crates/pico/src/derived_node.rs
use crate::{
    Database, Index, Key, ParamId,
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::Epoch,
};
```

```rust
// from crates/pico/src/macro_fns.rs
use crate::{Database, Index, ParamId};
```

```rust
// from crates/pico/src/dependency.rs
use crate::{derived_node::DerivedNodeId, epoch::Epoch, Key};
```

```rust
// from crates/pico/src/execute_memoized_function.rs
use crate::{
    Database, InnerFn, Key,
    dependency::{NodeKind, TrackedDependencies},
    derived_node::{DerivedNode, DerivedNodeId},
    dyn_eq::DynEq,
    epoch::Epoch,
};
```

```rust
// from crates/pico/src/garbage_collection.rs
use crate::{
    Database, DerivedNode, DerivedNodeId, DerivedNodeRevision, Index, InternalStorage, ParamId,
    dependency::{Dependency, NodeKind},
};
```

### Tests

Existing pico tests. They `use pico::{Database, SourceId, Storage}` and keep passing because of the re-export.

`FileSystemOperation::WriteFile` still holds `Index<FileContent>`. No new assertion: the type is the same, the crate is not.

`CurrentWorkingDirectory::get_singleton_key()` is the hash of `TypeId<CurrentWorkingDirectory>`. No production function only tests would call.

## Change 2: `pico_source`, `pico_macros` is `#[memo]` only

Add `pico_source`. Move the three derives. Slim `pico_macros` to `#[memo]`. `pico`'s `pico_macros` `[dependencies]` entry becomes a `[dev-dependencies]` pair:

```toml
# from crates/pico/Cargo.toml
[dependencies]
intern = { path = "../../relay-crates/intern" }
pico_types = { path = "../pico_types" }
u64_newtypes = { path = "../u64_newtypes" }
bincode = { workspace = true }
boxcar = { workspace = true }
dashmap = { workspace = true }
lru = { workspace = true }
serde = { workspace = true }
serde_derive = { workspace = true }
thiserror = { workspace = true }
tinyvec = { workspace = true, features = ["serde"] }
tracing = { workspace = true }

[dev-dependencies]
pico_macros = { path = "../pico_macros" }
pico_source = { path = "../pico_source" }
```

The library crate does not use the macros. Tests do.

Pico tests that currently write `use pico_macros::{Db, Source, memo}` write:

```rust
// from crates/pico/tests/basic.rs
use pico::{Database, SourceId, Storage};
use pico_macros::memo;
use pico_source::{Db, Source};
```

Same for every test file. `Singleton` comes from `pico_source`. Tests that only used `pico_macros::Db` and `pico_macros::memo` take `pico_source::Db` and `pico_macros::memo`.

`isograph_cli` production code today:

```rust
// from crates/isograph_cli/src/state.rs
use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};
```

After this change:

```rust
// from crates/isograph_cli/src/state.rs
use pico::{Database, SourceId, Storage};
use pico_source::{Db, Source};
```

```toml
# from crates/isograph_cli/Cargo.toml
pico = { path = "../pico" }
pico_source = { path = "../pico_source" }
```

Drop `pico_macros` from CLI `[dependencies]`. CLI does not write `#[memo]`. extract-iso-literals-from-file.md then drops `pico` and `pico_source` from production deps and keeps `pico` as a dev-dependency. That edit belongs to that doc.

### Tests

The same pico tests as today, with the import split above. Counters, reuse, GC, intern, tracked maps: same assertions.

A test crate that writes `#[derive(Source)]` without depending on `pico_source` does not compile. That is the Cargo.toml of `isograph_extract_typescript` once extract-iso-literals-from-file.md lands (`pico` + `pico_macros`, no `pico_source`). Do not add a production function only the tests call.

## Change 3: docs that name the crates

```text
# from Agents.md
From upstream isograph we keep pico, pico_types, pico_macros, pico_source, the swc plugin and its dependency chain
```

Drop `(unchanged)` on pico. The split is i2's.

`docs-website/docs/design-docs/pico.md`, after the opening paragraph, a crates section:

```text
# from docs-website/docs/design-docs/pico.md
pico_types is SourceId, Index, Key, Source, Singleton. pico is the engine (Database, Storage, memos, intern, set). pico_macros is #[memo]. pico_source is #[derive(Source)], #[derive(Singleton)], #[derive(Db)]. A crate that only names the identifiers depends on pico_types. A crate that writes #[memo] depends on pico and pico_macros. A crate that defines a source or a database depends on pico_source.
```

Pending docs that write a Cargo.toml for pico:

- extract-iso-literals-from-file.md Change 2: `isograph_compiler` depends on `pico`, `pico_macros`, and `pico_source`. Change 3: `isograph_extract_typescript` depends on `pico` and `pico_macros`, not `pico_source`. CLI production code depends on none of them.
- memo-on-trait-methods.md: `pico_macros` still owns `memo_macro`. Tests `use pico_macros::memo` and `use pico_source::{Db, Source}`.
- file-semantic-tokens.md, memoized-parse-iso-literal.md: `use pico_macros::memo` as they already write. The compiler crate already has `pico_source` from extract Change 2.

Those docs are not rewritten here. They pick up the crate names when work on them starts.

## Call sites after this lands

```
common_lang_types          pico_types
isograph_cli               pico, pico_source          (until extract-iso-literals-from-file.md)
isograph_compiler          pico, pico_macros, pico_source   (extract-iso-literals-from-file.md Change 2)
isograph_extract_typescript pico, pico_macros         (extract-iso-literals-from-file.md Change 3)
isograph_lsp               pico, pico_macros
isograph_protocol_graphql  pico, pico_macros
isograph_generate_typescript pico
isograph_config            pico_types, pico_source    (when CompilerConfig is a singleton)
isograph_parser            (none)
```
