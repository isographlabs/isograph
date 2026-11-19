# Close the validation gaps from the parity inventory

This doc closes the eight validation gaps of `refactors/pending/upstream-parity.md` (Validation, and the `__typename`, identity, and list-argument bullets of Schema and GraphQL and of Language and parser):

1. Merge and generation errors reach only the daemon log (`crates/isograph_cli/src/compile.rs:84-99`).
2. Schema field arguments are not checked against the schema.
3. Argument values are not type checked: variable type mismatch, literal against type, `null` for a non-null type, missing and extra input object fields.
4. Two selections with one response name in one selection set are accepted.
5. `field P.f to T` never checks `T`.
6. Users cannot select `__typename`.
7. A type whose `id` is not `ID!` silently has no identity.
8. List argument values parse, and the merge rejects them ("List arguments are not supported in queries yet.").

Upstream is `/Users/rbalicki/code/isograph` at `9d72f6fc`; its paths are written `isograph/<path>:<line>`. Each check holds upstream's behavior and message unless a delta is stated with its reason.

## The rule that places each check

The core (`isograph_compiler`) knows iso literals, entities, selectables, wrappers, and the shapes `NetworkProtocol` reports (`EntityShape`, `SelectableShape`). It knows nothing of GraphQL. A check that needs a GraphQL fact (argument definitions, scalar and enum coercion, input object fields, `__typename`, `ID!`, list input coercion) is a `NetworkProtocol` method that the core calls and whose verdict the core reports. Every such method is a method on the existing trait; no trait is added. The core reports a protocol verdict as an associated error type it only displays.

- Core: response name uniqueness (gap 4), field parents and pointer targets (gap 5, through `EntityShape`), selection shape against the target's shape, unknown arguments of a `field` declaration's selection, generation errors (gap 1), the per-file gating, and the list value in the merge (gap 8).
- `graphql_network_protocol`: schema argument definitions and every value type check (gaps 2, 3), the `__typename` selectable (gap 6), the `id` check and schema parse errors (gap 7), and the printing of list values (gap 8).

## What the user sees

