use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
use isograph_parser::{
    EntityNameWrapper, EntrypointDeclaration, IsoLiteralItem, SelectableDeclaration, Selection,
    SelectionSet,
};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::schema::{source_entity_definitions, source_selectable_definitions};
use crate::{ClientId, CompilationProfile, IsographDb, parsed_iso_literals_in_file};

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
        IsoLiteralItem::Entrypoint(declaration) => {
            entrypoint_declaration_error(db, client, declaration)
                .into_iter()
                .boxed()
        }
    }
}

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
    let duplicate = (source_selectable_definitions(db, client, key.entity, key.selectable).len()
        > 1)
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

/// The validation errors of `selection_set`, a selection set on `parent`, in literal-relative
/// spans. Each branch stops at its first error; sibling selections are still checked.
fn validate_selection_set<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection_set: &'a SelectionSet,
) -> impl Iterator<Item = WithSpan<ValidationError>> + 'a {
    selection_set
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(move |selection| {
            validate_selection(db, client, parent, selection.item.reference())
        })
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
        [selectable] => match (selection.selection_set.as_ref(), selectable.target_entity()) {
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
