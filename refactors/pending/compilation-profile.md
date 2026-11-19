# CompilationProfile

The compiler, the LSP, and the daemon are generic over one type parameter, `TProfile: CompilationProfile`. A profile names a `HostLanguage` and a `NetworkProtocol` as associated types. The binary defines the profile. No behavior changes.

Origin: isograph `crates/isograph_schema/src/compilation_profile.rs` `CompilationProfile` and `crates/isograph_schema/src/network_protocol.rs` `NetworkProtocol`. Delta: no derives as supertraits (`Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default`), because no generic code reads them. `CompilationProfile` has `HostLanguage` in place of `TargetPlatform` and no methods. `NetworkProtocol` has no associated types and no methods. The type parameter is `TProfile`, not upstream's `TCompilationProfile`.

## Types

```rust
// from crates/isograph_compiler/src/host_language.rs
/// One choice of host language and network protocol. A binary names one.
pub trait CompilationProfile: 'static {
    type HostLanguage: HostLanguage;
    type NetworkProtocol: NetworkProtocol;
}

/// The way the compiler communicates with the upstream.
pub trait NetworkProtocol: 'static {}
```

`CompilationProfile` sits next to `HostLanguage` and is re-exported by `pub use host_language::*`. The only bound the existing generic code needs is `'static` (pico `Storage<Self>`, `PhantomData` in `IsographDb`). `HostLanguage` keeps `Send + Sync + Sized + 'static`. `NetworkProtocol` gets methods with the schema parser work.

```rust
// from crates/graphql_network_protocol/src/lib.rs
use isograph_compiler::NetworkProtocol;

pub struct GraphQLNetworkProtocol;

impl NetworkProtocol for GraphQLNetworkProtocol {}
```

Crate name and struct name are upstream's (`isograph/crates/graphql_network_protocol`, `GraphQLNetworkProtocol`). Delta: unit struct, not `{}` with derives.

```rust
// from crates/ts_graphql_react_isograph_cli/src/main.rs
//! The isograph binary: TypeScript host, GraphQL protocol, React artifacts.

use std::process::ExitCode;

use graphql_network_protocol::GraphQLNetworkProtocol;
use isograph_compiler::CompilationProfile;
use isograph_extract_typescript::TypeScriptHostLanguage;

struct TypeScriptGraphQL;

impl CompilationProfile for TypeScriptGraphQL {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = GraphQLNetworkProtocol;
}

fn main() -> ExitCode {
    isograph_cli::run::<TypeScriptGraphQL>()
}
```

Before:

```rust
// from crates/ts_graphql_react_isograph_cli/src/main.rs
use isograph_extract_typescript::TypeScriptHostLanguage;

fn main() -> ExitCode {
    isograph_cli::run::<TypeScriptHostLanguage>()
}
```

`TypeScriptGraphQL` is private: only `main` reads it. No other item in the workspace has that name.

## Crate graph

```text
ts_graphql_react_isograph_cli
  -> isograph_cli -> isograph_lsp -> isograph_compiler
  -> isograph_extract_typescript -> isograph_compiler
  -> graphql_network_protocol -> isograph_compiler   (new)
  -> isograph_compiler                               (new direct dependency, for the impl)
```

`isograph_compiler`, `isograph_lsp`, `isograph_cli`, and `isograph_extract_typescript` do not depend on `graphql_network_protocol`, including as a dev-dependency.

```toml
# from crates/graphql_network_protocol/Cargo.toml
[package]
name = "graphql_network_protocol"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
isograph_compiler = { path = "../isograph_compiler" }

[lints]
workspace = true
```

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml
[dependencies]
graphql_network_protocol = { path = "../graphql_network_protocol" }
isograph_cli = { path = "../isograph_cli" }
isograph_compiler = { path = "../isograph_compiler" }
isograph_extract_typescript = { path = "../isograph_extract_typescript" }
```

## Per-type parameter decisions

A type takes the profile when it is the database or holds the database, or names a type that does. A type that reads only host-language associated types keeps taking the host language.

- `IsographDb<TProfile>`: profile. It is the per-binary pico database, and `phantom_data: PhantomData<TProfile>` makes it a different database type per profile, so memo slots of two profiles do not collide.
- `IsographState<TProfile>`: profile. It holds `db: IsographDb<TProfile>`.
- `Cli<TProfile>`, `CliVerb<TProfile>`, `Isograph<TProfile>` (the freddie `App`) in `isograph_cli/src/lib.rs`: profile. They start the daemon that owns `IsographState<TProfile>`.
- `IsoLiteralExtraction<THostLanguage>`: host language. It reads only `THostLanguage::LiteralContext`.
- `IsoLiteralError<THostLanguage>`: host language. It reads only `THostLanguage::Error`.
- `HostLanguage::extract_iso_literals_from_text -> Vec<IsoLiteralExtraction<Self>>`: unchanged.
- `isograph_extract_typescript::FileLiteral<'a>`: unchanged (concrete `IsoLiteralExtraction<TypeScriptHostLanguage>`).
- `isograph_extract_typescript::host_errors_for_extraction`: unchanged (takes no database).

## Functions

