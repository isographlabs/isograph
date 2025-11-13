use std::ops::Deref;

use crate::{
    ClientScalarOrObjectSelectable, IsographDatabase, MemoizedIsoLiteralError, NetworkProtocol,
    ObjectSelectableId, RefetchStrategy, ScalarSelectableId, Schema, SelectableNamedError,
    ServerObjectEntity, UnprocessedClientObjectSelectableSelectionSet,
    UnprocessedClientScalarSelectableSelectionSet, UnprocessedSelectionSet,
    UseRefetchFieldRefetchStrategy, ValidatedObjectSelection, ValidatedScalarSelection,
    ValidatedSelection, client_object_selectable_named, client_scalar_selectable_named,
    selectable_named, server_object_entity_named, server_scalar_selectable_named,
};
use common_lang_types::{
    Location, ParentObjectEntityNameAndSelectableName, SelectableName, ServerObjectEntityName,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use isograph_lang_types::{
    DefinitionLocation, ObjectSelection, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionType, UnvalidatedScalarFieldSelection, UnvalidatedSelection,
};
use thiserror::Error;

pub type ValidateAddSelectionSetsResultWithMultipleErrors<T, TNetworkProtocol> =
    Result<T, Vec<WithLocation<AddSelectionSetsError<TNetworkProtocol>>>>;

pub(crate) fn add_selection_sets_to_client_selectables<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    unprocessed_selection_sets: Vec<UnprocessedSelectionSet>,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<(), TNetworkProtocol> {
    let mut errors = vec![];
    for unprocessed_selection_set in unprocessed_selection_sets {
        match unprocessed_selection_set {
            SelectionType::Scalar(unprocessed_client_scalar_selection_set) => {
                if let Err(e) = process_unprocessed_client_field_item(
                    db,
                    schema,
                    unprocessed_client_scalar_selection_set,
                ) {
                    errors.extend(e)
                }
            }
            SelectionType::Object(unprocessed_client_object_selection_set) => {
                if let Err(e) = process_unprocessed_client_pointer_item(
                    db,
                    schema,
                    unprocessed_client_object_selection_set,
                ) {
                    errors.extend(e)
                }
            }
        }
    }
    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(())
    }
}

