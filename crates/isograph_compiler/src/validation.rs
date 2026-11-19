use std::collections::BTreeSet;

use common_lang_types::{
    EntityName, EntityNameAndSelectableName, RelativePathToSourceFile, SelectableName,
    SelectionName, VariableName,
};
use intern::Lookup;
use isograph_parser::{
    EntityNameWrapper, EntrypointDeclaration, IsoLiteralItem, ListTypeAnnotation,
    SelectableDeclaration, Selection, SelectionSet, TypeAnnotation, UnionVariant,
    VariableDeclarationList,
};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::merged_selection::UnknownFieldArgument;
use crate::schema::{Definitions, entity_definitions, selectable_definitions};
use crate::{
    ClientId, CompilationProfile, EntityOrigin, EntityShape, IsographDb, NetworkProtocol,
    ProfileEntity, SelectableKind, entity_named, entrypoint_fetch_plan, field_declaration,
    parsed_iso_literals_in_file, selectable_named,
};

/// A reference in an iso literal that the source declarations do not satisfy.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError<TNotFetchable: std::fmt::Display, TArgumentError: std::fmt::Display> {
    #[error("{0}")]
    UnknownEntity(UnknownEntityError),
    #[error("{0}")]
    UnknownSelectable(UnknownSelectableError),
    #[error("{0}")]
    DuplicateDefinition(DuplicateDefinitionError),
    #[error("{0}")]
    SelectionSetOnFieldWithoutTarget(SelectionSetOnFieldWithoutTargetError),
    #[error("{0}")]
    EntrypointNotFetchable(EntrypointNotFetchableError<TNotFetchable>),
    #[error("{0}")]
    MissingFieldArgument(MissingFieldArgument),
    #[error("`{0}` returns a leaf and takes no selection set.")]
    SelectionSetOnLeaf(SelectableKey),
    #[error("`{0}` returns an entity with selectables and needs a selection set.")]
    MissingSelectionSet(SelectableKey),
    #[error("`{0}` is an input type, which cannot be selected.")]
    InputTarget(EntityName),
    #[error("{0}")]
    DuplicateResponseName(DuplicateResponseName),
    #[error("{0}")]
    InvalidEntity(InvalidEntityError),
    #[error("{0}")]
    UnknownFieldArgument(UnknownFieldArgument),
    /// The protocol's verdict on a selection's argument values.
    #[error("{0}")]
    Argument(TArgumentError),
}

/// An entity that cannot parent a `field` declaration or be the target of its `to` clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidEntityError {
    #[error(
        "Invalid parent type. `{0}` is a leaf. You are attempting to define a client field on it. \
        In order to do so, the parent object must be an object, interface or union."
    )]
    LeafParent(EntityName),
    #[error(
        "Invalid parent type. `{0}` is an input type. You are attempting to define a client field \
        on it. In order to do so, the parent object must be an object, interface or union."
    )]
    InputParent(EntityName),
    #[error(
        "Invalid client pointer target type. `{0}` is a leaf. You are attempting to define a \
        pointer to it. In order to do so, the type must be an object, interface or union."
    )]
    LeafTarget(EntityName),
    #[error(
        "Invalid client pointer target type. `{0}` is an input type. You are attempting to define \
        a pointer to it. In order to do so, the type must be an object, interface or union."
    )]
    InputTarget(EntityName),
}

/// A selection whose response name, its alias or else its name, an earlier selection in the
/// same selection set has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("A field with name or alias `{0}` has already been defined")]
pub struct DuplicateResponseName(pub SelectionName);

/// A selection of `field` that passes no value for `variable`, which has no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{field}` declares `${variable}` with no default, and this selection does not pass it.")]
pub struct MissingFieldArgument {
    pub field: SelectableKey,
    pub variable: VariableName,
}

/// Why `entrypoint E.f` cannot be fetched.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointNotFetchableError<TNotFetchable: std::fmt::Display> {
    #[error("`{}.{}` comes from the schema; an entrypoint names a `field` declaration.", .0.entity, .0.selectable)]
    ProtocolSelectable(SelectableKey),
    #[error("{0}")]
    Entity(EntityNotFetchableError<TNotFetchable>),
}

