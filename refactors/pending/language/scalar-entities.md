# Scalar entities: parked ideas

Status: parked note. Not planned and not under discussion for implementation. Nothing here is a decision.

The selection shape rule, scalars as entities with client fields, everything being a link, a scalar value as its own record, the default reader of each target entity as a per-type module, the query text of a bare object selection, and the upstream and i2 rules that reject both shapes moved into plans: `refactors/pending/language/linked-only-selections.md` (compiler, validation, artifacts) and `refactors/pending/runtime/pico-runtime.md`, Selections in the Isograph layer (runtime). The path-keyed and value-keyed identities this note listed for a client field on a scalar are superseded there: the derived node lives in the scalar record's container. What stays parked is below.

## User-defined entities

A source file could declare an entity whose value is a TypeScript type. One candidate syntax is `iso<{ foo: string }>(\`entity FooRef\`)`, which declares the entity `FooRef` with value type `{ foo: string }`. Then `field FooRef.Bar { __self }` receives `{ foo: string }` as `__self`. Its default reader would be its declaration's (linked-only-selections.md, Default reader modules). The syntax is undecided.

## Per-type imports in both ASTs

This is a user direction, parked like the rest of this doc. The reader AST's part, a default reader per target entity as a hoisted per-type module, is planned in linked-only-selections.md. The rest is parked: for every concrete type a reader AST or a normalization AST encounters, the generated artifact imports what that type needs (a constructor or destructor, its TypeScript types, or anything else the type defines), hoisted to the top of the artifact and referenced by local name.

### What upstream hoists today

Upstream collects imports per field, never per type. `ReaderImports` is a set of (field, `ImportedFileCategory`) pairs, and the three categories are `ResolverReader`, `RefetchReader`, and `Entrypoint` (`../isograph/crates/artifact_content/src/import_statements.rs:7` to `:24`). `reader_imports_to_import_statement` prints them as default imports at the top of the artifact (`import_statements.rs:27`; called at `../isograph/crates/artifact_content/src/eager_reader_artifact.rs:89` and `../isograph/crates/artifact_content/src/refetch_reader_artifact.rs:77`).

The reader AST adds to that set at these sites in `../isograph/crates/artifact_content/src/reader_ast.rs`:

- A client field selection imports its resolver reader and references it as `readerArtifact` (`:473` to `:490`).
- An inline fragment and a client pointer import their resolver reader as the linked node's `condition` (`:282` to `:285`, `:301` to `:311`). A plain server linked field gets `condition: null` (`:290`).
- An imperatively loaded field imports its refetch reader (`:508` to `:535`).
- A `@loadable` selection imports its entrypoint (`:559` to `:564`), or with `lazyLoadArtifact` emits `loader: () => import("../../P/f/entrypoint")` with no hoisted import (`:595`).

The generated file `../isograph/demos/pet-demo/src/components/__isograph/Mutation/MutualBestFriendSetterOtherSide/resolver_reader.ts:4` hoists `import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';`, and `:49` references it as `readerArtifact: Pet__fullName__resolver_reader`.

Param types follow the same pattern: `ParamTypeImports` is a set of fields, printed as type imports of each field's `output_type` or `param_type` (`import_statements.rs:25`, `:46`, `:63`). The entrypoint artifact hoists its normalization AST, resolver reader, query text, and refetch query imports, or emits lazy `loader` thunks for the normalization AST and reader (`../isograph/crates/artifact_content/src/entrypoint_artifact.rs:433` to `:509`).

The normalization AST imports nothing. A linked field's concrete type is a string literal (`concreteType` at `../isograph/crates/artifact_content/src/normalization_ast_text.rs:68`; generated as `concreteType: "Pet"` at `../isograph/demos/pet-demo/src/components/__isograph/Mutation/SetTagline/normalization_ast.ts:22`). A scalar's TypeScript type is a fixed JavaScript type name (`string`, `boolean`, `number` at `../isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:35` to `:37`), printed inline with no import.

### What i2 hoists today

i2 hoists imports from the artifact value. `ArtifactValue` has `Import`, `ImportCallThunk`, and `NamedImport` variants (`crates/typescript_artifacts/src/module.rs:44` to `:57`). `imports_of` and `named_imports_of` collect every import in a value at any depth (`crates/typescript_artifacts/src/imports.rs:9`, `:28`). `module_of_artifact` (`crates/typescript_artifacts/src/module.rs:252`) and the reader artifact (`crates/typescript_artifacts/src/reader_artifact.rs:54` to `:63`) print them at the top of the module.

The reader AST imports per field. A `ReaderSelection::Resolver` node references its field's resolver reader through `ArtifactValue::Import` (`crates/typescript_artifacts/src/reader_artifact.rs:131` to `:140`), and the reader artifact imports the user's resolver through `ArtifactValue::NamedImport` (`reader_artifact.rs:37`). The generated file `demos/pet-demo/src/__isograph/Owner/OwnerBadge/resolver_reader.ts:5` hoists `import Owner__displayName__resolver_reader from './../displayName/resolver_reader';`, and `:17` references it. The entrypoint imports its normalization AST and query text and references its resolver reader through `ImportCallThunk` (`crates/typescript_artifacts/src/entrypoint.rs:69` to `:79`; generated at `demos/pet-demo/src/__isograph/Query/HomeRoute/entrypoint.ts:4` to `:6`, `:17`).

The normalization AST imports nothing. A record's entity is `{ kind: 'Known', entityName: '<name>' }`, a string (`crates/typescript_artifacts/src/normalization_ast.rs:115` to `:120`; generated at `demos/pet-demo/src/__isograph/Mutation/SetTagline/normalization_ast.ts:40` to `:41`).

The one per-type import i2 has is for a custom scalar's TypeScript type: `custom_scalar_imports` emits `import type { <name> as <scalar>__custom_scalar } from "<module>"` for each custom scalar a type file uses (`crates/typescript_artifacts/src/generate_types.rs:473` to `:490`). It is a type import in the type files, not a value import referenced from an AST.

### What this direction adds beyond the plan

- Imports keyed by concrete type in the normalization AST, not only in the reader AST.
- Value imports referenced from the normalization AST (for example a type's constructor or destructor), where both upstream and i2 today emit only the type's name as a string.