// TODO we should not be mutating items in the schema. Instead, we should be creating
// new items (the refetch and reader selection sets).
fn process_unprocessed_client_field_item<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    unprocessed_scalar_selection_set: UnprocessedClientScalarSelectableSelectionSet,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<(), TNetworkProtocol> {
    let memo_ref = client_scalar_selectable_named(
        db,
        unprocessed_scalar_selection_set.parent_object_entity_name,
        unprocessed_scalar_selection_set.client_scalar_selectable_name,
    );
    let client_scalar_selectable = memo_ref
        .deref()
        .as_ref()
        .map_err(|e| vec![WithLocation::new(e.clone().into(), Location::Generated)])?
        .as_ref()
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    let memo_ref =
        server_object_entity_named(db, client_scalar_selectable.parent_object_entity_name());
    let parent_object_entity = &memo_ref
        .deref()
        .as_ref()
        .expect(
            "Expected validation to have worked. \
            This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .item;

    let new_selection_set = get_validated_selection_set(
        db,
        unprocessed_scalar_selection_set.reader_selection_set,
        parent_object_entity,
        SelectionType::Scalar(client_scalar_selectable.type_and_field),
    )?;

    let refetch_strategy = get_validated_refetch_strategy(
        db,
        unprocessed_scalar_selection_set.refetch_strategy,
        parent_object_entity,
        SelectionType::Scalar(client_scalar_selectable.type_and_field),
    )?;

    let client_field = schema
        .client_scalar_selectable_mut(
            unprocessed_scalar_selection_set.parent_object_entity_name,
            unprocessed_scalar_selection_set.client_scalar_selectable_name,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    client_field.reader_selection_set = new_selection_set;
    client_field.refetch_strategy = refetch_strategy;

    Ok(())
}

// TODO we should not be mutating items in the schema. Instead, we should be creating
// new items (the refetch and reader selection sets).
fn process_unprocessed_client_pointer_item<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    unprocessed_client_object_selection_set: UnprocessedClientObjectSelectableSelectionSet,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<(), TNetworkProtocol> {
    let memo_ref = client_object_selectable_named(
        db,
        unprocessed_client_object_selection_set.parent_object_entity_name,
        unprocessed_client_object_selection_set.client_object_selectable_name,
    );
    let client_object_selectable = memo_ref
        .deref()
        .as_ref()
        .map_err(|e| vec![WithLocation::new(e.clone().into(), Location::Generated)])?
        .as_ref()
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    let memo_ref =
        server_object_entity_named(db, client_object_selectable.parent_object_entity_name());
    let parent_object_entity = &memo_ref
        .deref()
        .as_ref()
        .expect(
            "Expected validation to have worked. \
            This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .item;

    let new_selection_set = get_validated_selection_set(
        db,
        unprocessed_client_object_selection_set.reader_selection_set,
        parent_object_entity,
        SelectionType::Object(client_object_selectable.type_and_field),
    )?;

    let client_pointer = schema
        .client_object_selectable_mut(
            unprocessed_client_object_selection_set.parent_object_entity_name,
            unprocessed_client_object_selection_set.client_object_selectable_name,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    client_pointer.reader_selection_set = new_selection_set;

    Ok(())
}

/// At this point, all selectables have been defined. So, we can validate the parsed
/// selection set by confirming:
/// - each selectable exists in the schema, and is selected correctly (e.g. client fields
///   as scalars, etc)
/// - validate loadability/selectability (e.g. client fields cannot be selected updatably), and
/// - include the selectable id in the associated data
pub fn get_validated_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
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
            parent_object_entity,
            top_level_field_or_pointer,
        )
    }))
}

fn get_validated_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    with_span: WithSpan<UnvalidatedSelection>,
    selection_parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<WithSpan<ValidatedSelection>, TNetworkProtocol>
{
    with_span.and_then(|selection| match selection {
        SelectionType::Scalar(scalar_selection) => Ok(SelectionType::Scalar(
            get_validated_scalar_selection(
                db,
                selection_parent_object_entity,
                top_level_field_or_pointer,
                scalar_selection,
            )
            .map_err(|e| vec![e])?,
        )),
        SelectionType::Object(object_selection) => {
            Ok(SelectionType::Object(get_validated_object_selection(
                db,
                selection_parent_object_entity,
                top_level_field_or_pointer,
                object_selection,
            )?))
        }
    })
}

