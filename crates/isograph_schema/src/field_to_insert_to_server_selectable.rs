use common_lang_types::{
    SelectableName, ServerObjectEntityName, ServerSelectableName, UnvalidatedTypeName,
    VariableName, WithLocation, WithLocationPostfix,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLInputValueDefinition, GraphQLNamedTypeAnnotation, NameValuePair,
};
use isograph_lang_types::{
    ConstantValue, SelectionType, SelectionTypePostfix, TypeAnnotation, VariableDefinition,
};
use prelude::Postfix;
use thiserror::Error;

use crate::{
    FieldToInsert, IsographDatabase, NetworkProtocol, ServerEntityName, ServerObjectSelectable,
    ServerObjectSelectableVariant, ServerScalarSelectable, defined_entity,
};

pub type ScalarSelectionAndNonNullType<TNetworkProtocol> = (
    ServerScalarSelectable<TNetworkProtocol>,
    Option<GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
);

pub fn field_to_insert_to_server_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    server_field_to_insert: &WithLocation<FieldToInsert>,
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
                parent_object_entity_name,
                target_entity_type_name: *target_entity_type_name,
                selectable_name: server_field_to_insert.item.name.item,
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
    match selection_type {
        SelectionType::Scalar(scalar_entity_name) => (
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
        )
            .scalar_selected(),
        SelectionType::Object(object_entity_name) => {
            ServerObjectSelectable {
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
            }
            .object_selected()
        }
    }
    .ok()
}

#[derive(Clone, Error, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum FieldToInsertToServerSelectableError {
    #[error(
        "The field `{parent_object_entity_name}.{selectable_name}` has inner type `{target_entity_type_name}`, which does not exist"
    )]
    FieldTypenameDoesNotExist {
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: ServerSelectableName,
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

pub fn graphql_input_value_definition_to_variable_definition<TNetworkProtocol: NetworkProtocol>(
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
            convert_graphql_constant_value_to_isograph_constant_value(graphql_constant_value.item)
                .with_location(graphql_constant_value.location)
                .ok()
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

    VariableDefinition {
        name: input_value_definition.item.name.map(VariableName::from),
        type_,
        default_value,
    }
    .with_location(input_value_definition.location)
    .ok()
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
                    convert_graphql_constant_value_to_isograph_constant_value(x.item)
                        .with_location(x.location)
                })
                .collect::<Vec<_>>();
            ConstantValue::List(converted_list)
        }
        GraphQLConstantValue::Object(o) => {
            let converted_object = o
                .into_iter()
                .map(|name_value_pair| NameValuePair {
                    name: name_value_pair.name,
                    value: convert_graphql_constant_value_to_isograph_constant_value(
                        name_value_pair.value.item,
                    )
                    .with_location(name_value_pair.value.location),
                })
                .collect::<Vec<_>>();
            ConstantValue::Object(converted_object)
        }
    }
}