/// Why an entity cannot be fetched by itself.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntityNotFetchableError<TNotFetchable: std::fmt::Display> {
    #[error("`{0}` is declared in source files, so the network protocol cannot fetch it.")]
    SourceEntity(EntityName),
    #[error("`{0}` has no selectables, so it cannot be fetched.")]
    NotComposite(EntityName),
    #[error("{0}")]
    Protocol(TNotFetchable),
}

pub type ProfileValidationError<TProfile> = ValidationError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::ArgumentError,
>;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[display("{entity}.{selectable}")]
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
) -> Option<Vec<WithSpan<ProfileValidationError<TProfile>>>> {
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

type ValidationErrors<'a, TProfile> =
    Box<dyn Iterator<Item = WithSpan<ProfileValidationError<TProfile>>> + 'a>;

/// The validation errors of one iso literal item, in literal-relative spans.
fn item_validation_errors<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    item: &'a IsoLiteralItem,
) -> ValidationErrors<'a, TProfile> {
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

/// `DuplicateDefinition` on `name` when more than one source file declaration or the
/// protocol defines it.
fn duplicate_entity_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: WithSpan<EntityNameWrapper>,
) -> Option<WithSpan<ProfileValidationError<TProfile>>> {
    match entity_definitions(db, client, name.item.0) {
        Definitions::Duplicated => {
            ValidationError::DuplicateDefinition(DuplicateDefinitionError::Entity(name.item.0))
                .with_span(name.location)
                .wrap_some()
        }
        Definitions::Undefined | Definitions::Unique(_) | Definitions::ProtocolLookupFailed(_) => {
            None
        }
    }
}

/// Whether the source files or the protocol define the entity named `entity`.
fn entity_definedness<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> EntityDefinedness {
    match entity_definitions(db, client, entity) {
        Definitions::Undefined => EntityDefinedness::Undefined,
        Definitions::Unique(_) | Definitions::Duplicated | Definitions::ProtocolLookupFailed(_) => {
            EntityDefinedness::Defined
        }
    }
}

/// Whether an entity has at least one definition, however its lookup ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntityDefinedness {
    Undefined,
    Defined,
}

/// `field E.x { .. }`: `E` is defined, `E.x` is not duplicated, and the selection set
/// selects on `E`. Nothing under an undefined `E` is checked.
fn selectable_declaration_errors<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    declaration: &'a SelectableDeclaration,
) -> ValidationErrors<'a, TProfile> {
    let entity = declaration.parent_type.item.0;
    if let EntityDefinedness::Undefined = entity_definedness(db, client, entity) {
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
    let duplicate = match selectable_definitions(db, client, key.entity, key.selectable) {
        Definitions::Duplicated => {
            ValidationError::DuplicateDefinition(DuplicateDefinitionError::Selectable(key))
                .with_span(declaration.name.location)
                .wrap_some()
        }
        Definitions::Undefined | Definitions::Unique(_) | Definitions::ProtocolLookupFailed(_) => {
            None
        }
    };
    let parent = parent_error(db, client, entity).map(|error| {
        ValidationError::InvalidEntity(error).with_span(declaration.parent_type.location)
    });
    let targets = declaration.target_type.iter().flat_map(move |target_type| {
        named_types(target_type.item.reference())
            .into_iter()
            .filter_map(move |name| pointer_target_error(db, client, name))
    });
    duplicate
        .into_iter()
        .chain(parent)
        .chain(targets)
        .chain(
            declaration
                .selection_set
                .iter()
                .flat_map(move |selection_set| {
                    validate_selection_set(
                        db,
                        client,
                        declaration.variable_definitions.as_ref(),
                        entity,
                        selection_set.item.reference(),
                    )
                }),
        )
        .boxed()
}