Every error below is an editor diagnostic on the offending span and a line of `project_errors` (the compile's `errors=N`, and `isograph errors` once compiler-errors.md changes 5 and 6 land).

```text
src/Detail.ts:4:7: `Query.pet` requires the argument `id`, and this selection does not pass it.
src/Detail.ts:4:11: `Query.pet` takes no argument `petId`.
src/Detail.ts:4:15: Mismatched type. Received $id, with type String!, expected ID!.
src/Detail.ts:5:22: Expected input of type Int, found a string literal.
src/Detail.ts:6:30: Expected non null input of type ID!, found null
src/Detail.ts:7:28: `SetPetTaglineParams` requires the field `tagline`, and this object does not have it.
src/Detail.ts:7:40: `SetPetTaglineParams` has no field `taglin`.
src/Detail.ts:8:7: A field with name or alias `name` has already been defined
src/Pet.ts:2:28: Entity `Pett` is not defined.
src/Pet.ts:2:28: Invalid client pointer target type. `String` is a leaf. You are attempting to define a pointer to it. In order to do so, the type must be an object, interface or union.
src/Home.ts:3:5: These fields inline each other: Query.a -> Query.b.
src/Home.ts:9:28: The entrypoint selects nothing the server returns.
schema/schema.graphql:12:7: The `id` field on `Pet` must have type `ID!`.
schema/schema.graphql:3:1: Expected a name, found '{'.
```

`__typename` is selectable on every object, interface, and union, with or without an alias. `tags(names: ["a", "b"])` prints `tags(names: ["a", "b"])` in the query.

The compile writes artifacts only when `project_errors` is empty (upstream: `validate_entire_schema` fails before generation, `isograph/crates/isograph_schema/src/validate.rs:32-81`). Before this doc, a project with validation errors still wrote artifacts when generation happened to succeed.

## Gating

A file's generation errors are computed only when the file has no parse, host, or validation error, and only errors located inside the declaration's own literal are reported there. A field `F` in a clean file that inlines a broken `G` in another file returns `G`'s error located in `G`'s file, which `F`'s file drops and `G`'s file reports (from `G`'s own memo when `G`'s file is clean, or as `G`'s validation error). Each error is therefore reported once, at its origin, and a validation error never also appears as the merge's version of the same mistake.

## Changes

Each change is one commit with its tests. Prefactors first.

### 1. The editor reads `compiler_errors_in_file`

`lsp_diagnostics_for_file` (`crates/isograph_lsp/src/file_diagnostics.rs`) reads `iso_literal_parse_errors_in_file` and `iso_literal_validation_errors_in_file` itself. After this change it reads `compiler_errors_in_file`, so every later `CompilerError` variant reaches the editor and the compile from one join point.

```rust
// from crates/isograph_lsp/src/file_diagnostics.rs
#[memo]
pub fn lsp_diagnostics_for_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::Diagnostic>> {
    let errors = compiler_errors_in_file(db, client, path).as_ref()?;
    if errors.is_empty() {
        return Vec::new().wrap_some();
    }
    let line_index = line_index(db, client, path).as_ref()?;
    errors
        .iter()
        .map(|error| diagnostic(line_index, error.location, error.item.to_string()))
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`diagnostic(line_index, span, message)` is the existing closure as a private function. Tests: the existing `file_diagnostics.rs` tests pass unchanged.

### 2. Host literal errors are compiler errors

`TypeScriptHostError` (`MissingParentheses`, `MissingExport`, `MissingAssociatedFunction`) exists in `crates/isograph_extract_typescript/src/lib.rs`, and `host_errors_for_extraction` computes it, but only `file_literals` calls it and nothing calls `file_literals`. `HostLanguage` gains the method, the core gains the memo, and `CompilerError` gains the variant.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    // existing items unchanged
    /// The errors the host language reports for one literal, in file-absolute spans.
    fn literal_errors(
        extraction: &IsoLiteralExtraction<Self>,
        parsed: &ParsedIsoLiteral,
    ) -> Vec<WithSpan<Self::Error>>;
}
```

```rust
// from crates/isograph_compiler/src/iso_literals.rs
/// Host language errors of every iso literal in `path` as `client` reads it.
#[memo]
pub fn iso_literal_host_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<<TProfile::HostLanguage as HostLanguage>::Error>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| TProfile::HostLanguage::literal_errors(extraction, parsed))
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`CompilerError<THostError, TNotFetchable>` gains `Host(THostError)`; `compiler_errors_in_file` chains parse, host, then validation errors. The TypeScript implementation is `host_errors_for_extraction` with one delta: `MissingExport` is located at the field's name (literal offset added), upstream's location (`isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:201-206`); the other two stay on the literal. `file_literals`, `FileLiteral`, and `IsoLiteralError` are deleted; nothing reads them. The test host languages return no errors.

Tests (`crates/isograph_extract_typescript`): the existing `host_errors_for_extraction` tests move to `literal_errors`; a missing export covers the field name. (`crates/isograph_cli/src/compile.rs`): `project_errors_include_a_missing_export` names `fullName`'s span.

### 3. `SelectionMapError` is always located

`SelectionMapError.location` is `Option<DefinitionLocation>`, `None` only for the failed lookup of the field itself. The two cases become two types.

```rust
// from crates/isograph_compiler/src/merged_selection.rs
/// Why a field's merged selection map could not be built.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FieldSelectionMapError {
    /// The lookup of the field itself failed: it has no one definition.
    #[error("{}", .0.0.message)]
    FieldLookup(Diagnostic),
    #[error("{0}")]
    Located(SelectionMapError),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind}")]
pub struct SelectionMapError {
    pub location: DefinitionLocation,
    pub kind: SelectionMapErrorKind,
}
```

`field_selection_map` and `source_field_declaration` return `FieldSelectionMapError`. In `client_field_contribution`, the inlined field's `FieldLookup(d)` becomes `Located` of `Lookup(d)` at the selection's name, so a parent never reports a child's lookup as its own. `EntrypointQueryError::SelectionMap` carries `FieldSelectionMapError`. Tests: the merged selection tests that match `location: Some(..)` match the plain location.

### 4. Schema file errors reach the compiler and the editor

This change is compiler-errors.md change 9, which it replaces.

`schema_file_paths` moves verbatim from `crates/graphql_network_protocol/src/schema_files.rs` to `crates/isograph_compiler/src/database.rs`, beside `source_file_paths`; it reads only `schema_path_map`. `IsographDb::editor_buffer_schema_paths_of_client` mirrors `editor_buffer_paths_of_client` over `schema_path_map`.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    // existing items unchanged
    /// An error the protocol reports for a schema file.
    type SchemaFileError: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;

    /// The errors of the schema file `path` as `client` reads it, in file-absolute spans;
    /// `None` when the file has no text.
    fn schema_file_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<WithSpan<Self::SchemaFileError>>>;
}
```

```rust
// from crates/isograph_compiler/src/compiler_errors.rs
#[memo]
pub fn schema_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ProfileSchemaFileError<TProfile>>>> {
    TProfile::NetworkProtocol::schema_file_errors(db, client, path)
}
```

The GraphQL protocol's error and memo:

```rust
// from crates/graphql_network_protocol/src/schema_files.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLSchemaFileError {
    #[error("{0}")]
    Parse(graphql_schema_parser::ParseError),
}

#[memo]
pub fn graphql_schema_file_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<GraphQLSchemaFileError>>>;
```

Each failed definition contributes its `DefinitionError.error` at that error's span. `project_errors` reports `compiler_errors_in_file` of each `source_file_paths` path and `schema_errors_in_file` of each `schema_file_paths` path, sorted as today; a path in both lists gets both lists of errors, each correct for its role. The editor publishes `lsp_diagnostics_for_schema_file(db, client, path)` for each schema path the client has a buffer of. Test protocols return `None`.

Tests: (`graphql_network_protocol`) a broken definition is one error on its span and a clean file has none; (`compile.rs`) `project_errors_include_a_schema_parse_error`, replacing `project_errors_skip_schema_files` with the fact that a `.graphql` file is never parsed as iso; (`diagnostics.rs`) a client with a broken schema buffer is published its diagnostic.

### 5. A protocol selectable may have no definition text

`ProtocolSelectable.definition` and `Selectable.definition` become `Option<DefinitionLocation>`. `None` is a selectable the protocol defines with no text (change 11's `__typename`); AGENTS.md forbids borrowing the parent's span. `source_field_declaration` takes the declaration's location from `source_selectable_definitions(db, client, entity, selectable)`, which is non-optional, instead of from the merged `Selectable`: exactly one entry is the location, any other count is `FieldSelectionMapError::FieldLookup`. A schema selectable has no declaration to inline, so `NotASourceField` moves from a located `SelectionMapErrorKind` of the field itself to `FieldSelectionMapError::NotASourceField(SelectableKey)`; a parent that inlines one reports it located at its selection, as `SelectionMapErrorKind::NotASourceField`. Goto definition and references return nothing for `None`. Tests: existing ones pass.

### 6. Generation errors are compiler errors (gap 1)

```rust
// from crates/isograph_compiler/src/generation_errors.rs
/// A query generation error of a declaration in a source file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QueryGenerationError {
    #[error("{0}")]
    SelectionMap(SelectionMapErrorKind),
    /// The lookup of the declared field failed after validation accepted it.
    #[error("{}", .0.0.message)]
    FieldLookup(Diagnostic),
    #[error("{0}")]
    TopLevel(TopLevelError),
    #[error("{0}")]
    IntegerOutOfRange(IntegerOutOfRange),
}

/// Generation errors of the declarations in `path`, file-absolute. Empty when the file has a
/// parse, host, or validation error.
#[memo]
pub fn iso_literal_generation_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<QueryGenerationError>>>;
```

The memo returns `Some(vec![])` when any of the three earlier lists is non-empty. Otherwise, per literal:

- `field E.f`: `field_selection_map(db, client, E, f)`. `Located(e)` is reported at `e.location.span` when `e.location.path == path` and the literal's span contains it; `FieldLookup(d)` is reported at the declaration's name; `NotASourceField` is not reached for a declaration (a source declaration beside a schema field is `DuplicateDefinition`).
- `entrypoint E.f`: `query_for_entrypoint`. `TopLevel(t)` and `IntegerOutOfRange(i)` are reported at the entrypoint's name: they are properties of the entrypoint's operation, which no selection span holds. `SelectionMap` is reported by the field's own declaration, and `Lookup` and `NotFetchable` by validation, so those arms report nothing.
- `entity E`: nothing.

This doc does not change `field_types` or its `GenerationError` (other work owns `artifact_types.rs`). Its selection shape errors become unreachable in a clean project because validation reports them first: `validate_selection` gains, for a selection of a protocol selectable or a client pointer, the target's shape check, with the merge's messages.

```rust
// from crates/isograph_compiler/src/validation.rs
pub enum ValidationError<TNotFetchable, TArgumentError> {
    // existing variants
    #[error("`{0}` returns a leaf and takes no selection set.")]
    SelectionSetOnLeaf(SelectableKey),
    #[error("`{0}` returns an entity with selectables and needs a selection set.")]
    MissingSelectionSet(SelectableKey),
    #[error("`{0}` is an input type, which cannot be selected.")]
    InputTarget(EntityName),
}
```

`CompilerError` gains `Generation(QueryGenerationError)`, chained last. `field_selection_map` reports `InlineCycle` only when the cycle contains the field itself; a field that reaches a cycle through another field returns that member's error, located at the member, so the cycle is reported once per member. `compile` writes artifacts only when `project_errors` is `Ok` and empty. A generation failure that still occurs logs `tracing::error!` with the error (a broken invariant, the `ListedPathWithoutText` precedent), except `TypeScriptGenerationError::SourceNotRelative`, which depends on the output folder and is logged with its field (open decision 1).

Tests (`graphql_network_protocol`): an inline cycle is one error per field at its name; a field inlining a broken field in another file has no error of its own; a field whose file has a validation error has no generation error; an entrypoint whose field selects only a client pointer is `TopLevel(Empty)` at the entrypoint's name; `pets { stats }` without a selection set is `MissingSelectionSet` from validation and no generation error. (`compile.rs`): a project with a validation error writes nothing, then writes once fixed; `project_errors` includes a generation error with its absolute range.

### 7. One response name per selection set (gap 4)

Upstream: "A field with name or alias `x` has already been defined" (`isograph/crates/isograph_schema/src/validate_selection_sets.rs:76-86`), at the selection. Here it is at the second selection's response name: its alias when it has one, otherwise its name.

```rust
// from crates/isograph_compiler/src/validation.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("A field with name or alias `{0}` has already been defined")]
pub struct DuplicateResponseName(pub SelectionName);
```

`validate_selection_set` reports one per selection whose response name an earlier sibling has. Tests: `name name`, `a: name a: age`, `name n: name` (no error), and the same name in two nested sets (no error).

### 8. Field parents and pointer targets (gap 5)

`selectable_declaration_errors` checks the parent's shape and, for `to T`, each named type of the annotation at that name's span. Upstream: undefined target (`isograph/crates/isograph_schema/src/process_client_field_declaration.rs:115`), scalar target (`:133`), scalar parent (`:70`).

```rust
// from crates/isograph_compiler/src/validation.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidEntityError {
    #[error("Invalid parent type. `{0}` is a leaf. You are attempting to define a client field on it. In order to do so, the parent object must be an object, interface or union.")]
    LeafParent(EntityName),
    #[error("Invalid parent type. `{0}` is an input type. You are attempting to define a client field on it. In order to do so, the parent object must be an object, interface or union.")]
    InputParent(EntityName),
    #[error("Invalid client pointer target type. `{0}` is a leaf. You are attempting to define a pointer to it. In order to do so, the type must be an object, interface or union.")]
    LeafTarget(EntityName),
    #[error("Invalid client pointer target type. `{0}` is an input type. You are attempting to define a pointer to it. In order to do so, the type must be an object, interface or union.")]
    InputTarget(EntityName),
}
```

An undefined target is the existing `UnknownEntity`. A source entity is a valid parent and target. Delta: upstream names a scalar "a scalar"; the core has `EntityShape::Leaf` and names it "a leaf". Input parents and targets are new: an input object is never selected (`InputTarget`). Tests: `to Pett`, `to String`, `to SetPetTaglineParams`, `to [Pet]`, `to Owner` (source entity, no error), `field String.x`.

### 9. Schema field arguments (gap 2)

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    /// An error of a selection's argument values.
    type ArgumentError: Clone + Debug + PartialEq + Eq + std::fmt::Display + 'static;

    /// The errors of `arguments`, passed by a selection of the protocol selectable
    /// `entity.selectable`, inside a `field` declaration whose variables are `variables`.
    /// Spans are the literal's.
    fn selection_argument_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        selectable: SelectableKey,
        selection_name: Span,
        arguments: Option<&WithSpan<ArgumentList>>,
        variables: Option<&WithSpan<VariableDeclarationList>>,
    ) -> Vec<WithSpan<Self::ArgumentError>>;
}
```

`ValidationError` gains `Argument(TArgumentError)`. The GraphQL implementation reads the field's `FieldDefinition.arguments` through `schema_lookup::selectable_named`; a refinement or `__typename` has none. Upstream: missing arguments (`isograph/crates/isograph_schema/src/validate_use_of_arguments.rs:381`), required being non-null without a default (`:358-365`); extra arguments (`:399-419`). Delta: one error per argument, each at the selection's name (missing) or the argument (extra), the shape of the landed `MissingFieldArgument`.

```rust
// from crates/graphql_network_protocol/src/argument_errors.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLArgumentError {
    #[error("`{field}` requires the argument `{argument}`, and this selection does not pass it.")]
    MissingArgument(ArgumentOf),
    #[error("`{field}` takes no argument `{argument}`.")]
    UnknownArgument(ArgumentOf),
    // change 10's variants
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgumentOf {
    pub field: SelectableKey,
    pub argument: ArgumentName,
}
```

The core reports an argument of a client field or pointer selection that the field does not declare as `ValidationError::UnknownFieldArgument(UnknownFieldArgument)` at the argument, with the merge's message. Tests: `pet` without `id`; `pet(id: $id, petId: 1)`; `checkins` with no argument (nullable, no error); `newsfeed(limit: 1)` (missing `skip`, non-null with no default); `asPet(x: 1)`.

### 10. Argument value types (gap 3)

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait NetworkProtocol: Sized + 'static {
    /// The errors of `arguments`, passed by a selection of a `field` declaration whose
    /// variables are `field_variables`, inside a declaration whose variables are `variables`.
    fn field_argument_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        field_variables: Option<&WithSpan<VariableDeclarationList>>,
        arguments: Option<&WithSpan<ArgumentList>>,
        variables: Option<&WithSpan<VariableDeclarationList>>,
    ) -> Vec<WithSpan<Self::ArgumentError>>;
}
```

