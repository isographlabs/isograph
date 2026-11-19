# Find references

`textDocument/references` on a name in an iso literal returns every place in the source files that names the same entity or selectable, and the item's declaration when the request's `context.includeDeclaration` is true.

Origin: none. Upstream isograph `crates/isograph_lsp` has no references handler (no `References`, `ReferenceParams`, or `references_provider` anywhere in `crates/`). The position step is the one hover and go-to-definition use (`refactors/past/selection-hover-and-definition.md`); the per-file walk follows `iso_literal_validation_errors_in_file` (`crates/isograph_compiler/src/validation.rs`).

## What the user sees

```ts
// src/Pet.ts
iso(`entity Pet`)
iso(`field Pet.owner to Pet { id }`)
// src/fullName.ts
iso(`field Pet.fullName { id }`)
// src/x.ts
iso(`field Pet.x { owner { fullName } }`)
// src/y.ts
iso(`field Pet.y { n: fullName }`)
```

- Shift-F12 on `fullName` in `src/x.ts` lists `fullName` in `src/x.ts` and `fullName` (not `n`) in `src/y.ts`. With `includeDeclaration` the list also holds `fullName` in `src/fullName.ts`.
- Shift-F12 on any `Pet` lists the `Pet` of `field Pet.owner`, the `Pet` after `to`, and the `Pet` of `field Pet.fullName`, `field Pet.x`, `field Pet.y`. With `includeDeclaration` it also holds `Pet` in `entity Pet`.
- Shift-F12 on `asPet` in `feed { asPet { name } }` lists every `asPet` selection on `FeedItem`; the declaration is the location go-to-definition returns for `asPet` (the `Pet` of `type Pet` in the schema file).
- A file a client has open contributes its buffer for that client; another client reads disk.
- A position in the TypeScript around the literals returns `null`.
- A position inside a literal that names no entity or selectable (`field`, `{`, an argument), or names one whose lookup fails (undefined, defined more than once), returns `[]`.
- Schema files contribute only the declaration. The schema AST has no position resolution, so `pet: Pet` in a `.graphql` file is not a reference to `Pet`.

## Reference target

```rust
// from crates/isograph_compiler/src/references.rs
/// The entity or selectable a name in an iso literal refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceTarget {
    Entity(EntityName),
    Selectable(SelectableKey),
}
```

`SelectableKey` is the existing `pub struct SelectableKey { pub entity: EntityName, pub selectable: SelectableName }` in `validation.rs`.

## Per-file index

```rust
// from crates/isograph_compiler/src/references.rs
/// Every reference to an entity or selectable in the iso literals of `path` as `client`
/// reads it, as file-absolute spans. A declaration's own name is not a reference.
#[memo]
pub fn references_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ReferenceTarget>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| {
            let offset = extraction.iso_literal_start_index.0 as u32;
            parsed
                .item
                .iter()
                .filter_map(|parse| parse.item.item.as_ref())
                .flat_map(move |item| item_references(db, client, item.item.reference()))
                .map(move |reference| {
                    reference
                        .item
                        .with_span(reference.location.with_offset(offset))
                })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` when `path` has no text, same as `iso_literal_validation_errors_in_file`. Keyed by `client` and `path`; it depends on the lookups its walk makes (`entity_named`, `selectable_named`), so a declaration added in another file re-invokes it.

What each item contributes, in literal-relative spans:

```rust
// from crates/isograph_compiler/src/references.rs
type ReferenceIter<'a> = Box<dyn Iterator<Item = WithSpan<ReferenceTarget>> + 'a>;

fn item_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    item: &'a IsoLiteralItem,
) -> ReferenceIter<'a> {
    match item {
        IsoLiteralItem::Entity(_) => std::iter::empty().boxed(),
        IsoLiteralItem::Entrypoint(declaration) => entrypoint_references(declaration).boxed(),
        IsoLiteralItem::Selectable(declaration) => {
            selectable_declaration_references(db, client, declaration)
        }
    }
}

/// `entrypoint E.x`: `E`, then `E.x`.
fn entrypoint_references(
    declaration: &EntrypointDeclaration,
) -> impl Iterator<Item = WithSpan<ReferenceTarget>> {
    [
        entity_reference(declaration.parent_type),
        ReferenceTarget::Selectable(SelectableKey {
            entity: declaration.parent_type.item.0,
            selectable: declaration.name.item.0,
        })
        .with_span(declaration.name.location),
    ]
    .into_iter()
}

fn entity_reference(name: WithSpan<EntityNameWrapper>) -> WithSpan<ReferenceTarget> {
    ReferenceTarget::Entity(name.item.0).with_span(name.location)
}

/// `field E.x($v: T) to U { .. }`: `E`, every entity in `T` and `U`, then the selections.
/// The selections are walked only when `E` resolves.
fn selectable_declaration_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    declaration: &'a SelectableDeclaration,
) -> ReferenceIter<'a> {
    let variables = declaration
        .variable_definitions
        .iter()
        .flat_map(|list| list.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(|variable| type_annotation_references(variable.item.type_.item.reference()));
    let target = declaration
        .target_type
        .iter()
        .flat_map(|target| type_annotation_references(target.item.reference()));
    let selections = declaration
        .selection_set
        .iter()
        .filter_map(move |selection_set| {
            let entity = entity_named(db, client, declaration.parent_type.item.0)
                .as_ref()
                .ok()?;
            selection_set_references(db, client, entity.name, selection_set.item.reference())
                .wrap_some()
        })
        .flatten();
    std::iter::once(entity_reference(declaration.parent_type))
        .chain(variables)
        .chain(target)
        .chain(selections)
        .boxed()
}

/// Every entity `annotation` names.
fn type_annotation_references(annotation: &TypeAnnotation) -> ReferenceIter<'_> {
    match annotation {
        TypeAnnotation::Named(named) => std::iter::once(entity_reference(named.name)).boxed(),
        TypeAnnotation::List(list) => list_type_annotation_references(list),
        TypeAnnotation::Union(union) => union
            .0
            .iter()
            .flat_map(|variant| match variant.item.reference() {
                UnionVariant::Named(named) => std::iter::once(entity_reference(named.name)).boxed(),
                UnionVariant::List(list) => list_type_annotation_references(list),
                UnionVariant::Null(_) => std::iter::empty().boxed(),
            })
            .boxed(),
    }
}

fn list_type_annotation_references(list: &ListTypeAnnotation) -> ReferenceIter<'_> {
    list.inner
        .iter()
        .flat_map(|inner| type_annotation_references(inner.item.reference()))
        .boxed()
}
```

The selection walk is top-down with the same lookups the position step makes (`entity_for_selection_set_path` in `selectable_for_path.rs`): a selection set under `field E.x` selects on `E` when `entity_named(E)` is `Ok`; a selection set under a selection of `P.f` selects on `T` when `selectable_named(P, f)` is `Ok`, `selectable_target` is `Ok(T)`, and `entity_named(T)` is `Ok`. A failed lookup stops that branch. Its siblings are still walked. Every selection on a resolved parent `P` is a reference to `P.<name>`, whether or not `P.<name>` is defined. The alias is not part of the reference; the span is the field name's.

```rust
// from crates/isograph_compiler/src/references.rs
/// The references in `selection_set`, a selection set on `parent`.
fn selection_set_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection_set: &'a SelectionSet,
) -> ReferenceIter<'a> {
    selection_set
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(move |selection| {
            selection_references(db, client, parent, selection.item.reference())
        })
        .boxed()
}

/// `P.f` at the name of `selection`, then the references in its selection set when that set
/// selects on a resolved entity.
fn selection_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection: &'a Selection,
) -> ReferenceIter<'a> {
    let key = SelectableKey {
        entity: parent,
        selectable: selection.name.item.0.unchecked_conversion(),
    };
    let nested = selection
        .selection_set
        .iter()
        .filter_map(move |nested| {
            selection_set_references(
                db,
                client,
                nested_selection_set_entity(db, client, key)?,
                nested.item.reference(),
            )
            .wrap_some()
        })
        .flatten();
    std::iter::once(ReferenceTarget::Selectable(key).with_span(selection.name.location))
        .chain(nested)
        .boxed()
}

/// The entity a selection set under a selection of `key` selects on.
fn nested_selection_set_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    key: SelectableKey,
) -> Option<EntityName> {
    let selectable = selectable_named(db, client, key.entity, key.selectable)
        .as_ref()
        .ok()?;
    let target = selectable_target(selectable).ok()?;
    entity_named(db, client, target)
        .as_ref()
        .ok()
        .map(|entity| entity.name)
}
```

