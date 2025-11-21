use crate::{
    IsographDatabase, MemoizedIsoLiteralError, NetworkProtocol, ObjectSelectableId,
    RefetchStrategy, ScalarSelectableId, SelectableNamedError, UseRefetchFieldRefetchStrategy,
    ValidatedObjectSelection, ValidatedScalarSelection, ValidatedSelection, selectable_named,
    server_scalar_selectable_named,
};
use common_lang_types::{
    Location, ParentObjectEntityNameAndSelectableName, SelectableName, ServerObjectEntityName,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostFix, ObjectSelection, ScalarSelection,
    ScalarSelectionDirectiveSet, SelectionType, SelectionTypePostFix,
    UnvalidatedScalarFieldSelection, UnvalidatedSelection,
};
use prelude::Postfix;
use thiserror::Error;

pub type ValidateAddSelectionSetsResultWithMultipleErrors<T, TNetworkProtocol> =
    Result<T, Vec<WithLocation<AddSelectionSetsError<TNetworkProtocol>>>>;

/// At this point, all selectables have been defined. So, we can validate the parsed
/// selection set by confirming:
/// - each selectable exists in the schema, and is selected correctly (e.g. client fields
///   as scalars, etc)
/// - validate loadability/selectability (e.g. client fields cannot be selected updatably), and
/// - include the selectable id in the associated data
pub fn get_validated_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<
    Vec<WithSpan<ValidatedSelection>>,
    TNetworkProtocol,
> {
    get_all_errors_or_all_ok(selection_set.into_iter().map(|selection| {
        get_validated_selection(
            db,
            selection,
            parent_object_entity_name,
            top_level_field_or_pointer,
        )
    }))
}

fn get_validated_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    with_span: WithSpan<UnvalidatedSelection>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<WithSpan<ValidatedSelection>, TNetworkProtocol>
{
    with_span.and_then(|selection| match selection {
        SelectionType::Scalar(scalar_selection) => get_validated_scalar_selection(
            db,
            parent_object_entity_name,
            top_level_field_or_pointer,
            scalar_selection,
        )
        .map_err(|e| vec![e])?
        .scalar_selected()
        .ok(),
        SelectionType::Object(object_selection) => get_validated_object_selection(
            db,
            parent_object_entity_name,
            top_level_field_or_pointer,
            object_selection,
        )?
        .object_selected()
        .ok(),
    })
}

fn get_validated_scalar_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
    scalar_selection: UnvalidatedScalarFieldSelection,
) -> AddSelectionSetsResult<ValidatedScalarSelection, TNetworkProtocol> {
    let type_and_field = match top_level_field_or_pointer {
        SelectionType::Scalar(s) => s,
        SelectionType::Object(o) => o,
    };

    let location = selectable_named(
        db,
        parent_object_entity_name,
        scalar_selection.name.item.into(),
    )
    .as_ref()
    .map_err(|e| WithLocation::new(e.clone().into(), Location::Generated))?
    .as_ref()
    .ok_or_else(|| {
        WithLocation::new(
            AddSelectionSetsError::SelectionTypeSelectionFieldDoesNotExist {
                client_field_parent_type_name: type_and_field.parent_object_entity_name,
                client_field_name: type_and_field.selectable_name,
                field_parent_type_name: parent_object_entity_name,
                field_name: scalar_selection.name.item.into(),
                client_type: top_level_field_or_pointer.client_type().to_string(),
            },
            scalar_selection.name.location,
        )
    })?;

    let associated_data = match location {
        DefinitionLocation::Server(server_selectable_id) => {
            // TODO encode this in types
            if matches!(
                scalar_selection.scalar_selection_directive_set,
                ScalarSelectionDirectiveSet::Loadable(_)
            ) {
                return Err(WithLocation::new(
                    AddSelectionSetsError::ServerFieldCannotBeSelectedLoadably {
                        server_field_name: scalar_selection.name.item.into(),
                    },
                    scalar_selection.name.location,
                ));
            }

            let server_scalar_selectable = *server_selectable_id
                .as_ref()
                .as_scalar_result()
                .as_ref()
                .map_err(|object_selectable| {
                    WithLocation::new(
                        AddSelectionSetsError::SelectionTypeSelectionFieldIsNotScalar {
                            client_field_parent_type_name: type_and_field.parent_object_entity_name,
                            client_field_name: type_and_field.selectable_name,
                            field_parent_type_name: parent_object_entity_name,
                            field_name: scalar_selection.name.item.into(),
                            target_type_name: (*object_selectable.target_object_entity.inner())
                                .into(),
                            client_type: top_level_field_or_pointer.client_type().to_string(),
                            field_type: top_level_field_or_pointer.client_type(),
                        },
                        scalar_selection.name.location,
                    )
                })?;

            (
                server_scalar_selectable.parent_object_entity_name,
                server_scalar_selectable.name.item,
            )
                .server_defined()
        }
        DefinitionLocation::Client(client_type) => {
            let client_scalar_selectable =
                *client_type.as_ref().as_scalar().as_ref().ok_or_else(|| {
                    WithLocation::new(
                    AddSelectionSetsError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: type_and_field.parent_object_entity_name,
                        client_field_name: type_and_field.selectable_name,
                        field_parent_type_name: parent_object_entity_name,
                        field_name: scalar_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    scalar_selection.name.location,
                )
                })?;
            (
                client_scalar_selectable.parent_object_entity_name,
                client_scalar_selectable.name.item,
            )
                .client_defined()
        }
    };

    Ok(ScalarSelection {
        name: scalar_selection.name,
        reader_alias: scalar_selection.reader_alias,
        associated_data,
        scalar_selection_directive_set: scalar_selection.scalar_selection_directive_set,
        arguments: scalar_selection.arguments,
    })
}