Every `THostLanguage: HostLanguage` parameter in `isograph_compiler`, `isograph_lsp`, and `isograph_cli` becomes `TProfile: CompilationProfile`. A call of a host-language associated function becomes `TProfile::HostLanguage::f`, with `HostLanguage` imported for method resolution. A return type that named `IsoLiteralExtraction<THostLanguage>` names `IsoLiteralExtraction<TProfile::HostLanguage>`. `IsographDb<impl HostLanguage>` becomes `IsographDb<impl CompilationProfile>`.

Memo, before:

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn extract_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<THostLanguage>>> {
    let contents = file_contents(db, client, path).as_ref()?;
    THostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}
```

After:

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn extract_iso_literals<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<TProfile::HostLanguage>>> {
    let contents = file_contents(db, client, path).as_ref()?;
    TProfile::HostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}
```

`#[memo]` accepts the generic `TProfile` and the projection `TProfile::HostLanguage` in the return type. The same rewrite applies to `file_contents`, `parsed_iso_literal`, `parsed_iso_literals_in_file`, `iso_literal_parse_errors_in_file`, `text_through_last_iso_literal`, `iso_literal_extraction`, `parsed_iso_literal_by_id`, and the `IsographDb` inherent impl.

Database, before and after:

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<TProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<TProfile>,
}
```

was `IsographDb<THostLanguage: HostLanguage>` with `PhantomData<THostLanguage>`.

State and `handle`:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) source_globs: crate::watch::SourceGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
}

pub fn handle<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
```

Host-language call in `isograph_cli`:

```rust
// from crates/isograph_cli/src/watch.rs
    match TProfile::HostLanguage::should_skip_source_file(relative.reference()) {
```

was `THostLanguage::should_skip_source_file(...)`. Same in `diagnostics.rs`.

Entry point:

```rust
// from crates/isograph_cli/src/lib.rs
pub fn run<TProfile: CompilationProfile>() -> ExitCode {
```

was `run<THostLanguage: HostLanguage>()`.

`file_literals` in `isograph_extract_typescript` took the concrete `IsographDb<TypeScriptHostLanguage>`. That type no longer exists. It takes any profile whose host language is TypeScript:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
pub fn file_literals<'a, TProfile: CompilationProfile<HostLanguage = TypeScriptHostLanguage>>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<FileLiteral<'a>>> {
```

## Test profiles

Each crate whose tests build an `IsographDb` defines its own profile under `#[cfg(test)]`, with a `TestNetworkProtocol`. A `#[cfg(test)]` item is not visible to another crate, and none of these crates may depend on `graphql_network_protocol`.

`isograph_compiler` `database.rs` tests, beside `TestHostLanguage`:

```rust
// from crates/isograph_compiler/src/database.rs
    struct TestNetworkProtocol;

    impl NetworkProtocol for TestNetworkProtocol {}

    struct TestProfile;

    impl CompilationProfile for TestProfile {
        type HostLanguage = TestHostLanguage;
        type NetworkProtocol = TestNetworkProtocol;
    }
```

`IsographDb::<TestHostLanguage>` becomes `IsographDb::<TestProfile>`.

`isograph_extract_typescript` second test module (the one that builds a database): the same two items with `type HostLanguage = TypeScriptHostLanguage;`. `IsographDb::<TypeScriptHostLanguage>` and `IsographDb<TypeScriptHostLanguage>` become `TestProfile`. `IsoLiteralExtraction<TypeScriptHostLanguage>` and `TypeScriptHostLanguage::should_skip_source_file` stay.

`isograph_lsp` and `isograph_cli`: one module per crate, declared `#[cfg(test)] mod test_profile;` in `lib.rs`, used by every test module in that crate.

```rust
// from crates/isograph_lsp/src/test_profile.rs
use isograph_compiler::{CompilationProfile, NetworkProtocol};
use isograph_extract_typescript::TypeScriptHostLanguage;

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
```

`crates/isograph_cli/src/test_profile.rs` is the same file. In tests, `use isograph_extract_typescript::TypeScriptHostLanguage;` becomes `use crate::test_profile::TestProfile;` and `TypeScriptHostLanguage` becomes `TestProfile` (`IsographDb::<_>`, `IsographState<_>`, `lsp_semantic_tokens_for_file::<_>`).

`isograph_cli` `watch.rs` tests keep `InHost` and add:

```rust
// from crates/isograph_cli/src/watch.rs
    struct InHostProfile;

    impl CompilationProfile for InHostProfile {
        type HostLanguage = InHost;
        type NetworkProtocol = TestNetworkProtocol;
    }
```

with `use crate::test_profile::TestNetworkProtocol;`. `IsographState<InHost>`, `IsographDb::<InHost>`, `apply::<InHost>`, and `start::<InHost>` take `InHostProfile`.

## Docs

- `AGENTS.md` first paragraph lists `crates/graphql_network_protocol`.
- `docs-website/docs/design-docs/pico.md` and `event-model.md`: `THostLanguage: HostLanguage` becomes `TProfile: CompilationProfile`, `IsographDb<THostLanguage>` becomes `IsographDb<TProfile>`, and host calls read `TProfile::HostLanguage::`. `event-model.md` states that a `CompilationProfile` names the `HostLanguage` and the `NetworkProtocol`.

## End-user experience

None. `isograph` behaves identically. `cargo build --workspace --all-targets`, `cargo test --workspace`, and clippy pass.