/// `InvalidEntityError` when the defined entity `parent` cannot parent a `field` declaration.
fn parent_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
) -> Option<InvalidEntityError> {
    let entity = entity_named(db, client, parent).as_ref().ok()?;
    match target_shape::<TProfile>(entity) {
        TargetShape::Composite => None,
        TargetShape::Leaf => InvalidEntityError::LeafParent(parent).wrap_some(),
        TargetShape::Input => InvalidEntityError::InputParent(parent).wrap_some(),
    }
}

/// The error of `name`, a type a `to` clause names: undefined, or not an entity a pointer can
/// point to.
fn pointer_target_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    name: WithSpan<EntityNameWrapper>,
) -> Option<WithSpan<ProfileValidationError<TProfile>>> {
    let target = name.item.0;
    if let EntityDefinedness::Undefined = entity_definedness(db, client, target) {
        return ValidationError::UnknownEntity(UnknownEntityError(target))
            .with_span(name.location)
            .wrap_some();
    }
    let entity = entity_named(db, client, target).as_ref().ok()?;
    let error = match target_shape::<TProfile>(entity) {
        TargetShape::Composite => return None,
        TargetShape::Leaf => InvalidEntityError::LeafTarget(target),
        TargetShape::Input => InvalidEntityError::InputTarget(target),
    };
    ValidationError::InvalidEntity(error)
        .with_span(name.location)
        .wrap_some()
}

/// Every named type in `annotation`, in source order.
fn named_types(annotation: &TypeAnnotation) -> Vec<WithSpan<EntityNameWrapper>> {
    match annotation {
        TypeAnnotation::Named(named) => named.name.wrap_vec(),
        TypeAnnotation::List(list) => list_named_types(list),
        TypeAnnotation::Union(union) => union
            .0
            .iter()
            .flat_map(|variant| match variant.item.reference() {
                UnionVariant::Named(named) => named.name.wrap_vec(),
                UnionVariant::List(list) => list_named_types(list),
                UnionVariant::Null(_) => Vec::new(),
            })
            .collect(),
    }
}

fn list_named_types(list: &ListTypeAnnotation) -> Vec<WithSpan<EntityNameWrapper>> {
    list.inner
        .iter()
        .flat_map(|inner| named_types(inner.item.reference()))
        .collect()
}

/// `entrypoint E.f`: `E` and `E.f` are defined, `E` comes from the protocol, `E.f` is a
/// `field` declaration, and the protocol can fetch `E`.
fn entrypoint_declaration_error<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    declaration: &EntrypointDeclaration,
) -> Option<WithSpan<ProfileValidationError<TProfile>>> {
    let entity = declaration.parent_type.item.0;
    if let EntityDefinedness::Undefined = entity_definedness(db, client, entity) {
        return ValidationError::UnknownEntity(UnknownEntityError(entity))
            .with_span(declaration.parent_type.location)
            .wrap_some();
    }
    let key = SelectableKey {
        entity,
        selectable: declaration.name.item.0,
    };
    match selectable_definitions(db, client, key.entity, key.selectable) {
        Definitions::Undefined => {
            return ValidationError::UnknownSelectable(UnknownSelectableError(key))
                .with_span(declaration.name.location)
                .wrap_some();
        }
        Definitions::Unique(_) | Definitions::Duplicated | Definitions::ProtocolLookupFailed(_) => {
        }
    }
    let entity = entity_named(db, client, key.entity).as_ref().ok()?;
    let selectable = selectable_named(db, client, key.entity, key.selectable)
        .as_ref()
        .ok()?;
    let location = |error: &EntrypointNotFetchableError<_>| match error {
        EntrypointNotFetchableError::ProtocolSelectable(_) => declaration.name.location,
        EntrypointNotFetchableError::Entity(_) => declaration.parent_type.location,
    };
    entrypoint_fetch_plan(db, client, entity, selectable)
        .err()
        .map(|error| {
            let span = location(error.reference());
            ValidationError::EntrypointNotFetchable(error).with_span(span)
        })
}