fn get_validated_scalar_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
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

    let location_memo_ref = selectable_named(
        db,
        selection_parent_object_entity.name.item,
        scalar_selection.name.item.into(),
    );
    let location = location_memo_ref
        .deref()
        .as_ref()
        .map_err(|e| WithLocation::new(e.clone().into(), Location::Generated))?
        .as_ref()
        .ok_or_else(|| {
            WithLocation::new(
                AddSelectionSetsError::SelectionTypeSelectionFieldDoesNotExist {
                    client_field_parent_type_name: type_and_field.parent_object_entity_name,
                    client_field_name: type_and_field.selectable_name,
                    field_parent_type_name: selection_parent_object_entity.name.item,
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
                            field_parent_type_name: selection_parent_object_entity.name.item,
                            field_name: scalar_selection.name.item.into(),
                            target_type_name: (*object_selectable.target_object_entity.inner())
                                .into(),
                            client_type: top_level_field_or_pointer.client_type().to_string(),
                            field_type: top_level_field_or_pointer.client_type(),
                        },
                        scalar_selection.name.location,
                    )
                })?;

            DefinitionLocation::Server((
                server_scalar_selectable.parent_object_entity_name,
                server_scalar_selectable.name.item,
            ))
        }
        DefinitionLocation::Client(client_type) => {
            let client_scalar_selectable =
                *client_type.as_ref().as_scalar().as_ref().ok_or_else(|| {
                    WithLocation::new(
                    AddSelectionSetsError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: type_and_field.parent_object_entity_name,
                        client_field_name: type_and_field.selectable_name,
                        field_parent_type_name: selection_parent_object_entity.name.item,
                        field_name: scalar_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    scalar_selection.name.location,
                )
                })?;
            DefinitionLocation::Client((
                client_scalar_selectable.parent_object_entity_name,
                client_scalar_selectable.name.item,
            ))
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
    selection_parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
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

    let selectable_memo_ref = selectable_named(
        db,
        selection_parent_object_entity.name.item,
        object_selection.name.item.into(),
    );
    let selectable = selectable_memo_ref
        .deref()
        .as_ref()
        .map_err(|e| vec![WithLocation::new(e.clone().into(), Location::Generated)])?
        .as_ref()
        .ok_or_else(|| {
            vec![WithLocation::new(
                AddSelectionSetsError::SelectionTypeSelectionFieldDoesNotExist {
                    client_field_parent_type_name: type_and_field.parent_object_entity_name,
                    client_field_name: type_and_field.selectable_name,
                    field_parent_type_name: selection_parent_object_entity.name.item,
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
                    let memo_ref = server_scalar_selectable_named(
                        db,
                        server_scalar_selectable.parent_object_entity_name,
                        (server_scalar_selectable.name.item).into(),
                    );
                    let server_scalar_selectable = memo_ref
                        .deref()
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
                            field_parent_type_name: selection_parent_object_entity.name.item,
                            field_name: object_selection.name.item.into(),
                            target_type_name: server_scalar_entity_name.into(),
                            client_type: top_level_field_or_pointer.client_type().to_string(),
                        },
                        Location::generated(),
                    )]
                })?;

            (
                DefinitionLocation::Server((
                    server_object_selectable.parent_object_entity_name,
                    server_object_selectable.name.item,
                )),
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
                        field_parent_type_name: selection_parent_object_entity.name.item,
                        field_name: object_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    Location::generated(),
                )]
                })?;

            (
                DefinitionLocation::Client((
                    client_object_selectable.parent_object_entity_name,
                    client_object_selectable.name.item,
                )),
                *client_object_selectable.target_object_entity_name.inner(),
            )
        }
    };

    let memo_ref = server_object_entity_named(db, new_parent_object_entity_name);
    let new_parent_object_entity = &memo_ref
        .deref()
        .as_ref()
        .expect(
            "Expected validation to have worked. \
            This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .item;

    Ok(ObjectSelection {
        name: object_selection.name,
        reader_alias: object_selection.reader_alias,
        object_selection_directive_set: object_selection.object_selection_directive_set,
        associated_data,
        arguments: object_selection.arguments,
        selection_set: get_validated_selection_set(
            db,
            object_selection.selection_set,
            new_parent_object_entity,
            top_level_field_or_pointer,
        )?,
    })
}

fn get_validated_refetch_strategy<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    refetch_strategy: Option<RefetchStrategy<(), ()>>,
    parent_object: &ServerObjectEntity<TNetworkProtocol>,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<
    Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,
    TNetworkProtocol,
> {
    match refetch_strategy {
        Some(RefetchStrategy::UseRefetchField(use_refetch_field_strategy)) => Ok(Some(
            RefetchStrategy::UseRefetchField(UseRefetchFieldRefetchStrategy {
                refetch_selection_set: get_validated_selection_set(
                    db,
                    use_refetch_field_strategy.refetch_selection_set,
                    parent_object,
                    top_level_field_or_pointer,
                )?,
                root_fetchable_type_name: use_refetch_field_strategy.root_fetchable_type_name,
                generate_refetch_query: use_refetch_field_strategy.generate_refetch_query,
            }),
        )),
        Some(RefetchStrategy::RefetchFromRoot) => Ok(Some(RefetchStrategy::RefetchFromRoot)),
        None => Ok(None),
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
