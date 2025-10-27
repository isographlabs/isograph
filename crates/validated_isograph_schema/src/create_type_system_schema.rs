use std::{collections::HashMap, ops::Deref};

use common_lang_types::{
    SelectableName, ServerObjectEntityName, UnvalidatedTypeName, VariableName, WithLocation,
};
use graphql_lang_types::{GraphQLConstantValue, GraphQLInputValueDefinition, NameValuePair};
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::{ConstantValue, SelectionType, TypeAnnotation, VariableDefinition};
use isograph_schema::{
    CreateAdditionalFieldsError, ExposeAsFieldToInsert, FieldToInsert, IsographDatabase,
    NetworkProtocol, Schema, ServerEntityName, ServerObjectSelectable,
    ServerObjectSelectableVariant, ServerScalarSelectable, UnprocessedClientFieldItem,
    UnprocessedClientPointerItem, defined_entity,
};
use pico_macros::legacy_memo;
use thiserror::Error;

#[legacy_memo]
#[allow(clippy::type_complexity)]
pub fn create_type_system_schema<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        // TODO combine these into one hashmap?
        HashMap<ServerObjectEntityName, Vec<ExposeAsFieldToInsert>>,
        HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
    ),
    CreateSchemaError<TNetworkProtocol>,
> {
    let memo_ref = TNetworkProtocol::parse_type_system_documents(db);
    let (items, _fetchable_types) = memo_ref
        .deref()
        .as_ref()
        .map_err(|e| CreateSchemaError::ParseAndProcessTypeSystemDocument { message: e.clone() })?;

    let mut field_queue = HashMap::new();
    let mut expose_as_field_queue = HashMap::new();

    for item in items.into_iter().flat_map(|x| x.as_ref().as_object()) {
        field_queue.insert(
            item.server_object_entity.item.name.item,
            item.fields_to_insert.clone(),
        );

        expose_as_field_queue.insert(
            item.server_object_entity.item.name.item,
            item.expose_as_fields_to_insert.clone(),
        );
    }

    Ok((expose_as_field_queue, field_queue))
}

/// Create a schema from the type system document, i.e. avoid parsing any
/// iso literals. It *does* set server fields. Parsing iso literals is done in a future step.
///
/// This is sufficient for some queries, like answering "Where is a server field defined."
#[legacy_memo]
#[allow(clippy::type_complexity)]
pub(crate) fn create_type_system_schema_with_server_selectables<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        Schema<TNetworkProtocol>,
        Vec<SelectionType<UnprocessedClientFieldItem, UnprocessedClientPointerItem>>,
    ),
    CreateSchemaError<TNetworkProtocol>,
> {
    let (expose_as_field_queue, field_queue) = create_type_system_schema(db).to_owned()?;

    let mut unvalidated_isograph_schema = Schema::new();

    process_field_queue(
        db,
        &mut unvalidated_isograph_schema,
        field_queue,
        &db.get_isograph_config().options,
    )?;

    // Step one: we can create client selectables. However, we must create all
    // client selectables before being able to create their selection sets, because
    // selection sets refer to client selectables. We hold onto these selection sets
    // (both reader selection sets and refetch selection sets) in the unprocess_items
    // vec, then process it later.
    let mut unprocessed_items = vec![];

    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            let unprocessed_scalar_item = unvalidated_isograph_schema.create_new_exposed_field(
                db,
                expose_as_field,
                parent_object_entity_name,
            )?;

            unprocessed_items.push(SelectionType::Scalar(unprocessed_scalar_item));
        }
    }

    Ok((unvalidated_isograph_schema, unprocessed_items))
}

