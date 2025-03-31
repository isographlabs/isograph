use common_lang_types::{Location, WithLocation, WithSpan};
use isograph_lang_types::{
    DefinitionLocation, LinkedFieldSelection, ObjectSelectionDirectiveSet, ScalarFieldSelection,
    ScalarSelectionDirectiveSet, SelectionType, SelectionTypeContainingSelections,
    UnvalidatedScalarFieldSelection,
};
use isograph_schema::{
    get_all_errors_or_all_ok, ClientFieldOrPointer, OutputFormat, RefetchStrategy, SchemaObject,
    UnprocessedClientFieldItem, UnprocessedItem, UnvalidatedSchema, UseRefetchFieldRefetchStrategy,
    ValidateSchemaError, ValidateSchemaResult, ValidatedLinkedFieldAssociatedData,
    ValidatedLinkedFieldSelection, ValidatedScalarFieldSelection,
    ValidatedScalarSelectionAssociatedData, ValidatedSelection,
};

pub type ValidateSchemaResultWithMultipleErrors<T> =
    Result<T, Vec<WithLocation<ValidateSchemaError>>>;

pub(crate) fn add_selection_sets_to_client_selectables<TOutputFormat: OutputFormat>(
    schema: &mut UnvalidatedSchema<TOutputFormat>,
    unprocessed_items: Vec<UnprocessedItem>,
) -> ValidateSchemaResultWithMultipleErrors<()> {
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
            SelectionType::Object(_) => todo!("process unprocessed pointer item"),
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
fn process_unprocessed_client_field_item<TOutputFormat: OutputFormat>(
    schema: &mut UnvalidatedSchema<TOutputFormat>,
    unprocessed_item: UnprocessedClientFieldItem,
) -> ValidateSchemaResultWithMultipleErrors<()> {
    let client_field = schema.client_field(unprocessed_item.client_field_id);
    let parent_object = schema
        .server_field_data
        .object(client_field.parent_object_id);

    let new_selection_set = get_validated_selection_set(
        schema,
        unprocessed_item.reader_selection_set,
        parent_object,
        &client_field,
    )?;

    let refetch_strategy = get_validated_refetch_strategy(
        schema,
        unprocessed_item.refetch_strategy,
        parent_object,
        &client_field,
    )?;

    let client_field = schema.client_field_mut(unprocessed_item.client_field_id);

    client_field.reader_selection_set = new_selection_set;
    client_field.refetch_strategy = refetch_strategy;

    Ok(())
}

/// At this point, all selectables have been defined. So, we can validate the parsed
/// selection set by confirming:
/// - each selectable exists in the schema, and is selected correctly (e.g. client fields
///   as scalars, etc)
/// - validate loadability/selectability (e.g. client fields cannot be selected updatably), and
/// - include the selectable id in the associated data
fn get_validated_selection_set<TOutputFormat: OutputFormat>(
    schema: &UnvalidatedSchema<TOutputFormat>,
    selection_set: Vec<
        WithSpan<
            SelectionTypeContainingSelections<
                ScalarSelectionDirectiveSet,
                ObjectSelectionDirectiveSet,
            >,
        >,
    >,
    parent_object: &SchemaObject<TOutputFormat>,
    top_level_field_or_pointer: &impl ClientFieldOrPointer<
        ValidatedScalarSelectionAssociatedData,
        ValidatedLinkedFieldAssociatedData,
    >,
) -> ValidateSchemaResultWithMultipleErrors<Vec<WithSpan<ValidatedSelection>>> {
    get_all_errors_or_all_ok(selection_set.into_iter().map(|selection| {
        get_validated_selection(schema, selection, parent_object, top_level_field_or_pointer)
    }))
}

fn get_validated_selection<TOutputFormat: OutputFormat>(
    schema: &UnvalidatedSchema<TOutputFormat>,
    with_span: WithSpan<
        SelectionTypeContainingSelections<ScalarSelectionDirectiveSet, ObjectSelectionDirectiveSet>,
    >,
    selection_parent_object: &SchemaObject<TOutputFormat>,
    top_level_field_or_pointer: &impl ClientFieldOrPointer<
        ValidatedScalarSelectionAssociatedData,
        ValidatedLinkedFieldAssociatedData,
    >,
) -> ValidateSchemaResultWithMultipleErrors<WithSpan<ValidatedSelection>> {
    with_span.and_then(|selection| match selection {
        SelectionType::Scalar(scalar_selection) => Ok(SelectionType::Scalar(
            get_validated_scalar_selection(
                schema,
                selection_parent_object,
                top_level_field_or_pointer,
                scalar_selection,
            )
            .map_err(|e| vec![e])?,
        )),
        SelectionType::Object(object_selection) => {
            Ok(SelectionType::Object(get_validated_object_selection(
                schema,
                selection_parent_object,
                top_level_field_or_pointer,
                object_selection,
            )?))
        }
    })
}

fn get_validated_scalar_selection<TOutputFormat: OutputFormat>(
    schema: &UnvalidatedSchema<TOutputFormat>,
    selection_parent_object: &SchemaObject<TOutputFormat>,
    top_level_field_or_pointer: &impl ClientFieldOrPointer<
        ValidatedScalarSelectionAssociatedData,
        ValidatedLinkedFieldAssociatedData,
    >,
    scalar_selection: UnvalidatedScalarFieldSelection,
) -> ValidateSchemaResult<ValidatedScalarFieldSelection> {
    let location = selection_parent_object
        .encountered_fields
        .get(&scalar_selection.name.item.into())
        .ok_or_else(|| {
            WithLocation::new(
                ValidateSchemaError::SelectionTypeSelectionFieldDoesNotExist {
                    client_field_parent_type_name: top_level_field_or_pointer
                        .type_and_field()
                        .type_name,
                    client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                    field_parent_type_name: selection_parent_object.name,
                    field_name: scalar_selection.name.item.into(),
                    client_type: top_level_field_or_pointer.client_type().to_string(),
                },
                scalar_selection.name.location,
            )
        })?;

    let location = match *location {
        DefinitionLocation::Server(server_scalar_selectable_id) => {
            if matches!(
                scalar_selection.associated_data,
                ScalarSelectionDirectiveSet::Loadable(_)
            ) {
                return Err(WithLocation::new(
                    ValidateSchemaError::ServerFieldCannotBeSelectedLoadably {
                        server_field_name: scalar_selection.name.item.into(),
                    },
                    scalar_selection.name.location,
                ));
            }

            let selection = schema.server_field(server_scalar_selectable_id);
            if let Some((_, object_id)) = selection.target_server_entity.as_object() {
                let object = schema.server_field_data.object(*object_id.inner());

                return Err(WithLocation::new(
                    ValidateSchemaError::SelectionTypeSelectionFieldIsNotScalar {
                        client_field_parent_type_name: top_level_field_or_pointer
                            .type_and_field()
                            .type_name,
                        client_field_name: top_level_field_or_pointer.name().into(),
                        field_parent_type_name: selection_parent_object.name,
                        field_name: scalar_selection.name.item.into(),
                        target_type_name: object.name.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                        field_type: top_level_field_or_pointer.client_type(),
                    },
                    scalar_selection.name.location,
                ));
            }

            DefinitionLocation::Server(server_scalar_selectable_id)
        }
        DefinitionLocation::Client(client_type) => {
            let client_field_id = *client_type.as_scalar().ok_or_else(|| {
                WithLocation::new(
                    ValidateSchemaError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: top_level_field_or_pointer
                            .type_and_field()
                            .type_name,
                        client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                        field_parent_type_name: selection_parent_object.name,
                        field_name: scalar_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    scalar_selection.name.location,
                )
            })?;
            DefinitionLocation::Client(client_field_id)
        }
    };

    Ok(ScalarFieldSelection {
        name: scalar_selection.name,
        reader_alias: scalar_selection.reader_alias,
        associated_data: ValidatedScalarSelectionAssociatedData {
            location,
            selection_variant: scalar_selection.associated_data,
        },
        arguments: scalar_selection.arguments,
    })
}

fn get_validated_object_selection<TOutputFormat: OutputFormat>(
    schema: &UnvalidatedSchema<TOutputFormat>,
    selection_parent_object: &SchemaObject<TOutputFormat>,
    top_level_field_or_pointer: &impl ClientFieldOrPointer<
        ValidatedScalarSelectionAssociatedData,
        ValidatedLinkedFieldAssociatedData,
    >,
    object_selection: LinkedFieldSelection<
        ScalarSelectionDirectiveSet,
        ObjectSelectionDirectiveSet,
    >,
) -> ValidateSchemaResultWithMultipleErrors<ValidatedLinkedFieldSelection> {
    let location = selection_parent_object
        .encountered_fields
        .get(&object_selection.name.item.into())
        .ok_or_else(|| {
            vec![WithLocation::new(
                ValidateSchemaError::SelectionTypeSelectionFieldDoesNotExist {
                    client_field_parent_type_name: top_level_field_or_pointer
                        .type_and_field()
                        .type_name,
                    client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                    field_parent_type_name: selection_parent_object.name,
                    field_name: object_selection.name.item.into(),
                    client_type: top_level_field_or_pointer.client_type().to_string(),
                },
                object_selection.name.location,
            )]
        })?;

    let (location, new_parent_object_id) = match *location {
        DefinitionLocation::Server(server_scalar_selectable_id) => {
            let selection = schema.server_field(server_scalar_selectable_id);
            let new_parent_object_id = match &selection.target_server_entity {
                SelectionType::Scalar(scalar) => {
                    let scalar_object_name =
                        schema.server_field_data.scalar(*scalar.inner()).name.item;

                    return Err(vec![WithLocation::new(
                        ValidateSchemaError::SelectionTypeSelectionFieldIsScalar {
                            client_field_parent_type_name: top_level_field_or_pointer
                                .type_and_field()
                                .type_name,
                            client_field_name: top_level_field_or_pointer.name().into(),
                            field_parent_type_name: selection_parent_object.name,
                            field_name: object_selection.name.item.into(),
                            target_type_name: scalar_object_name.into(),
                            client_type: top_level_field_or_pointer.client_type().to_string(),
                        },
                        Location::generated(),
                    )]);
                }
                SelectionType::Object((_, object)) => object.inner(),
            };

            (
                DefinitionLocation::Server(server_scalar_selectable_id),
                *new_parent_object_id,
            )
        }
        DefinitionLocation::Client(client_type) => {
            let client_pointer_id = *client_type.as_object().ok_or_else(|| {
                vec![WithLocation::new(
                    ValidateSchemaError::SelectionTypeSelectionClientPointerSelectedAsScalar {
                        client_field_parent_type_name: top_level_field_or_pointer
                            .type_and_field()
                            .type_name,
                        client_field_name: top_level_field_or_pointer.type_and_field().field_name,
                        field_parent_type_name: selection_parent_object.name,
                        field_name: object_selection.name.item.into(),
                        client_type: top_level_field_or_pointer.client_type().to_string(),
                    },
                    Location::generated(),
                )]
            })?;
            let client_pointer = schema.client_pointer(client_pointer_id);

            (
                DefinitionLocation::Client(client_pointer_id),
                *client_pointer.to.inner(),
            )
        }
    };

    let new_parent_object = schema.server_field_data.object(new_parent_object_id);

    Ok(LinkedFieldSelection {
        name: object_selection.name,
        reader_alias: object_selection.reader_alias,
        associated_data: ValidatedLinkedFieldAssociatedData {
            parent_object_id: new_parent_object_id,
            field_id: location,
            selection_variant: object_selection.associated_data,
            concrete_type: new_parent_object.concrete_type,
        },
        arguments: object_selection.arguments,
        selection_set: get_validated_selection_set(
            schema,
            object_selection.selection_set,
            new_parent_object,
            top_level_field_or_pointer,
        )?,
    })
}

fn get_validated_refetch_strategy<TOutputFormat: OutputFormat>(
    schema: &UnvalidatedSchema<TOutputFormat>,
    refetch_strategy: Option<
        RefetchStrategy<ScalarSelectionDirectiveSet, ObjectSelectionDirectiveSet>,
    >,
    parent_object: &SchemaObject<TOutputFormat>,
    top_level_field_or_pointer: &impl ClientFieldOrPointer<
        ValidatedScalarSelectionAssociatedData,
        ValidatedLinkedFieldAssociatedData,
    >,
) -> ValidateSchemaResultWithMultipleErrors<
    Option<
        RefetchStrategy<ValidatedScalarSelectionAssociatedData, ValidatedLinkedFieldAssociatedData>,
    >,
> {
    match refetch_strategy {
        Some(RefetchStrategy::UseRefetchField(use_refetch_field_strategy)) => Ok(Some(
            RefetchStrategy::UseRefetchField(UseRefetchFieldRefetchStrategy {
                refetch_selection_set: get_validated_selection_set(
                    schema,
                    use_refetch_field_strategy.refetch_selection_set,
                    parent_object,
                    top_level_field_or_pointer,
                )?,
                root_fetchable_type: use_refetch_field_strategy.root_fetchable_type,
                generate_refetch_query: use_refetch_field_strategy.generate_refetch_query,
                refetch_query_name: use_refetch_field_strategy.refetch_query_name,
            }),
        )),
        None => Ok(None),
    }
}
