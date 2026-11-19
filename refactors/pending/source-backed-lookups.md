# Source-backed lookups

`entity_named` and `selectable_named` stop being stubs. They read the `entity` and `field` declarations in the iso literals of every source file a client sees. Hover and go-to-definition on entity, selectable, and selection names (`refactors/past/selection-hover-and-definition.md`) then work for project-declared entities and fields. The GraphQL schema is not read yet; it merges in with the `NetworkProtocol` integration (see Later: the protocol's entities).

Origin: isograph `flattened_entity_named` / `flattened_selectable_named` (`crates/isograph_schema`), which collect definitions by name and report duplicates. Delta: i2 collects from source files only, keyed by `client`, and returns `DiagnosticResult` with the duplicate and missing cases as messages.

## What the user sees

```ts
// src/Pet.ts
iso(`entity Pet`)
iso(`field Pet.owner to Pet { id }`)
// src/fullName.ts
iso(`field Pet.fullName { owner { fullName } }`)
```

- Hover on `fullName` inside `owner { fullName }` shows `field **Pet.fullName**`. Go-to-definition jumps to `fullName` in `src/fullName.ts`.
- Hover on `owner` shows `field **Pet.owner**: **Pet**`. Go-to-definition jumps to `owner` in `src/Pet.ts`.
- Hover on `Pet` in any of the three literals shows `entity **Pet**`. Go-to-definition jumps to `Pet` in `entity Pet`.
- A second `entity Pet` anywhere a client sees: hover on `Pet` shows nothing and go-to-definition is `null`, everywhere, until one is removed.
- `field Pet.plain` (no `to`) and a selection `plain { id }`: hover on `id` shows nothing. `plain` itself still hovers.
- A literal with a parse error elsewhere contributes what it parsed and does not affect other lookups.
- An unsaved buffer counts for the client that has it open. Another client reads disk.
- Hover on a declaration's own name (`x` in `field Pet.x`) shows the field when that declaration is the only one, whether or not `Pet` is declared. The undeclared `Pet` is a validation error (`refactors/pending/validation-diagnostics.md`).

## Source file paths

```rust
// from crates/isograph_compiler/src/database.rs
/// Every source file path `client` reads: disk paths and `client`'s open paths, sorted.
#[memo]
pub fn source_file_paths<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let map = db.get_source_path_map().tracked();
    map.disk
        .iter()
        .copied()
        .chain(
            map.open
                .iter()
                .filter(|key| key.client == client)
                .map(|key| key.path),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
```

The memo iterates the map, so it reads `tracked()`. Adding or removing any source path re-invokes it; it backdates when the list is `==`. A content change does not write the map (`refactors/past/source-and-schema-path-maps.md`), so typing does not re-invoke it.

Known gap: pico records no dependency when a memo reads a tracked map whose counter was never written (`refactors/pending/pico-missing-key-dependency.md`). A lookup on a database where `source_path_map` was never written does not see the first source file. In the daemon the boot scan writes the map before the first request; tests insert their files before the first lookup.

## Items of the source files

```rust
// from crates/isograph_compiler/src/schema.rs
/// An iso literal item parsed in a source file, with the byte offset of its literal in that file.
pub(crate) struct SourceIsoLiteralItem<'db> {
    pub path: RelativePathToSourceFile,
    pub literal_start: u32,
    pub item: &'db IsoLiteralItem,
}

/// Every iso literal item parsed in a source file `client` reads. A literal with parse errors
/// contributes the item it parsed.
pub(crate) fn source_iso_literal_items<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> impl Iterator<Item = SourceIsoLiteralItem<'_>> {
    source_file_paths(db, client).iter().flat_map(move |path| {
        parsed_iso_literals_in_file(db, client, path.dereference())
            .iter()
            .flatten()
            .filter_map(move |(extraction, parsed)| {
                let item = parsed.item.as_ref()?.item.item.as_ref()?;
                SourceIsoLiteralItem {
                    path: path.dereference(),
                    literal_start: extraction.iso_literal_start_index.0 as u32,
                    item: item.item.reference(),
                }
                .wrap_some()
            })
    })
}
```

It is a plain function: its items borrow from `parsed_iso_literals_in_file` results. The callers below are memos keyed by name, so the walk runs once per name per edit, and each file's parse is the existing per-file memo.

## Definitions by name

```rust
// from crates/isograph_compiler/src/schema.rs
/// Where `entity <entity_name>` is declared in the source files `client` reads.
#[memo]
pub(crate) fn source_entity_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> Vec<DefinitionLocation> {
    source_iso_literal_items(db, client)
        .filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Entity(declaration) => (declaration.name.item.0 == entity_name)
                .then(|| DefinitionLocation {
                    path: source_item.path,
                    span: declaration.name.location.with_offset(source_item.literal_start),
                }),
            IsoLiteralItem::Entrypoint(_) | IsoLiteralItem::Selectable(_) => None,
        })
        .collect()
}

/// The selectable each `field <entity_name>.<selectable_name>` declaration in the source files
/// `client` reads defines.
#[memo]
pub(crate) fn source_selectable_definitions<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Vec<Selectable> {
    source_iso_literal_items(db, client)
        .filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Selectable(declaration) => (declaration.parent_type.item.0
                == entity_name
                && declaration.name.item.0 == selectable_name)
                .then(|| selectable_of_declaration(source_item.path, source_item.literal_start, declaration)),
            IsoLiteralItem::Entity(_) | IsoLiteralItem::Entrypoint(_) => None,
        })
        .collect()
}

/// The selectable `declaration`, in the literal at byte `literal_start` of `path`, defines.
fn selectable_of_declaration(
    path: RelativePathToSourceFile,
    literal_start: u32,
    declaration: &SelectableDeclaration,
) -> Selectable {
    Selectable {
        name: declaration.name.item.0,
        parent_entity: declaration.parent_type.item.0,
        description: declaration.description.map(|description| description.item.0),
        target_entity: declaration
            .target_type
            .as_ref()
            .and_then(|target_type| named_entity_of_type_annotation(target_type.item.reference())),
        definition: DefinitionLocation {
            path,
            span: declaration.name.location.with_offset(literal_start),
        },
    }
}

/// The entity a type annotation names, under its list and nullability wrappers. `None` for
/// `[]`, which names no entity.
fn named_entity_of_type_annotation(type_annotation: &TypeAnnotation) -> Option<EntityName> {
    match type_annotation {
        TypeAnnotation::Named(named) => named.name.item.0.wrap_some(),
        TypeAnnotation::List(list) => named_entity_of_list(list),
        TypeAnnotation::Union(union) => union.0.iter().find_map(|variant| match variant.item.reference() {
            UnionVariant::Named(named) => named.name.item.0.wrap_some(),
            UnionVariant::List(list) => named_entity_of_list(list),
            UnionVariant::Null(_) => None,
        }),
    }
}

fn named_entity_of_list(list: &ListTypeAnnotation) -> Option<EntityName> {
    named_entity_of_type_annotation(list.inner.as_ref()?.item.reference())
}
```

The parser builds a union only as `[written, Null]`, so the first non-`Null` variant is the one written. `source_entity_definitions` and `source_selectable_definitions` are memos so that validation (step 2) reads the same lists to report duplicates at every definition.

## Lookups

```rust
// from crates/isograph_compiler/src/schema.rs
/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    /// `None` for a field declared without `to`.
    pub target_entity: Option<EntityName>,
    pub definition: DefinitionLocation,
}
```

Before: `pub target_entity: EntityName`. `None` departs from `mental-model.md`, where a field without `to` points at an anonymous entity. Nothing can be selected on that anonymous entity, so i2 represents it as no target; `to []` (no named entity) is also `None`.

```rust
// from crates/isograph_compiler/src/schema.rs
/// The entity named `entity_name` in the schema as `client` reads it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Entity> {
    match source_entity_definitions(db, client, entity_name).as_slice() {
        [definition] => Entity {
            name: entity_name,
            definition: definition.dereference(),
        }
        .wrap_ok(),
        [] => Diagnostic::new(format!("Entity `{entity_name}` is not defined."), None).wrap_err(),
        [_, _, ..] => Diagnostic::new(
            format!("Entity `{entity_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
    }
}