fn get_validated_object_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
    object_selection: ObjectSelection<(), ()>,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<ValidatedObjectSelection, TNetworkProtocol> {
    let type_and_field = match top_level_field_or_pointer {
        SelectionType::Scalar(s) => s,
        SelectionType::Object(o) => o,
    };

    // TODO this can be vastly simplified... it looks like we're looking up the same object
    // multiple times :) and the result we're returning might be in the parameters anyway.

    let selectable = selectable_named(
        db,
        parent_object_entity_name,
        object_selection.name.item.into(),
    )
    .as_ref()
    .map_err(|e| vec![WithLocation::new(e.clone().into(), Location::Generated)])?
    .as_ref()
    .ok_or_else(|| {
        vec![WithLocation::new(
            AddSelectionSetsError::SelectionTypeSelectionFieldDoesNotExist {
                client_field_parent_type_name: type_and_field.parent_object_entity_name,
                client_field_name: type_and_field.selectable_name,
                field_parent_type_name: parent_object_entity_name,
                field_name: object_selection.name.item.into(),
                client_type: top_level_field_or_pointer.client_type().to_string(),
            },
            object_selection.name.location,
        )]
    })?;

    let (associated_data, new_parent_object_entity_name) = match selectable {
        DefinitionLocation::Server(server_selectable) => {
            let server_object_selectable = *server_selectable
                .as_ref()
                .as_object_result()
                .as_ref()
                .map_err(|server_scalar_selectable| {
                    let server_scalar_selectable = server_scalar_selectable_named(
                        db,
                        server_scalar_selectable.parent_object_entity_name,
                        (server_scalar_selectable.name.item).into(),
                    )
                    .as_ref()
                    .expect(
                        "Expected validation to have succeeded. \
                            This is indicative of a bug in Isograph.",
                    )
                    .as_ref()
                    .expect(
                        "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                    );
                    let server_scalar_entity_name =
                        *server_scalar_selectable.target_scalar_entity.inner();

                    vec![WithLocation::new(
                        AddSelectionSetsError::SelectionTypeSelectionFieldIsScalar {
                            client_field_parent_type_name: type_and_field.parent_object_entity_name,
                            client_field_name: type_and_field.selectable_name,
                            field_parent_type_name: parent_object_entity_name,
                            field_name: object_selection.name.item.into(),
                            target_type_name: server_scalar_entity_name.into(),
                            client_type: top_level_field_or_pointer.client_type().to_string(),
                        },
                        Location::generated(),
                    )]
                })?;

            (
                (
                    server_object_selectable.parent_object_entity_name,
                    server_object_selectable.name.item,
                )
                    .server_defined(),
                *server_object_selectable.target_object_entity.inner(),
            )
        }
        DefinitionLocation::Client(client_type) => {
            let client_object_selectable =
                *client_type.as_ref().as_object().as_ref().ok_or_else(|| {
                    vec![WithLocation::new(
                    AddSelectionSetsError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: type_and_field.parent_object_entity_name,
                        client_field_name: type_and_field.selectable_name,
                        field_parent_type_name: parent_object_entity_name,
                        field_name: object_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    Location::generated(),
                )]
                })?;

            (
                (
                    client_object_selectable.parent_object_entity_name,
                    client_object_selectable.name.item,
                )
                    .client_defined(),
                *client_object_selectable.target_object_entity_name.inner(),
            )
        }
    };

    Ok(ObjectSelection {
        name: object_selection.name,
        reader_alias: object_selection.reader_alias,
        object_selection_directive_set: object_selection.object_selection_directive_set,
        associated_data,
        arguments: object_selection.arguments,
        selection_set: get_validated_selection_set(
            db,
            object_selection.selection_set,
            new_parent_object_entity_name,
            top_level_field_or_pointer,
        )?,
    })
}

pub fn get_validated_refetch_strategy<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    unvalidated_refetch_strategy: RefetchStrategy<(), ()>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<
    RefetchStrategy<ScalarSelectableId, ObjectSelectableId>,
    TNetworkProtocol,
> {
    match unvalidated_refetch_strategy {
        RefetchStrategy::UseRefetchField(use_refetch_field_strategy) => Ok(
            RefetchStrategy::UseRefetchField(UseRefetchFieldRefetchStrategy {
                refetch_selection_set: get_validated_selection_set(
                    db,
                    use_refetch_field_strategy.refetch_selection_set,
                    parent_object_entity_name,
                    top_level_field_or_pointer,
                )?,
                root_fetchable_type_name: use_refetch_field_strategy.root_fetchable_type_name,
                generate_refetch_query: use_refetch_field_strategy.generate_refetch_query,
            }),
        ),
        RefetchStrategy::RefetchFromRoot => Ok(RefetchStrategy::RefetchFromRoot),
    }
}

pub fn get_all_errors_or_all_ok<T, E>(
    items: impl Iterator<Item = Result<T, Vec<E>>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut oks = vec![];
    let mut errors = vec![];

    for item in items {
        match item {
            Ok(ok) => oks.push(ok),
            Err(e) => errors.extend(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

type AddSelectionSetsResult<T, TNetworkProtocol> =
    Result<T, WithLocation<AddSelectionSetsError<TNetworkProtocol>>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum AddSelectionSetsError<TNetworkProtocol: NetworkProtocol> {
    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected, but that \
        field does not exist on `{field_parent_type_name}`"
    )]
    SelectionTypeSelectionFieldDoesNotExist {
        client_field_parent_type_name: ServerObjectEntityName,
        client_field_name: SelectableName,
        field_parent_type_name: ServerObjectEntityName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a scalar, \
        but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    SelectionTypeSelectionFieldIsNotScalar {
        client_field_parent_type_name: ServerObjectEntityName,
        client_field_name: SelectableName,
        field_parent_type_name: ServerObjectEntityName,
        field_name: SelectableName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, \
        the field `{field_parent_type_name}.{field_name}` is selected as a linked field, \
        but that field's type is `{target_type_name}`, which is a scalar."
    )]
    SelectionTypeSelectionFieldIsScalar {
        client_field_parent_type_name: ServerObjectEntityName,
        client_field_name: SelectableName,
        field_parent_type_name: ServerObjectEntityName,
        field_name: SelectableName,
        target_type_name: UnvalidatedTypeName,
        client_type: String,
    },

    #[error(
        "In the client {client_type} `{client_field_parent_type_name}.{client_field_name}`, the \
        pointer `{field_parent_type_name}.{field_name}` is selected as a scalar. \
        However, client pointers can only be selected as linked fields."
    )]
    SelectionTypeSelectionClientPointerSelectedAsScalar {
        client_field_parent_type_name: ServerObjectEntityName,
        client_field_name: SelectableName,
        field_parent_type_name: ServerObjectEntityName,
        field_name: SelectableName,
        client_type: String,
    },

    #[error("`{server_field_name}` is a server field, and cannot be selected with `@loadable`")]
    ServerFieldCannotBeSelectedLoadably { server_field_name: SelectableName },

    #[error("{0}")]
    MemoizedIsoLiteralError(#[from] MemoizedIsoLiteralError<TNetworkProtocol>),

    #[error("{0}")]
    SelectableNamedError(#[from] SelectableNamedError<TNetworkProtocol>),
}
