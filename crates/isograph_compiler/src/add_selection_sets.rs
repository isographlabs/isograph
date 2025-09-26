use common_lang_types::{
    Location, SelectableName, ServerObjectEntityName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use isograph_lang_types::{
    DefinitionLocation, ObjectSelection, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionType, UnvalidatedScalarFieldSelection, UnvalidatedSelection,
};
use isograph_schema::{
    ClientScalarOrObjectSelectable, NetworkProtocol, ObjectSelectableId, RefetchStrategy,
    ScalarSelectableId, Schema, ServerObjectEntity, UnprocessedClientFieldItem,
    UnprocessedClientPointerItem, UnprocessedItem, UseRefetchFieldRefetchStrategy,
    ValidatedObjectSelection, ValidatedScalarSelection, ValidatedSelection,
};
use thiserror::Error;

pub type ValidateAddSelectionSetsResultWithMultipleErrors<T> =
    Result<T, Vec<WithLocation<AddSelectionSetsError>>>;

pub(crate) fn add_selection_sets_to_client_selectables<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    schema: &mut Schema<TNetworkProtocol>,
    unprocessed_items: Vec<UnprocessedItem>,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<()> {
    let mut errors = vec![];
    for unprocessed_item in unprocessed_items {
        match unprocessed_item {
            SelectionType::Scalar(unprocessed_client_field_item) => {
                if let Err(e) =
                    process_unprocessed_client_field_item(schema, unprocessed_client_field_item)
                {
                    errors.extend(e)
                }
            }
            SelectionType::Object(unprocessed_client_pointer_item) => {
                if let Err(e) =
                    process_unprocessed_client_pointer_item(schema, unprocessed_client_pointer_item)
                {
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
fn process_unprocessed_client_field_item<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &mut Schema<TNetworkProtocol>,
    unprocessed_item: UnprocessedClientFieldItem,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<()> {
    let client_field = schema
        .client_field(
            unprocessed_item.parent_object_entity_name,
            unprocessed_item.client_field_name,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );
    let parent_object = schema
        .server_entity_data
        .server_object_entity(client_field.parent_object_entity_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

    let new_selection_set = get_validated_selection_set(
        schema,
        unprocessed_item.reader_selection_set,
        parent_object,
        client_field.parent_object_entity_name,
        &client_field,
    )?;

    let refetch_strategy = get_validated_refetch_strategy(
        schema,
        unprocessed_item.refetch_strategy,
        parent_object,
        client_field.parent_object_entity_name,
        &client_field,
    )?;

    let client_field = schema
        .client_field_mut(
            unprocessed_item.parent_object_entity_name,
            unprocessed_item.client_field_name,
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
fn process_unprocessed_client_pointer_item<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &mut Schema<TNetworkProtocol>,
    unprocessed_item: UnprocessedClientPointerItem,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<()> {
    let client_pointer = schema
        .client_pointer(
            unprocessed_item.parent_object_entity_name,
            unprocessed_item.client_object_selectable_name,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );
    let parent_object = schema
        .server_entity_data
        .server_object_entity(client_pointer.parent_object_entity_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

    let new_selection_set = get_validated_selection_set(
        schema,
        unprocessed_item.reader_selection_set,
        parent_object,
        client_pointer.parent_object_entity_name,
        &client_pointer,
    )?;

    let client_pointer = schema
        .client_pointer_mut(
            unprocessed_item.parent_object_entity_name,
            unprocessed_item.client_object_selectable_name,
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
fn get_validated_selection_set<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &Schema<TNetworkProtocol>,
    selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    parent_object: &ServerObjectEntity<TNetworkProtocol>,
    selection_parent_object_name: ServerObjectEntityName,
    top_level_field_or_pointer: &impl ClientScalarOrObjectSelectable,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<Vec<WithSpan<ValidatedSelection>>> {
    get_all_errors_or_all_ok(selection_set.into_iter().map(|selection| {
        get_validated_selection(
            schema,
            selection,
            parent_object,
            selection_parent_object_name,
            top_level_field_or_pointer,
        )
    }))
}

fn get_validated_selection<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &Schema<TNetworkProtocol>,
    with_span: WithSpan<UnvalidatedSelection>,
    selection_parent_object: &ServerObjectEntity<TNetworkProtocol>,
    selection_parent_object_name: ServerObjectEntityName,
    top_level_field_or_pointer: &impl ClientScalarOrObjectSelectable,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<WithSpan<ValidatedSelection>> {
    with_span.and_then(|selection| match selection {
        SelectionType::Scalar(scalar_selection) => Ok(SelectionType::Scalar(
            get_validated_scalar_selection(
                schema,
                selection_parent_object,
                selection_parent_object_name,
                top_level_field_or_pointer,
                scalar_selection,
            )
            .map_err(|e| vec![e])?,
        )),
        SelectionType::Object(object_selection) => {
            Ok(SelectionType::Object(get_validated_object_selection(
                schema,
                selection_parent_object,
                selection_parent_object_name,
                top_level_field_or_pointer,
                object_selection,
            )?))
        }
    })
}

fn get_validated_scalar_selection<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &Schema<TNetworkProtocol>,
    selection_parent_object: &ServerObjectEntity<TNetworkProtocol>,
    selection_parent_object_name: ServerObjectEntityName,
    top_level_field_or_pointer: &impl ClientScalarOrObjectSelectable,
    scalar_selection: UnvalidatedScalarFieldSelection,
) -> AddSelectionSetsResult<ValidatedScalarSelection> {
    let location = schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(&selection_parent_object_name)
        .expect(
            "Expected selection_parent_object_id to exist \
            in server_object_entity_available_selectables",
        )
        .selectables
        .get(&scalar_selection.name.item.into())
        .ok_or_else(|| {
            WithLocation::new(
                AddSelectionSetsError::SelectionTypeSelectionFieldDoesNotExist {
                    client_field_parent_type_name: top_level_field_or_pointer
                        .type_and_field()
                        .type_name,
                    client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                    field_parent_type_name: selection_parent_object.name.item,
                    field_name: scalar_selection.name.item.into(),
                    client_type: top_level_field_or_pointer.client_type().to_string(),
                },
                scalar_selection.name.location,
            )
        })?;

    let associated_data = match *location {
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

            let server_scalar_selectable_id =
                *server_selectable_id.as_scalar_result().as_ref().map_err(
                    |(parent_object_entity_name, server_object_selectable_name)| {
                        let object_selectable = schema
                            .server_object_selectable(
                                *parent_object_entity_name,
                                *server_object_selectable_name,
                            )
                            .expect(
                                "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                            );
                        let object = schema
                            .server_entity_data
                            .server_object_entity(*object_selectable.target_object_entity.inner())
                            .expect(
                                "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                            );

                        WithLocation::new(
                            AddSelectionSetsError::SelectionTypeSelectionFieldIsNotScalar {
                                client_field_parent_type_name: top_level_field_or_pointer
                                    .type_and_field()
                                    .type_name,
                                client_field_name: top_level_field_or_pointer.name().into(),
                                field_parent_type_name: selection_parent_object.name.item,
                                field_name: scalar_selection.name.item.into(),
                                target_type_name: object.name.item.into(),
                                client_type: top_level_field_or_pointer.client_type().to_string(),
                                field_type: top_level_field_or_pointer.client_type(),
                            },
                            scalar_selection.name.location,
                        )
                    },
                )?;

            DefinitionLocation::Server(server_scalar_selectable_id)
        }
        DefinitionLocation::Client(client_type) => {
            let (parent_object_entity_name, client_field_name) =
                *client_type.as_scalar().as_ref().ok_or_else(|| {
                    WithLocation::new(
                    AddSelectionSetsError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: top_level_field_or_pointer
                            .type_and_field()
                            .type_name,
                        client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                        field_parent_type_name: selection_parent_object.name.item,
                        field_name: scalar_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    scalar_selection.name.location,
                )
                })?;
            DefinitionLocation::Client((parent_object_entity_name, client_field_name))
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

fn get_validated_object_selection<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &Schema<TNetworkProtocol>,
    selection_parent_object: &ServerObjectEntity<TNetworkProtocol>,
    selection_parent_object_name: ServerObjectEntityName,
    top_level_field_or_pointer: &impl ClientScalarOrObjectSelectable,
    object_selection: ObjectSelection<(), ()>,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<ValidatedObjectSelection> {
    let location = schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(&selection_parent_object_name)
        .expect(
            "Expected selection_parent_object_id to exist \
            in server_object_entity_available_selectables",
        )
        .selectables
        .get(&object_selection.name.item.into())
        .ok_or_else(|| {
            vec![WithLocation::new(
                AddSelectionSetsError::SelectionTypeSelectionFieldDoesNotExist {
                    client_field_parent_type_name: top_level_field_or_pointer
                        .type_and_field()
                        .type_name,
                    client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                    field_parent_type_name: selection_parent_object.name.item,
                    field_name: object_selection.name.item.into(),
                    client_type: top_level_field_or_pointer.client_type().to_string(),
                },
                object_selection.name.location,
            )]
        })?;

    let (associated_data, new_parent_object_entity_name) = match *location {
        DefinitionLocation::Server(server_selectable_id) => {
            let (parent_object_entity_name, server_object_selectable_name) =
                *server_selectable_id.as_object_result().as_ref().map_err(
                    |(parent_object_entity_name, server_scalar_selectable_name)| {
                        let server_scalar_selectable = schema
                            .server_scalar_selectable(
                                *parent_object_entity_name,
                                *server_scalar_selectable_name,
                            )
                            .expect(
                                "Expected selectable to exist. \
                                This is indicative of a bug in Isograph.",
                            );
                        let server_scalar = schema
                            .server_entity_data
                            .server_scalar_entity(
                                *server_scalar_selectable.target_scalar_entity.inner(),
                            )
                            .expect(
                                "Expected entity to exist. \
                                This is indicative of a bug in Isograph.",
                            );

                        vec![WithLocation::new(
                            AddSelectionSetsError::SelectionTypeSelectionFieldIsScalar {
                                client_field_parent_type_name: top_level_field_or_pointer
                                    .type_and_field()
                                    .type_name,
                                client_field_name: top_level_field_or_pointer.name().into(),
                                field_parent_type_name: selection_parent_object.name.item,
                                field_name: object_selection.name.item.into(),
                                target_type_name: server_scalar.name.item.into(),
                                client_type: top_level_field_or_pointer.client_type().to_string(),
                            },
                            Location::generated(),
                        )]
                    },
                )?;
            let server_object_selectable = schema
                .server_object_selectable(parent_object_entity_name, server_object_selectable_name)
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                );

            (
                DefinitionLocation::Server((
                    parent_object_entity_name,
                    server_object_selectable_name,
                )),
                *server_object_selectable.target_object_entity.inner(),
            )
        }
        DefinitionLocation::Client(client_type) => {
            let (parent_object_entity_name, client_pointer_name) =
                *client_type.as_object().as_ref().ok_or_else(|| {
                    vec![WithLocation::new(
                    AddSelectionSetsError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: top_level_field_or_pointer
                            .type_and_field()
                            .type_name,
                        client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                        field_parent_type_name: selection_parent_object.name.item,
                        field_name: object_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    Location::generated(),
                )]
                })?;
            let client_pointer = schema
                .client_pointer(parent_object_entity_name, client_pointer_name)
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                );

            (
                DefinitionLocation::Client((parent_object_entity_name, client_pointer_name)),
                *client_pointer.target_object_entity_name.inner(),
            )
        }
    };

    let new_parent_object = schema
        .server_entity_data
        .server_object_entity(new_parent_object_entity_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

    Ok(ObjectSelection {
        name: object_selection.name,
        reader_alias: object_selection.reader_alias,
        object_selection_directive_set: object_selection.object_selection_directive_set,
        associated_data,
        arguments: object_selection.arguments,
        selection_set: get_validated_selection_set(
            schema,
            object_selection.selection_set,
            new_parent_object,
            new_parent_object_entity_name,
            top_level_field_or_pointer,
        )?,
    })
}

fn get_validated_refetch_strategy<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &Schema<TNetworkProtocol>,
    refetch_strategy: Option<RefetchStrategy<(), ()>>,
    parent_object: &ServerObjectEntity<TNetworkProtocol>,
    selection_parent_object_name: ServerObjectEntityName,
    top_level_field_or_pointer: &impl ClientScalarOrObjectSelectable,
) -> ValidateAddSelectionSetsResultWithMultipleErrors<
    Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,
> {
    match refetch_strategy {
        Some(RefetchStrategy::UseRefetchField(use_refetch_field_strategy)) => Ok(Some(
            RefetchStrategy::UseRefetchField(UseRefetchFieldRefetchStrategy {
                refetch_selection_set: get_validated_selection_set(
                    schema,
                    use_refetch_field_strategy.refetch_selection_set,
                    parent_object,
                    selection_parent_object_name,
                    top_level_field_or_pointer,
                )?,
                root_fetchable_type_name: use_refetch_field_strategy.root_fetchable_type_name,
                generate_refetch_query: use_refetch_field_strategy.generate_refetch_query,
            }),
        )),
        Some(RefetchStrategy::RefetchFromRoot(root_operation_name)) => {
            Ok(Some(RefetchStrategy::RefetchFromRoot(root_operation_name)))
        }
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

type AddSelectionSetsResult<T> = Result<T, WithLocation<AddSelectionSetsError>>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum AddSelectionSetsError {
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
}