/// The selectable named `selectable_name` on the entity named `entity_name` in the schema as
/// `client` reads it.
#[memo]
pub fn selectable_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    match source_selectable_definitions(db, client, entity_name, selectable_name).as_slice() {
        [selectable] => selectable.clone().wrap_ok(),
        [] => Diagnostic::new(
            format!("Field `{entity_name}.{selectable_name}` is not defined."),
            None,
        )
        .wrap_err(),
        [_, _, ..] => Diagnostic::new(
            format!("Field `{entity_name}.{selectable_name}` is defined more than once."),
            None,
        )
        .wrap_err(),
    }
}
```

The diagnostics carry no location: the caller holds the use site, and validation (step 2) places its own errors.

## The walk through a field without a target

Before:

```rust
// from crates/isograph_compiler/src/selectable_for_path.rs
SelectionSetParent::Selection(selection_path) => {
    let parent_selectable = selectable_for_selection_path(db, client, selection_path)?;
    selectable_for_entity(db, client, parent_selectable.target_entity, selectable_name)
}
```

After:

```rust
// from crates/isograph_compiler/src/selectable_for_path.rs
SelectionSetParent::Selection(selection_path) => {
    let parent_selectable = selectable_for_selection_path(db, client, selection_path)?;
    selectable_for_entity(db, client, selectable_target(&parent_selectable)?, selectable_name)
}