Both methods check each passed value against its expected type, upstream's `value_satisfies_type` (`isograph/crates/isograph_schema/src/validate_argument_types.rs:142-298`):

- Variable: undefined (`:476`, "This variable is not defined: $x"); type mismatch (`:162`) by GraphQL's rule, upstream's `variable_type_satisfies_argument_type` (`:52-126`).
- `null` for a non-null type (`:232`).
- A literal against a scalar: an integer for `Int`, `Float`, `ID`; a string for `String`, `ID`; a boolean for `Boolean` (`:173-222`, message `:483-492`). An `Int` literal outside 32 bits is `IntegerOutOfRange` (spec 3.5.1). Delta: a custom scalar accepts every literal, because its literal coercion is defined by the server (spec 3.5); upstream rejects every literal for it. An enum accepts no literal: the iso grammar has none.
- An input object: missing fields (`:338`), extra fields (`:439`), each field's value checked. Delta: a non-null field with a default is not missing (spec 5.6.4); upstream ignores defaults. One error per field, at the object (missing) or the entry name (extra).
- A list type: a list literal's items are each checked against the item type (upstream checks them against the list type, `:439-447`, which is wrong); a non-list value is checked against the item type (input coercion, spec 3.11; upstream rejects it).

```rust
// from crates/graphql_network_protocol/src/argument_errors.rs
pub enum GraphQLArgumentError {
    // change 9's variants
    #[error("This variable is not defined: ${0}")]
    UndefinedVariable(VariableName),
    #[error("Mismatched type. Received ${variable}, with type {variable_type}, expected {expected}.")]
    VariableTypeMismatch(VariableTypeMismatch),
    #[error("Expected input of type {expected}, found {literal}.")]
    LiteralTypeMismatch(LiteralTypeMismatch),
    #[error("Expected non null input of type {0}, found null")]
    NullForNonNull(InputType),
    #[error("`{0}` is outside GraphQL's 32-bit `Int` range.")]
    IntegerOutOfRange(i64),
    #[error("`{entity}` requires the field `{field}`, and this object does not have it.")]
    MissingInputField(InputFieldOf),
    #[error("`{entity}` has no field `{field}`.")]
    UnknownInputField(InputFieldOf),
}
```