/// The validation errors of `selection_set`, a selection set on `parent`, in literal-relative
/// spans. Each branch stops at its first error; sibling selections are still checked.
fn validate_selection_set<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    variables: Option<&'a WithSpan<VariableDeclarationList>>,
    parent: EntityName,
    selection_set: &'a SelectionSet,
) -> impl Iterator<Item = WithSpan<ProfileValidationError<TProfile>>> + 'a {
    let selections = || {
        selection_set
            .0
            .iter()
            .filter_map(|slot| slot.item.item.as_ref())
    };
    let duplicates = selections()
        .scan(BTreeSet::new(), |seen, selection| {
            let name = selection
                .item
                .reader_alias
                .as_ref()
                .unwrap_or(selection.item.name.reference());
            (!seen.insert(name.item.0))
                .then(|| {
                    ValidationError::DuplicateResponseName(DuplicateResponseName(name.item.0))
                        .with_span(name.location)
                })
                .wrap_some()
        })
        .flatten();
    duplicates.chain(selections().flat_map(move |selection| {
        validate_selection(db, client, variables, parent, selection.item.reference())
    }))
}

fn validate_selection<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    variables: Option<&'a WithSpan<VariableDeclarationList>>,
    parent: EntityName,
    selection: &'a Selection,
) -> ValidationErrors<'a, TProfile> {
    let key = SelectableKey {
        entity: parent,
        selectable: selection.name.item.0.unchecked_conversion(),
    };
    match selectable_definitions(db, client, key.entity, key.selectable) {
        Definitions::Undefined => std::iter::once(
            ValidationError::UnknownSelectable(UnknownSelectableError(key))
                .with_span(selection.name.location),
        )
        .boxed(),
        Definitions::Unique(selectable) => {
            let arguments = match selectable.kind {
                SelectableKind::ClientField | SelectableKind::ClientPointer(_) => {
                    field_argument_errors(db, client, variables, key, selection)
                }
                SelectableKind::Server(_) => TProfile::NetworkProtocol::selection_argument_errors(
                    db,
                    client,
                    key,
                    selection.name.location,
                    selection.arguments.as_ref(),
                    variables,
                )
                .into_iter()
                .map(|error| error.map(ValidationError::Argument))
                .collect(),
            };
            let nested: ValidationErrors<'a, TProfile> =
                match (selection.selection_set.as_ref(), selectable.target_entity()) {
                    (None, None) => std::iter::empty().boxed(),
                    (Some(nested), None) => std::iter::once(
                        ValidationError::SelectionSetOnFieldWithoutTarget(
                            SelectionSetOnFieldWithoutTargetError(key),
                        )
                        .with_span(nested.location),
                    )
                    .boxed(),
                    (selection_set, Some(target)) => {
                        target_errors(db, client, variables, key, selection, selection_set, target)
                    }
                };
            arguments.into_iter().chain(nested).boxed()
        }
        Definitions::Duplicated | Definitions::ProtocolLookupFailed(_) => {
            std::iter::empty().boxed()
        }
    }
}

/// What a selection of a selectable whose target entity has this shape needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetShape {
    /// Selected whole: no selection set.
    Leaf,
    /// Never selected.
    Input,
    /// Selected through a selection set.
    Composite,
}

/// The shape of `entity` as a selection's target. A source entity is composite.
fn target_shape<TProfile: CompilationProfile>(entity: &ProfileEntity<TProfile>) -> TargetShape {
    match entity.origin.reference() {
        EntityOrigin::Source => TargetShape::Composite,
        EntityOrigin::Protocol(data) => match TProfile::NetworkProtocol::entity_shape(data) {
            EntityShape::Leaf => TargetShape::Leaf,
            EntityShape::Input => TargetShape::Input,
            EntityShape::Composite(_) => TargetShape::Composite,
        },
    }
}