/// The entity selections under a selection of `selectable` select on.
pub fn selectable_target(selectable: &Selectable) -> DiagnosticResult<EntityName> {
    selectable.target_entity.ok_or_else(|| {
        Diagnostic::new(
            format!(
                "Field `{}.{}` has no target to select into.",
                selectable.parent_entity, selectable.name
            ),
            None,
        )
    })
}
```

## Hover text

Before:

```rust
// from crates/isograph_lsp/src/hover.rs
format!(
    "field **{}.{}**: **{}**{description}",
    selectable.parent_entity, selectable.name, selectable.target_entity
)
```

After:

```rust
// from crates/isograph_lsp/src/hover.rs
let target = selectable
    .target_entity
    .map(|target| format!(": **{target}**"))
    .unwrap_or_default();
format!(
    "field **{}.{}**{target}{description}",
    selectable.parent_entity, selectable.name
)
```

## Later: the protocol's entities

When the `NetworkProtocol` integration lands, `entity_named` and `selectable_named` match on the source definitions chained with the protocol's definitions of the same name (`graphql-schema-parser/integration.md`), so a source `entity Pet` and a GraphQL `type Pet` are two definitions and therefore a duplicate. `source_entity_definitions` and `source_selectable_definitions` stay source-only.

## Tests

`crates/isograph_lsp` (memo level; files inserted with source membership before the first lookup):

- `entity Pet` in `src/Pet.ts`, `field Pet.fullName` in `src/fullName.ts`, `field Pet.x { fullName }` in `src/x.ts`: hover on `fullName` in `src/x.ts` is `field **Pet.fullName**`; definition is `src/fullName.ts` at the span of `fullName`; hover and definition on `Pet` in `src/x.ts` name `src/Pet.ts` at `Pet`.
- `field Pet.owner to Pet` and `field Pet.x { owner { fullName } }`: hover on the nested `fullName` is `Pet.fullName`; hover on `owner` shows `: **Pet**`; `to [Pet!]` and `to Pet` both give target `Pet`; `to []` gives `None`.
- `field Pet.plain` and `field Pet.x { plain { id } }`: hover on `id` is `None`; `selectable_target` of `Pet.plain` is the no-target error.
- two `entity Pet` (two files, and two literals in one file): hover on `Pet` is `None`; `entity_named` is the duplicate error.
- a broken literal in another file (`field Pet.y { ) }`) leaves the results above unchanged; a broken literal that still parsed `entity Pet` contributes it.
- client 1's buffer declares `field Pet.fullName`, disk does not: client 1's lookup is `Ok`, client 2's is the missing error.
- a file outside the source membership that declares `entity Pet` contributes nothing.

`crates/isograph_cli/src/state.rs` (handle seam, `DiskChanged` then `textDocument/hover` and `textDocument/definition`): the three-file example; the definition response's URI is `file:///tmp/proj/src/fullName.ts` and its range is `fullName`'s range in that file. The existing `hover_on_a_schema_name_is_null_until_the_schema_exists` keeps its facts except the declaration names, whose own declaration now resolves: hover on `x` in `field Pet.x` shows `field **Pet.x**`, and definition lands on it.