`InputType` is the protocol's input type (`Named`, `List`, `NonNull`), converted from the schema's `TypeAnnotation` and from the iso `TypeAnnotation`, displayed in GraphQL syntax. `LiteralKind` (`String`, `Integer`, `Boolean`, `Object`, `List`) displays as "a string literal" and so on. A type whose name the schema does not define is not checked here. Tests: one per rule and its degenerate case (empty object, empty list, a nullable variable into a nullable argument, `$x: Int` into `Int!`, a list of lists).

### 11. `__typename` (gap 6)

Upstream defines `__typename` on every object, interface, and union (`isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:331-355`, `:472-500`). `graphql_selectable(db, client, entity, "__typename")` returns, for those types, a `ProtocolSelectable { description: None, target: Wrapper::Inner(String), definition: None, data: GraphQLSelectableData::Field }`, and `graphql_selectables` lists it. Delta: the target is `String`, not upstream's literal entity `"Pet"`; a literal output type is artifact-set-and-types.md's (`:261`), whose owner types it. The core's `complete` already merges a user's leaf `__typename` with no arguments into `CompletedSelection::Discriminator` under every selection (query-generation.md, Completion, step 3), so the query prints one `__typename`, the normalization AST has no node for it, and an alias only renames the reader's key. At the top level a user's `__typename` is an ordinary selection, upstream's behavior. Reading it is reader-ast.md's: the store does not keep the discriminator (store.md), so the reader reads the entity name of the record's link. Tests: `pets { __typename }` validates, prints `__typename` once, and `t: __typename` too; `__typename { x }` is `SelectionSetOnLeaf`; `__typename(a: 1)` is `UnknownArgument`.