/// The errors of `selection`, a selection of `key` whose target entity is `target`, and of its
/// `selection_set`: the target's shape decides whether it takes one.
fn target_errors<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    variables: Option<&'a WithSpan<VariableDeclarationList>>,
    key: SelectableKey,
    selection: &'a Selection,
    selection_set: Option<&'a WithSpan<SelectionSet>>,
    target: EntityName,
) -> ValidationErrors<'a, TProfile> {
    let Ok(entity) = entity_named(db, client, target) else {
        return match (selection_set, entity_definedness(db, client, target)) {
            (Some(nested), EntityDefinedness::Defined) => {
                validate_selection_set(db, client, variables, target, nested.item.reference())
                    .boxed()
            }
            (None, _) | (Some(_), EntityDefinedness::Undefined) => std::iter::empty().boxed(),
        };
    };
    match (target_shape::<TProfile>(entity), selection_set) {
        (TargetShape::Input, _) => {
            std::iter::once(ValidationError::InputTarget(target).with_span(selection.name.location))
                .boxed()
        }
        (TargetShape::Leaf, None) => std::iter::empty().boxed(),
        (TargetShape::Leaf, Some(nested)) => {
            std::iter::once(ValidationError::SelectionSetOnLeaf(key).with_span(nested.location))
                .boxed()
        }
        (TargetShape::Composite, None) => std::iter::once(
            ValidationError::MissingSelectionSet(key).with_span(selection.name.location),
        )
        .boxed(),
        (TargetShape::Composite, Some(nested)) => {
            validate_selection_set(db, client, variables, target, nested.item.reference()).boxed()
        }
    }
}

/// The errors of the arguments `selection` passes to the `field` declaration `field`, inside a
/// declaration whose variables are `variables`: each variable with no default it does not pass,
/// each argument the field does not declare, and the protocol's verdict on the values.
fn field_argument_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    variables: Option<&WithSpan<VariableDeclarationList>>,
    field: SelectableKey,
    selection: &Selection,
) -> Vec<WithSpan<ProfileValidationError<TProfile>>> {
    let Ok(declaration) = field_declaration(
        db,
        client,
        EntityNameAndSelectableName {
            parent_entity_name: field.entity,
            selectable_name: field.selectable,
        },
    ) else {
        return Vec::new();
    };
    let field_variables = declaration.declaration.variable_definitions.as_ref();
    let declared: Vec<&str> = field_variables
        .iter()
        .flat_map(|list| list.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
        .map(|variable| variable.item.name.item.0.item.0.lookup())
        .collect();
    let unknown = selection
        .arguments
        .iter()
        .flat_map(|arguments| arguments.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
        .filter(|argument| !declared.contains(&argument.item.name.item.0.lookup()))
        .map(|argument| {
            ValidationError::UnknownFieldArgument(UnknownFieldArgument {
                field,
                argument: argument.item.name.item.0,
            })
            .with_span(argument.item.name.location)
        });
    let values = TProfile::NetworkProtocol::field_argument_errors(
        db,
        client,
        field_variables,
        selection.arguments.as_ref(),
        variables,
    )
    .into_iter()
    .map(|error| error.map(ValidationError::Argument));
    missing_field_arguments(db, client, field, selection)
        .into_iter()
        .map(|missing| {
            ValidationError::MissingFieldArgument(missing).with_span(selection.name.location)
        })
        .chain(unknown)
        .chain(values)
        .collect()
}

/// The variables of the `field` declaration `field` that have no default and that `selection`
/// does not pass, in declaration order. Empty when the declaration cannot be read.
fn missing_field_arguments<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: SelectableKey,
    selection: &Selection,
) -> Vec<MissingFieldArgument> {
    let Ok(declaration) = field_declaration(
        db,
        client,
        EntityNameAndSelectableName {
            parent_entity_name: field.entity,
            selectable_name: field.selectable,
        },
    ) else {
        return Vec::new();
    };
    let passed: Vec<&str> = selection
        .arguments
        .iter()
        .flat_map(|arguments| arguments.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
        .map(|argument| argument.item.name.item.0.lookup())
        .collect();
    declaration
        .declaration
        .variable_definitions
        .iter()
        .flat_map(|list| list.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
        .filter(|variable| variable.item.default_value.is_none())
        .map(|variable| variable.item.name.item.0.item.0)
        .filter(|name| !passed.contains(&name.lookup()))
        .map(|variable| MissingFieldArgument { field, variable })
        .collect()
}
