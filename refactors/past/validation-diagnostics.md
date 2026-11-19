# Validation diagnostics

Each open source file gets validation errors beside its parse errors: an unknown entity, an unknown field, a duplicated definition, and a selection set under a field that has no target. The errors read the source-backed lookups (`refactors/past/source-backed-lookups.md`).

Origin: isograph `crates/isograph_schema/src/validate_use_of_arguments.rs` walks every selection set with `visit_selection_set` and collects errors per selection. Delta: i2 walks one file's literals, keyed by `client` and `path`, returns errors with file spans, and does not check arguments.

## What the user sees

```ts
// src/Owner.ts, open in the editor
iso(`entity Owner`)
iso(`field Owner.displayName`)
iso(`field Owner.badge { displayName, nickname }`)      // `nickname`: Field `Owner.nickname` is not defined.
iso(`field Ownr.badge2 { anything { goes } }`)         // `Ownr`: Entity `Ownr` is not defined. Nothing else.
iso(`field Owner.plain { displayName { id } }`)        // `{ id }`: Field `Owner.displayName` has no target to select into.
iso(`entrypoint Owner.missing`)                        // `missing`: Field `Owner.missing` is not defined.
```

A second `entity Owner` in any source file puts `Entity `Owner` is defined more than once.` on the name of every `entity Owner` in open files. Every error is an LSP diagnostic of severity ERROR from source `isograph`, published for open source files with the existing debounce. Arguments are not checked.

## Types

```rust
// from crates/isograph_compiler/src/validation.rs
/// A reference in an iso literal that the source declarations do not satisfy.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("{0}")]
    UnknownEntity(UnknownEntityError),
    #[error("{0}")]
    UnknownSelectable(UnknownSelectableError),
    #[error("{0}")]
    DuplicateDefinition(DuplicateDefinitionError),
    #[error("{0}")]
    SelectionSetOnFieldWithoutTarget(SelectionSetOnFieldWithoutTargetError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Entity `{0}` is not defined.")]
pub struct UnknownEntityError(pub EntityName);

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Field `{}.{}` is not defined.", .0.entity, .0.selectable)]
pub struct UnknownSelectableError(pub SelectableKey);

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DuplicateDefinitionError {
    #[error("Entity `{0}` is defined more than once.")]
    Entity(EntityName),
    #[error("Field `{}.{}` is defined more than once.", .0.entity, .0.selectable)]
    Selectable(SelectableKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Field `{}.{}` has no target to select into.", .0.entity, .0.selectable)]
pub struct SelectionSetOnFieldWithoutTargetError(pub SelectableKey);

/// The identity of a selectable: its parent entity and its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectableKey {
    pub entity: EntityName,
    pub selectable: SelectableName,
}
```

The span of each error is the offending name: the entity name for `UnknownEntity`, the selection's field name (not its alias) or the entrypoint's name for `UnknownSelectable`, the declared name for `DuplicateDefinition`, and the selection set for `SelectionSetOnFieldWithoutTarget`.

## The memo

