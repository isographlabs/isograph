# Pluggable compiler: the seams

isograph is built, from day one, as library crates around a small set of seams, and the shipped `isograph` binary is one composition of them. Another composition — another input language, another wire format, another output language or framework — is a wrapper binary built on the same crates, not a fork.

This is the discussion half of the doc: it fixes which seams exist, what crosses each one, and how the pieces are packaged. The ordered changes land with the pipeline docs (extraction is scheduled with `resilient-parser.md`'s stage 3 doc; schema and generation get their own), each of which specifies its stage's types against the boundaries named here. Types marked (future) below get their real definitions in those docs; what this doc pins down is which side of a seam they live on.

## The axes, and the seams they map to

Five things vary; they map to three seams, because the last three are not independent in code.

1. The input language: which files are scanned, and how isograph literals are found in them. TypeScript/JavaScript first; a Rust or Python codebase carrying literals is another implementor. This is the extraction seam.
2. The query type: what a compiled entrypoint becomes on the wire — GraphQL text, a persisted document id, SQL, a tRPC call — and, on the other end, how the schema that selections validate against is described (GraphQL SDL today). Both directions belong to one implementor, because they must agree; this is the `NetworkProtocol` seam, keeping upstream's name.
3. The artifact language: the language of the generated files (TypeScript today).
4. The framework: the conventions wrapping a resolver into something usable — React's component-and-hooks shape today.
5. The artifact kinds: which files are generated per entrypoint and resolver (query text, param types, readers, entrypoints).

Axes 3–5 combine inside one generator: a reader artifact for React-in-TypeScript is not a TypeScript layer times a React layer times a reader layer, it is one piece of code generation whose language sets the syntax, whose framework sets the wrappers, and whose kinds are its output set. So the third seam is generation as a whole, with the framework entering as a parameter of a language's generator rather than as its own seam.

Each seam is a trait, which is the case AGENTS.md reserves traits for: a boundary whose second implementor is the point. The first implementors (TypeScript in, GraphQL over the wire, TypeScript-and-React out) are written against the seams, not fused into the core.

## What is fixed, not pluggable

- The literal grammar and `isograph_parser`. The language of the literal is the product; every profile parses the same literals into the same trees.
- The compiler's own model of a project between parsing and generation: the parsed literals, the validated selections. Protocol and generation plug into that model; they do not each get their own.
- pico sits above the seams: a seam implementor is a deterministic pure function of its inputs, so its results can live behind memoization. An implementor that reads the filesystem or the clock inside the seam breaks that; reading the world happens in the daemon's sources and effects (`event-model.md`), never inside a seam.
- The daemon and CLI library are profile-agnostic. `IsographRequest` and dispatch never name a profile; a binary gets the whole lifecycle, socket, and LSP surface by composing its profile into `isograph_cli::IsographCli::run`.

## The boundary types

### Extraction

The extraction seam is `HostLanguage` in extract-iso-literals.md, in `crates/isograph_compiler`. `extract_iso_literals` finds iso literals in a file and returns `WithErrors` (`item` / `errors`). `item` is `WithSpan<(&str, THostLanguage::LiteralContext)>`. The first implementor is `TypeScriptHostLanguage` in `isograph_extract_typescript`. File extensions for the watcher stay a later field on that trait.

### NetworkProtocol

```rust
/// The query-type seam: how the schema is described to the compiler, and what one entrypoint
/// becomes on the wire. One implementor per way of talking to a server.
pub trait NetworkProtocol {
    /// The parsed type system selections validate against.
    type Schema;

    /// (future: `SchemaSources` and `Diagnostic` are specified by the schema doc.)
    fn parse_schema(&self, sources: &SchemaSources) -> Result<Self::Schema, Vec<Diagnostic>>;

    /// What one entrypoint sends over the wire: GraphQL text, a persisted id, SQL, a tRPC
    /// path. (future: `CompiledEntrypoint` is specified by the generation doc.)
    fn request_body(&self, entrypoint: &CompiledEntrypoint<'_, Self::Schema>) -> String;
}
```

### Generation

```rust
/// A generated file. Producing these is the seam; writing them to disk is an effect the daemon
/// performs, so generation stays pure.
pub struct Artifact {
    pub path: PathBuf,
    pub contents: String,
}

/// The generation seam: everything a compiled project becomes on disk. One implementor per
/// artifact language. (future: `CompiledProject` is specified by the generation doc.)
pub trait GenerateArtifacts {
    fn generate(&self, project: &CompiledProject<'_>) -> Vec<Artifact>;
}

/// TypeScript generation, with the framework as data on the generator: the difference between
/// React output and another framework's is which wrappers surround the same readers, not a
/// different generator. (future: the fields of `FrameworkBindings` are specified by the
/// generation doc.)
pub struct TypeScriptGenerator {
    pub framework: FrameworkBindings,
}
```

## The profile

A profile is one choice on every axis, composed as a struct — upstream's `GraphQLAndJavascriptProfile` is the shape being kept, made explicit:

```rust
/// One composition of the seams. The pipeline is generic over this and nothing else.
pub struct Profile<TExtract, TProtocol, TGenerate> {
    pub extract: TExtract,
    pub protocol: TProtocol,
    pub generate: TGenerate,
}
```

Static generics, not trait objects: a binary compiles the profile it ships, monomorphized through the pipeline the way upstream threads its profile type parameter. The shipped binary's profile is `Profile { extract: TypeScriptHostLanguage, protocol: GraphQlProtocol, generate: TypeScriptGenerator { framework: react_bindings() } }`.

## Crates

```
isograph_parser
    ^
isograph_compiler
    ^
isograph_extract_typescript, isograph_protocol_graphql, isograph_generate_typescript
    ^
ts_graphql_react_isograph_cli

isograph_compiler
    ^
isograph_lsp, isograph_cli
    ^
ts_graphql_react_isograph_cli
```

`crates/isograph_compiler`: the seam traits, the boundary types, the fixed project model, and the pipeline. Depends on `isograph_parser`; contains no implementor. extract-iso-literals.md creates this crate with `HostLanguage` only.

`crates/isograph_extract_typescript`, `crates/isograph_protocol_graphql`, `crates/isograph_generate_typescript`: the first implementor of each seam, one crate each, none depending on another.

`crates/isograph_cli`: library. Bootstraps the daemon, lifecycle verbs, and LSP given a `Profile`. Depends on `isograph_compiler` and `isograph_lsp`. Does not depend on any implementor crate. Does not name TypeScript, GraphQL, or React.

```rust
// from crates/isograph_cli/src/lib.rs
use std::process::ExitCode;

use isograph_compiler::{GenerateArtifacts, HostLanguage, NetworkProtocol, Profile};

pub struct IsographCli<TExtract, TProtocol, TGenerate> {
    pub profile: Profile<TExtract, TProtocol, TGenerate>,
}

impl<TExtract, TProtocol, TGenerate> IsographCli<TExtract, TProtocol, TGenerate>
where
    TExtract: HostLanguage,
    TProtocol: NetworkProtocol,
    TGenerate: GenerateArtifacts,
{
    pub fn run(self) -> ExitCode {
        // clap, freddie lifecycle verbs, `isograph lsp` -> isograph_lsp::start(self.profile.extract)
    }
}
```

`crates/ts_graphql_react_isograph_cli`: the first-party binary. Consumer of `isograph_cli`. This is where TypeScript, GraphQL, and React start for the product: it constructs that profile and calls `run`. Binary name is `isograph`. The no-op split (library plus this binary, `run` taking no type params) is ts-graphql-react-isograph-cli.md. Both crates are root workspace members. Integration tests that drive `CARGO_BIN_EXE_isograph` live here.

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml
[package]
name = "ts_graphql_react_isograph_cli"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[[bin]]
name = "isograph"
path = "src/main.rs"

[dependencies]
isograph_cli = { path = "../isograph_cli" }
isograph_compiler = { path = "../isograph_compiler" }
isograph_extract_typescript = { path = "../isograph_extract_typescript" }
isograph_generate_typescript = { path = "../isograph_generate_typescript" }
isograph_protocol_graphql = { path = "../isograph_protocol_graphql" }

[lints]
workspace = true
```

```rust
// from crates/ts_graphql_react_isograph_cli/src/main.rs
use std::process::ExitCode;

use isograph_cli::IsographCli;
use isograph_compiler::Profile;
use isograph_extract_typescript::TypeScriptHostLanguage;
use isograph_generate_typescript::{TypeScriptGenerator, react_bindings};
use isograph_protocol_graphql::GraphQlProtocol;

fn main() -> ExitCode {
    IsographCli {
        profile: Profile {
            extract: TypeScriptHostLanguage,
            protocol: GraphQlProtocol,
            generate: TypeScriptGenerator {
                framework: react_bindings(),
            },
        },
    }
    .run()
}
```

The crate split is ts-graphql-react-isograph-cli.md. After that doc, `isograph_cli` is a `[lib]` and `ts_graphql_react_isograph_cli` has the bin. Both are root workspace members.

A wrapper outside this repo is the same shape as `ts_graphql_react_isograph_cli` with a different profile and a different `App::NAME`. Nothing in `isograph_compiler`, `isograph_cli`, or `isograph_lsp` knows whether it is running inside `ts_graphql_react_isograph_cli` or inside a wrapper.

lsp-semantic-tokens.md is the first doc that needs a binary to name `TypeScriptHostLanguage`. Until `NetworkProtocol` and `GenerateArtifacts` exist, `IsographCli` is generic over `THostLanguage` only:

```rust
// from crates/isograph_cli/src/lib.rs (lsp-semantic-tokens.md)
pub struct IsographCli<THostLanguage: HostLanguage> {
    pub host: THostLanguage,
}

impl<THostLanguage: HostLanguage> IsographCli<THostLanguage> {
    pub fn run(self) -> ExitCode {
        // `isograph lsp` -> isograph_lsp::start(self.host)
    }
}
```

```rust
// from crates/ts_graphql_react_isograph_cli/src/main.rs (lsp-semantic-tokens.md)
fn main() -> ExitCode {
    IsographCli {
        host: TypeScriptHostLanguage,
    }
    .run()
}
```

## Open questions

- Profile selection: one binary carries one profile, or a binary carries several and the config names one. One-binary-one-profile is the current position; a registry only earns its complexity if shipping multiple first-party profiles in the default binary becomes real.
- `App::NAME` is `"isograph"` on the `Isograph` impl in `isograph_cli`. A second in-process wrapper cannot share that daemon lock. The name becomes data the binary supplies when a second binary exists.
- `FrameworkBindings`: data on the generator (current position), a second trait, or a closed enum. Data keeps out-of-tree frameworks possible without a trait; the generation doc decides when the real fields exist.
- Artifact kinds: whether a generator's output set is fixed per generator or independently toggleable (a user who wants readers but not entrypoints). Currently fixed per generator.
- Whether `source_extensions` belongs on `ExtractLiterals` or file discovery moves wholly into the extraction seam (an input language where "which files" is not an extension check, like literals in markdown code fences, would force the latter).
