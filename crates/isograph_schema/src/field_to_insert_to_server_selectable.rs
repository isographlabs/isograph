use common_lang_types::{
    SelectableName, ServerObjectEntityName, UnvalidatedTypeName, VariableName, WithLocation,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLInputValueDefinition, GraphQLNamedTypeAnnotation, NameValuePair,
};
use isograph_lang_types::{ConstantValue, SelectionType, TypeAnnotation, VariableDefinition};
use thiserror::Error;

use crate::{
    FieldToInsert, IsographDatabase, NetworkProtocol, ServerEntityName, ServerObjectSelectable,
    ServerObjectSelectableVariant, ServerScalarSelectable, defined_entity,
};

pub type ScalarSelectionAndNonNullType<TNetworkProtocol> = (
    ServerScalarSelectable<TNetworkProtocol>,
    Option<GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
);

pub fn field_to_insert_to_server_selectable<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    server_field_to_insert: WithLocation<FieldToInsert>,
) -> Result<
    SelectionType<
        ScalarSelectionAndNonNullType<TNetworkProtocol>,
        ServerObjectSelectable<TNetworkProtocol>,
    >,
    FieldToInsertToServerSelectableError,
> {
    let target_entity_type_name = server_field_to_insert.item.graphql_type.inner();
    let target_entity_type_name_non_null = server_field_to_insert
        .item
        .graphql_type
        .inner_non_null_named_type()
        .cloned();

    let selection_type = defined_entity(db, *target_entity_type_name)
        .to_owned()
        .expect(
            "Expected parsing to have succeeded. \
                        This is indicative of a bug in Isograph.",
        )
        .ok_or(
            FieldToInsertToServerSelectableError::FieldTypenameDoesNotExist {
                target_entity_type_name: *target_entity_type_name,
            },
        )?;

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
    let selectable = match selection_type {
        SelectionType::Scalar(scalar_entity_name) => SelectionType::Scalar((
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
                javascript_type_override: server_field_to_insert.item.javascript_type_override,
                parent_object_entity_name,
                arguments,
                phantom_data: std::marker::PhantomData,
            },
            target_entity_type_name_non_null,
        )),
        SelectionType::Object(object_entity_name) => {
            SelectionType::Object(ServerObjectSelectable {
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
            })
        }
    };
    Ok(selectable)
}

#[derive(Clone, Error, PartialEq, Eq, Debug)]
pub enum FieldToInsertToServerSelectableError {
    #[error("This field has type `{target_entity_type_name}`, which does not exist")]
    FieldTypenameDoesNotExist {
        target_entity_type_name: UnvalidatedTypeName,
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
}

pub fn graphql_input_value_definition_to_variable_definition<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
    parent_type_name: ServerObjectEntityName,
    field_name: SelectableName,
) -> Result<WithLocation<VariableDefinition<ServerEntityName>>, FieldToInsertToServerSelectableError>
{
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok(WithLocation::new(
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
                .ok_or(
                    FieldToInsertToServerSelectableError::FieldArgumentTypeDoesNotExist {
                        argument_type: input_type_name.into(),
                        argument_name: input_value_definition.item.name.item.into(),
                        parent_type_name,
                        field_name,
                    },
                )
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
