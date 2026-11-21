# Scalar entities and the selection shape rule

Status: parked note. Not planned and not under discussion for implementation. This doc records ideas from a discussion so they are not lost. Nothing here is a decision.

## Selection shape rule

A selection without a selection set returns the field's value at its own type. A selection with a selection set returns an object of the selected fields.

- `name` returns the string.
- `best_friend` returns `{}`, or `null` when the field is null. The link never reaches user code.
- `best_friend` and `best_friend {}` are therefore the same selection.
- `name {}` returns `{}`, or `null` when the field is null.
- `best_friends` returns `[{}, null, ...]`, one element per list item.

## Scalars are entities

In the schema, every scalar is an entity. `String` is an entity, so `name { toUpperCase }` selects a client field `toUpperCase` on `String`. The resolver of `toUpperCase` receives the raw string.

## User-defined entities

A source file could declare an entity whose value is a TypeScript type. One candidate syntax is `iso<{ foo: string }>(\`entity FooRef\`)`, which declares the entity `FooRef` with value type `{ foo: string }`. Then `field FooRef.Bar { __self }` receives `{ foo: string }` as `__self`. The syntax is undecided.

## Runtime consequences

These use the terms of `refactors/pending/runtime/pico-runtime.md`.

Every case of the shape rule reads the field's source node. A null to non-null transition, a non-null to null transition, and a link move each mark the readers of that source node.

Under bare `best_friend`, the reader walk creates a fresh `{}` on each read. Two such objects compare equal under the structural rule (`resolverResultsEqual`), so a link move under bare `best_friend` cuts off at the reader and its dependents are not recomputed.

A client field on a scalar needs a node identity. Two options came up:

- (a) A derived node in the record's container, keyed by the path (for example `name/toUpperCase`). The record owns the node, and the node depends on the field's source node. Deleting the record disposes it, and garbage collection treats it like any other derived node the record owns.
- (b) A derived node keyed by the scalar value, shared across records. Every distinct value creates a node, so the set is unbounded, and no record owns the node, so garbage collection has no owner to collect it with.

Option (a) is held in the discussion. It is not decided.

## Query text

GraphQL requires a selection set on a field whose type is an object, interface, or union. Bare `best_friend` in an Isograph literal therefore still requests a field in the generated query (for example `id` or `__typename`), so that the response distinguishes null from non-null and the normalizer can store the link.

## Upstream behavior

Upstream isograph rejects both shapes this rule accepts. The upstream parser classifies a selection by the presence of a selection set: with one it is `SelectionType::Object`, without one it is `SelectionType::Scalar` (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:450` to `:476`). Validation then rejects a scalar selection of a server field whose target is an object (`../isograph/crates/isograph_schema/src/validate_selection_sets.rs:140`), a scalar selection of a client pointer (`:157`), an object selection of a server field whose target is a scalar (`:293`), and an object selection of a client field (`:309`). A client field on a scalar parent is rejected with "Invalid parent type. `X` is a scalar." (`../isograph/crates/isograph_schema/src/process_client_field_declaration.rs:72`), and a client pointer to a scalar with "Invalid client pointer target type." (`:135`).

Adopting this rule deviates from upstream. The reason is that a selection's result type follows from the field's type alone, and scalars gain client fields.

## i2 today

i2's parser accepts both shapes. `Selection::selection_set` is `Option<WithSpan<SelectionSet>>` (`crates/isograph_parser/src/selections.rs:33`), set by `consume_selection_set` (`crates/isograph_parser/src/selections.rs:89`, called at `:130`) for any selection.

i2's validation rejects both, matching upstream. `target_errors` (`crates/isograph_compiler/src/validation.rs:559`) reports `ValidationError::SelectionSetOnLeaf` for a selection set on a leaf target (`:583`, message "`X` returns a leaf and takes no selection set." at `:40`) and `ValidationError::MissingSelectionSet` for a composite target without one (`:587`, message at `:42`). `parent_error` (`:314`) reports `InvalidEntityError::LeafParent` for a `field` declaration on a leaf parent (`:322`, message at `:75`), and `pointer_target_error` reports `InvalidEntityError::LeafTarget` for a pointer to a leaf (`:343`, message at `:85`).

## Everything is a link

This is a user idea, parked like the rest of this doc.

Every selection follows a link to an entity. A selection with no selection set calls the target entity's default reader. `String`'s default reader returns the raw string. An object entity's default reader returns `{}`. A user-defined entity's default reader returns its `__self` value. A selection with a selection set reads the selected fields on the target entity.

Consequences:

- The reader AST collapses its scalar and linked node kinds into one kind. That kind carries an optional selection set and the target entity's default reader.
- The engine is unchanged. Each selection is still one `readSource` of the field's source node.
- A scalar entity has no record. Following the link to a `String` takes the value, and the entity's identity is its value. Under that reading, value-keyed identity (option (b) above) is the natural identity of a client field on `String`. Path-keyed identity (option (a)) still works: the derived node attaches to the record and path that produced the value.
- The TypeScript type of a selection with no selection set is the target entity's self type.
- The query text is unchanged.

## Per-type imports in both ASTs

This is a user direction, parked like the rest of this doc.

The default reader lives in the reader AST. The compiler emits it per selection, and the runtime does not look it up in a registry. A selection with no selection set carries one of three default readers:

- `Value`: the selection returns the field's value.
- `EmptyObject`: the selection returns `{}`, or `null` when the field is null.
- `Reader`: the selection calls a reader the artifact imports, referenced through an import thunk.

The compiler emits the default reader instead of a runtime registry for three reasons. A read does no lookup. The environment has no registry, so there is no registration and no registration order. The static target of a selection always decides its default reader: an abstract target is an object entity, and a GraphQL union cannot mix scalars and objects.

The direction generalizes this. For every concrete type a reader AST or a normalization AST encounters, the generated artifact imports what that type needs: its default reader, a constructor or destructor, its TypeScript types, or anything else the type defines. The imports are hoisted to the top of the artifact, and the AST references each import by its local name.

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

### What this direction adds

- Imports keyed by concrete type, not only by field: each concrete type a reader AST or normalization AST encounters contributes the imports that type defines.
- The reader AST's default reader per selection (`Value`, `EmptyObject`, or `Reader` with an import thunk), where both upstream and i2 today have no default reader.
- Value imports referenced from the normalization AST (for example a type's constructor or destructor), where both upstream and i2 today emit only the type's name as a string.