/// Now that we have processed all objects and scalars, we can process fields (i.e.
/// selectables), as we have the knowledge of whether the field points to a scalar
/// or object.
///
/// For each field:
/// - insert it into to the parent object's encountered_fields
/// - append it to schema.server_fields
/// - if it is an id field, modify the parent object
fn process_field_queue<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    field_queue: HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
    options: &CompilerConfigOptions,
) -> Result<(), CreateSchemaError<TNetworkProtocol>> {
    for (parent_object_entity_name, field_definitions_to_insert) in field_queue {
        for server_field_to_insert in field_definitions_to_insert.into_iter() {
            let target_entity_type_name = server_field_to_insert.item.graphql_type.inner();

            let selection_type = defined_entity(db, *target_entity_type_name)
                .to_owned()
                .expect(
                    "Expected parsing to have succeeded. \
                    This is indicative of a bug in Isograph.",
                )
                .ok_or(CreateSchemaError::FieldTypenameDoesNotExist {
                    target_entity_type_name: *target_entity_type_name,
                })?;

            let arguments = server_field_to_insert
                .item
                .arguments
                // TODO don't clone
                .clone()
                .into_iter()
                .map(|input_value_definition| {
                    graphql_input_value_definition_to_variable_definition(
                        db,
                        input_value_definition,
                        parent_object_entity_name,
                        server_field_to_insert.item.name.item.into(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let description = server_field_to_insert.item.description.map(|d| d.item);

            match selection_type {
                SelectionType::Scalar(scalar_entity_name) => {
                    schema.insert_server_scalar_selectable(
                        ServerScalarSelectable {
                            description,
                            name: server_field_to_insert
                                .item
                                .name
                                .map(|x| x.unchecked_conversion()),
                            target_scalar_entity: TypeAnnotation::from_graphql_type_annotation(
                                server_field_to_insert.item.graphql_type.clone(),
                            )
                            .map(&mut |_| scalar_entity_name),
                            javascript_type_override: server_field_to_insert
                                .item
                                .javascript_type_override,
                            parent_object_entity_name,
                            arguments,
                            phantom_data: std::marker::PhantomData,
                        },
                        options,
                        server_field_to_insert
                            .item
                            .graphql_type
                            .inner_non_null_named_type(),
                    )?;
                }
                SelectionType::Object(object_entity_name) => {
                    schema
                        .insert_server_object_selectable(ServerObjectSelectable {
                            description,
                            name: server_field_to_insert.item.name.map(|x| x.unchecked_conversion()),
                            target_object_entity: TypeAnnotation::from_graphql_type_annotation(
                                server_field_to_insert.item.graphql_type.clone(),
                            )
                            .map(&mut |_| object_entity_name),
                            parent_object_entity_name,
                            arguments,
                            phantom_data: std::marker::PhantomData,
                            object_selectable_variant:
                                // TODO this is hacky
                                if server_field_to_insert.item.is_inline_fragment {
                                    ServerObjectSelectableVariant::InlineFragment
                                } else {
                                    ServerObjectSelectableVariant::LinkedField
                                }
                        })?;
                }
            }
        }
    }

    Ok(())
}

pub fn graphql_input_value_definition_to_variable_definition<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
    parent_type_name: ServerObjectEntityName,
    field_name: SelectableName,
) -> Result<WithLocation<VariableDefinition<ServerEntityName>>, CreateSchemaError<TNetworkProtocol>>
{
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok::<_, CreateSchemaError<TNetworkProtocol>>(WithLocation::new(
                convert_graphql_constant_value_to_isograph_constant_value(
                    graphql_constant_value.item,
                ),
                graphql_constant_value.location,
            ))
        })
        .transpose()?;

    let type_ = input_value_definition
        .item
        .type_
        .clone()
        .and_then(|input_type_name| {
            let entity_name: UnvalidatedTypeName =
                (*input_value_definition.item.type_.inner()).into();
            defined_entity(db, entity_name)
                .to_owned()
                .expect(
                    "Expected parsing to have succeeded. \
                    This is indicative of a bug in Isograph.",
                )
                .ok_or(CreateSchemaError::FieldArgumentTypeDoesNotExist {
                    argument_type: input_type_name.into(),
                    argument_name: input_value_definition.item.name.item.into(),
                    parent_type_name,
                    field_name,
                })
        })?;

    Ok(WithLocation::new(
        VariableDefinition {
            name: input_value_definition.item.name.map(VariableName::from),
            type_,
            default_value,
        },
        input_value_definition.location,
    ))
}

fn convert_graphql_constant_value_to_isograph_constant_value(
    graphql_constant_value: GraphQLConstantValue,
) -> ConstantValue {
    match graphql_constant_value {
        GraphQLConstantValue::Int(i) => ConstantValue::Integer(i),
        GraphQLConstantValue::Boolean(b) => ConstantValue::Boolean(b),
        GraphQLConstantValue::String(s) => ConstantValue::String(s),
        GraphQLConstantValue::Float(f) => ConstantValue::Float(f),
        GraphQLConstantValue::Null => ConstantValue::Null,
        GraphQLConstantValue::Enum(e) => ConstantValue::Enum(e),
        GraphQLConstantValue::List(l) => {
            let converted_list = l
                .into_iter()
                .map(|x| {
                    WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(x.item),
                        x.location,
                    )
                })
                .collect::<Vec<_>>();
            ConstantValue::List(converted_list)
        }
        GraphQLConstantValue::Object(o) => {
            let converted_object = o
                .into_iter()
                .map(|name_value_pair| NameValuePair {
                    name: name_value_pair.name,
                    value: WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(
                            name_value_pair.value.item,
                        ),
                        name_value_pair.value.location,
                    ),
                })
                .collect::<Vec<_>>();
            ConstantValue::Object(converted_object)
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CreateSchemaError<TNetworkProtocol: NetworkProtocol + 'static> {
    #[error("{message}")]
    ParseAndProcessTypeSystemDocument {
        message: TNetworkProtocol::ParseTypeSystemDocumentsError,
    },

    #[error("{}", message)]
    CreateAdditionalFields {
        message: CreateAdditionalFieldsError<TNetworkProtocol>,
    },

    #[error(
        "The argument `{argument_name}` on field `{parent_type_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_type_name: ServerObjectEntityName,
        field_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },

    #[error("This field has type `{target_entity_type_name}`, which does not exist")]
    FieldTypenameDoesNotExist {
        target_entity_type_name: UnvalidatedTypeName,
    },
}

impl<TNetworkProtocol: NetworkProtocol + 'static>
    From<CreateAdditionalFieldsError<TNetworkProtocol>> for CreateSchemaError<TNetworkProtocol>
{
    fn from(value: CreateAdditionalFieldsError<TNetworkProtocol>) -> Self {
        CreateSchemaError::CreateAdditionalFields { message: value }
    }
}