```rust
// from crates/isograph_compiler/src/validation.rs
/// Validation errors of every iso literal in `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn iso_literal_validation_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ValidationError>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| {
            let offset = extraction.iso_literal_start_index.0 as u32;
            parsed
                .item
                .iter()
                .filter_map(|parse| parse.item.item.as_ref())
                .flat_map(move |item| item_validation_errors(db, client, item.item.reference()))
                .map(move |error| error.item.with_span(error.location.with_offset(offset)))
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`iso_literal_parse_errors_in_file` is the same shape for parse errors. A literal with parse errors is validated on the item it parsed.

## One item

```rust
// from crates/isograph_compiler/src/validation.rs
type ValidationErrors<'a> = Box<dyn Iterator<Item = WithSpan<ValidationError>> + 'a>;

/// The validation errors of one iso literal item, in literal-relative spans.
fn item_validation_errors<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    item: &'a IsoLiteralItem,
) -> ValidationErrors<'a> {
    match item {
        IsoLiteralItem::Entity(declaration) => duplicate_entity_error(db, client, declaration.name)
            .into_iter()
            .boxed(),
        IsoLiteralItem::Selectable(declaration) => {
            selectable_declaration_errors(db, client, declaration)
        }
        IsoLiteralItem::Entrypoint(declaration) => entrypoint_declaration_error(db, client, declaration)
            .into_iter()
            .boxed(),
    }
}
```

Each `.boxed()` (`prelude::Postfix`) returns a `Box` of a concrete iterator, which the return type `ValidationErrors<'a>` coerces to the trait object. The arms return different iterator types, and `validate_selection` recurses, so the result is boxed.

```rust
// from crates/isograph_compiler/src/validation.rs
/// `DuplicateDefinition` on `name` when more than one source file declaration defines it.
fn duplicate_entity_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: WithSpan<EntityNameWrapper>,
) -> Option<WithSpan<ValidationError>> {
    (source_entity_definitions(db, client, name.item.0).len() > 1).then(|| {
        ValidationError::DuplicateDefinition(DuplicateDefinitionError::Entity(name.item.0))
            .with_span(name.location)
    })
}

/// `field E.x { .. }`: `E` is defined, `E.x` is not duplicated, and the selection set
/// selects on `E`. Nothing under an undefined `E` is checked.
fn selectable_declaration_errors<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    declaration: &'a SelectableDeclaration,
) -> ValidationErrors<'a> {
    let entity = declaration.parent_type.item.0;
    if source_entity_definitions(db, client, entity).is_empty() {
        return std::iter::once(
            ValidationError::UnknownEntity(UnknownEntityError(entity))
                .with_span(declaration.parent_type.location),
        )
        .boxed();
    }
    let key = SelectableKey {
        entity,
        selectable: declaration.name.item.0,
    };
    let duplicate = (source_selectable_definitions(db, client, key.entity, key.selectable).len() > 1)
        .then(|| {
            ValidationError::DuplicateDefinition(DuplicateDefinitionError::Selectable(key))
                .with_span(declaration.name.location)
        });
    duplicate
        .into_iter()
        .chain(
            declaration
                .selection_set
                .iter()
                .flat_map(move |selection_set| {
                    validate_selection_set(db, client, entity, selection_set.item.reference())
                }),
        )
        .boxed()
}

/// `entrypoint Q.X`: `Q` is defined, then `Q.X` is defined.
fn entrypoint_declaration_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    declaration: &EntrypointDeclaration,
) -> Option<WithSpan<ValidationError>> {
    let entity = declaration.parent_type.item.0;
    if source_entity_definitions(db, client, entity).is_empty() {
        return ValidationError::UnknownEntity(UnknownEntityError(entity))
            .with_span(declaration.parent_type.location)
            .wrap_some();
    }
    let key = SelectableKey {
        entity,
        selectable: declaration.name.item.0,
    };
    source_selectable_definitions(db, client, key.entity, key.selectable)
        .is_empty()
        .then(|| {
            ValidationError::UnknownSelectable(UnknownSelectableError(key))
                .with_span(declaration.name.location)
        })
}
```

"Defined" is "at least one definition": a duplicated entity is reported at its declarations and is otherwise treated as defined, so its fields are still checked. `entity_named` stays `Err` for it, so hover still shows nothing there.

## Selection sets

```rust
// from crates/isograph_compiler/src/validation.rs
/// The validation errors of `selection_set`, a selection set on `parent`, in literal-relative
/// spans. Each branch stops at its first error; sibling selections are still checked.
pub fn validate_selection_set<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection_set: &'a SelectionSet,
) -> impl Iterator<Item = WithSpan<ValidationError>> + 'a {
    selection_set
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(move |selection| validate_selection(db, client, parent, selection.item.reference()))
}

fn validate_selection<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection: &'a Selection,
) -> ValidationErrors<'a> {
    let key = SelectableKey {
        entity: parent,
        selectable: selection.name.item.0.unchecked_conversion(),
    };
    match source_selectable_definitions(db, client, key.entity, key.selectable).as_slice() {
        [] => std::iter::once(
            ValidationError::UnknownSelectable(UnknownSelectableError(key))
                .with_span(selection.name.location),
        )
        .boxed(),
        [selectable] => match (selection.selection_set.as_ref(), selectable.target_entity) {
            (None, _) => std::iter::empty().boxed(),
            (Some(nested), None) => std::iter::once(
                ValidationError::SelectionSetOnFieldWithoutTarget(
                    SelectionSetOnFieldWithoutTargetError(key),
                )
                .with_span(nested.location),
            )
            .boxed(),
            (Some(nested), Some(target)) => {
                if source_entity_definitions(db, client, target).is_empty() {
                    std::iter::empty().boxed()
                } else {
                    validate_selection_set(db, client, target, nested.item.reference()).boxed()
                }
            }
        },
        [_, _, ..] => std::iter::empty().boxed(),
    }
}
```

`unchecked_conversion` is the walk's existing `SelectionName` to `SelectableName` conversion (`selectable_for_selection_path`). A duplicated field selected here is reported at its declarations and stops this branch, since which target to recurse into is undecided. A target entity that is not defined stops the branch with no error here: no checked reference names it.

## Diagnostics

Before:

```rust
// from crates/isograph_lsp/src/file_diagnostics.rs
let errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
if errors.is_empty() {
    return Vec::new().wrap_some();
}
let line_index = line_index(db, client, path).as_ref()?;
errors
    .iter()
    .map(|error| lsp_types::Diagnostic {
        range: line_index.range(error.location),
        severity: lsp_types::DiagnosticSeverity::ERROR.wrap_some(),
        source: DIAGNOSTIC_SOURCE.to_owned().wrap_some(),
        message: error.item.to_string(),
        ..Default::default()
    })
    .collect::<Vec<_>>()
    .wrap_some()
```

After:

```rust
// from crates/isograph_lsp/src/file_diagnostics.rs
let parse_errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
let validation_errors = iso_literal_validation_errors_in_file(db, client, path).as_ref()?;
if parse_errors.is_empty() && validation_errors.is_empty() {
    return Vec::new().wrap_some();
}
let line_index = line_index(db, client, path).as_ref()?;
let diagnostic = |location: Span, message: String| lsp_types::Diagnostic {
    range: line_index.range(location),
    severity: lsp_types::DiagnosticSeverity::ERROR.wrap_some(),
    source: DIAGNOSTIC_SOURCE.to_owned().wrap_some(),
    message,
    ..Default::default()
};
parse_errors
    .iter()
    .map(|error| diagnostic(error.location, error.item.to_string()))
    .chain(
        validation_errors
            .iter()
            .map(|error| diagnostic(error.location, error.item.to_string())),
    )
    .collect::<Vec<_>>()
    .wrap_some()
```

Scope and debounce are unchanged: open source files, published when the debounce armed by `didOpen`, `didChange`, or `didClose` fires.

## Tests

`crates/isograph_lsp/src/file_diagnostics.rs` (memo level, source membership):

- each kind with its message and range: `field Ownr.x { a }` (UnknownEntity on `Ownr`), `field Owner.x { nickname }` (UnknownSelectable on `nickname`), `entity Owner` twice in one file and once in another file (DuplicateDefinition on each `Owner` name in this file), `field Owner.d` twice (DuplicateDefinition on each `d`), `field Owner.plain { plain { id } }` (SelectionSetOnFieldWithoutTarget on `{ id }`), `entrypoint Owner.missing` (UnknownSelectable on `missing`), `entrypoint Ownr.x` (UnknownEntity on `Ownr` only).
- cascade: `field Ownr.x { a { b } }` is exactly one error; `field Owner.x { nope { deeper }, displayName, nickname }` is errors on `nope` and `nickname` only; a selection through `to Ownr` (undefined) produces nothing under it.
- an aliased selection is checked by field name: `field Owner.x { alias: displayName }` has no error; `alias: nope` errors on `nope`.
- arguments are not checked: `field Owner.x { displayName(id: 1) }` has no error.
- a pet-demo-like set of literals with source-only entities (`entity Owner`, `entity Pet`, `field Owner.displayName`, `field Pet.owner to Owner! { id }` with `field Pet.id`, `field Owner.OwnerBadge { displayName }`, `field Pet.card { owner { displayName, OwnerBadge } }`, `entrypoint Pet.card`) has no diagnostics.
- parse and validation errors are both published, parse errors first.

`crates/isograph_cli/src/state.rs` (handle seam): `didOpen` of a literal selecting an undefined field, then the debounce fires, publishes that one diagnostic with its range.