### 12. `id` must be `ID!` (gap 7)

Upstream: "The `id` field on `T` must have type `ID!`." under `on_invalid_id_type`, default `error` (`isograph/crates/isograph_schema/src/memoized/server_selectable_access.rs:38-62`, `isograph/crates/isograph_config/src/compilation_options.rs:233,254-262`). Here every field named `id` of an object or interface definition or extension whose type is not exactly `ID!` is `GraphQLSchemaFileError::InvalidIdField(EntityName)` at the field's type. Delta: no config option (open decision 2); `id: ID` is an error too, because a nullable id cannot key a record and i2 already gives such a type no identity; the message drops upstream's suppression sentence. Tests: `id: String`, `id: ID`, `id: [ID!]`, an extension's `id: Int`, `id: ID!` (none), an input object's `id: Int` (none).

### 13. List arguments (gap 8)

Upstream has `NonConstantValue::List` and panics when printing or aliasing it (`isograph/crates/isograph_lang_types/src/declarations/selection_argument.rs:86`). Query-generation.md's open decision 5 held an error because "the alias format is shared with the runtime"; it is not: the runtime reads `responseKey` from the AST (store.md, line 57) and builds store keys with `stableStringify`, which keeps array order. This change supports them.

- `MergedValue::List(Vec<MergedValue>)`; `merged_value` builds it, each unparsed item `UnparsedArgument`; `ListArgument` is deleted. Alias chunk `a_<item chunks joined by _>_c`, parallel to the object's `o_..._c`; two keys with one alias remain `AliasCollision`.
- `NormalizationValue::List(Vec<NormalizationValue>)`; the TypeScript printer writes `{ kind: "List", value: [..] }`; `ArgumentValue` in `libs/isograph-runtime/src/artifacts.ts` gains `{ readonly kind: 'List'; readonly value: ReadonlyArray<ArgumentValue> }`.
- The GraphQL printer prints `[a, b]`.

Tests: merging, substitution into a list, alias, printing, the normalization AST, and an empty list.

## Open decisions

1. `SourceNotRelative`, the one generation error that depends on the output folder, which is not in the database. Options: log it naming the field (held); put the generation options into the database as a singleton source so the check runs per file. Cost of the second: a source written by `serve`, and a host language method over it.
2. `on_invalid_id_type`. Options: always an error (held); upstream's option with `ignore`, `warn`, `error`. Cost of the option: i2 has no warning severity in its diagnostics, `isograph errors`, or exit codes, config values do not reach memos, and query-generation.md reserves a `graphql` config section for identity.
3. Custom scalars accept every literal (held) versus upstream's rejection of every literal.

## Docs this doc amends

- compiler-errors.md: change 9 is this doc's change 4.
- query-generation.md: open decision 5 is settled by change 13.
- reader-ast.md: a `__typename` selection reads the entity name of the record's link.