Declaration names are not indexed: `entity E` contributes nothing and the `x` of `field E.x` is not a reference. The declaration comes from the target's `DefinitionLocation` (below). When the target resolves, its definition is unique, so for a source-declared item that location is the declaration name's span, and indexing it would list it twice under `includeDeclaration`.

`lib.rs` exports `references::{ReferenceTarget, references_in_file}`.

## Target under the cursor

```rust
// from crates/isograph_lsp/src/entity_or_selectable.rs
impl<TEntityData, TSelectableData> EntityOrSelectable<TEntityData, TSelectableData> {
    /// The identity references to this item carry.
    pub(crate) fn reference_target(&self) -> ReferenceTarget {
        match self {
            EntityOrSelectable::Entity(entity) => ReferenceTarget::Entity(entity.name),
            EntityOrSelectable::Selectable(selectable) => ReferenceTarget::Selectable(SelectableKey {
                entity: selectable.parent_entity,
                selectable: selectable.name,
            }),
        }
    }
}
```

## The request

```rust
// from crates/isograph_lsp/src/references.rs
/// What a position inside an iso literal names, for find references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferencesUnderCursor {
    Target(References),
    /// The position names no entity or selectable, or names one whose lookup fails.
    NoTarget,
}

/// The declaration of an entity or selectable, and every reference to it in the source files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct References {
    pub declaration: DefinitionLocation,
    pub references: Vec<ReferencesInFile>,
}

/// The ranges of the references in one source file, in that file's line index as the client
/// reads it. `ranges` is not empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferencesInFile {
    pub path: RelativePathToSourceFile,
    pub ranges: Vec<lsp_types::Range>,
}

/// The references to the item named at `position` in `path` as `client` reads it. `None` when
/// `position` is not inside an iso literal.
pub fn references_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<ReferencesUnderCursor> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    entity_or_selectable_at(db, client, literal_id, cursor)
        .map_or(ReferencesUnderCursor::NoTarget, |item| {
            ReferencesUnderCursor::Target(references_to(db, client, item.reference()))
        })
        .wrap_some()
}

/// The schema item the char at `cursor` in `literal_id` names, when its lookup succeeds.
fn entity_or_selectable_at<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    literal_id: LiteralId,
    cursor: Span,
) -> Option<ProfileEntityOrSelectable<TProfile>> {
    let parsed = parsed_iso_literal_by_id(db, literal_id)?;
    entity_or_selectable_of_node(db, client, resolve_offset(parsed, cursor)?.reference())?.ok()
}

fn references_to<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    item: &ProfileEntityOrSelectable<TProfile>,
) -> References {
    let target = item.reference_target();
    References {
        declaration: item.definition(),
        references: source_file_paths(db, client)
            .iter()
            .filter_map(|path| references_in_file_to(db, client, path.dereference(), target))
            .collect(),
    }
}

/// The references to `target` in `path`. `None` when there are none.
fn references_in_file_to<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    target: ReferenceTarget,
) -> Option<ReferencesInFile> {
    let references = references_in_file(db, client, path).as_ref()?;
    let line_index = line_index(db, client, path).as_ref()?;
    let ranges = references
        .iter()
        .filter(|reference| reference.item == target)
        .map(|reference| line_index.range(reference.location))
        .collect::<Vec<_>>();
    (!ranges.is_empty()).then(|| ReferencesInFile { path, ranges })
}
```

A position inside an empty or whitespace-only literal resolves to no node and returns `NoTarget` (`[]`), not `null`; `null` is only for a position outside every literal. This matches completion, not definition.

`lib.rs` exports `references::{References, ReferencesInFile, ReferencesUnderCursor, references_at_lsp_position}`.

## Adapter and dispatch

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn references_response<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    params: lsp_types::ReferenceParams,
) -> LSPRuntimeResult<<lsp_types::request::References as lsp_types::request::Request>::Result> {
    let position = params.text_document_position;
    let path = relative_from_uri(db, position.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::references_at_lsp_position(db, client, path, position.position)
        .map(|under_cursor| match under_cursor {
            isograph_lsp::ReferencesUnderCursor::Target(references) => reference_lsp_locations(
                db,
                client,
                params
                    .context
                    .include_declaration
                    .then_some(references.declaration),
                references.references,
            ),
            isograph_lsp::ReferencesUnderCursor::NoTarget => Vec::new().wrap_ok(),
        })
        .transpose()
}

/// `declaration`, then every reference, as LSP locations.
fn reference_lsp_locations<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    declaration: Option<DefinitionLocation>,
    references: Vec<isograph_lsp::ReferencesInFile>,
) -> LSPRuntimeResult<Vec<lsp_types::Location>> {
    let declaration = declaration
        .map(|declaration| definition_lsp_location(db, client, declaration))
        .transpose()?;
    let references = references
        .into_iter()
        .map(|file| file_lsp_locations(db, file))
        .collect::<LSPRuntimeResult<Vec<_>>>()?;
    declaration
        .into_iter()
        .chain(references.into_iter().flatten())
        .collect::<Vec<_>>()
        .wrap_ok()
}

fn file_lsp_locations<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    file: isograph_lsp::ReferencesInFile,
) -> LSPRuntimeResult<Vec<lsp_types::Location>> {
    let uri = uri_for_path(db, file.path)
        .map_err(|error| LSPRuntimeError::UnexpectedError(error.to_string()))?;
    file.ranges
        .into_iter()
        .map(|range| lsp_types::Location {
            uri: uri.clone(),
            range,
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}
```

`includeDeclaration` is the LSP's `bool`. It becomes `Option<DefinitionLocation>` at the adapter and goes no further.

```rust
// from crates/isograph_cli/src/state.rs (dispatch_lsp_request), after Completion
            .on_request_sync::<lsp_types::request::References>(
                crate::adapter::references_response::<TProfile>,
            )?
```

```rust
// from crates/isograph_cli/src/lsp_socket.rs (ServerCapabilities)
        references_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

## Tests

Memo-level, `crates/isograph_compiler/src/references.rs` (`TestProfile`, `iso(` extraction, no protocol):

- `entity`, `entrypoint`, `field` parent, `to` (plain, list, nullable, union), and variable types: each entity name is `Entity`; `entity Pet` contributes nothing; the `x` of `field Pet.x` is not a reference.
- selections: top-level and nested under a pointer resolve to the parent entity; an alias records the field name's span; a selection on an undefined field is still a reference to that field.
- branch stops: a selection set under an undefined parent entity, under a field without a target, under a field whose target is undefined, and under an undefined field contributes nothing; siblings are still walked.
- a literal with parse errors contributes what it parsed.
- spans are file-absolute across two literals in one file.
- degenerate: a path with no text is `None`; a file with no literal and an empty literal are `Some(vec![])`.

Handle seam, `crates/isograph_cli/src/state.rs`, over `textDocument/references`:

- an entity: references from `field` parents and `to`, across files; `includeDeclaration` adds `entity Pet`.
- a field: references from selections in two files, nested and aliased; `includeDeclaration` on and off.
- a schema refinement field `asPet` with `SCHEMA`: references in two files; the declaration is the schema location.
- a client's open buffer contributes for that client; another client reads disk.
- outside every literal is `null`; on `field` and on an undefined field's selection is `[]`.

`lsp_socket.rs`: the `initialize` capabilities test asserts `referencesProvider: true`.
