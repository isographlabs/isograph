use std::fmt::Debug;

use common_lang_types::SelectableName;
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};

use isograph_lang_types::{
    DefinitionLocation, SelectionType, ServerEntityId, TypeAnnotation, UnionVariant,
};
use isograph_schema::{NetworkProtocol, Schema, ServerSelectableId};

pub(crate) fn format_parameter_type<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    type_: GraphQLTypeAnnotation<ServerEntityId>,
    indentation_level: u8,
) -> String {
    match type_ {
        GraphQLTypeAnnotation::Named(named_inner_type) => {
            format!(
                "{} | null | void",
                format_server_field_type(schema, named_inner_type.item, indentation_level)
            )
        }
        GraphQLTypeAnnotation::List(list) => {
            format!(
                "ReadonlyArray<{}> | null",
                format_server_field_type(schema, *list.inner(), indentation_level)
            )
        }
        GraphQLTypeAnnotation::NonNull(non_null) => match *non_null {
            GraphQLNonNullTypeAnnotation::Named(named_inner_type) => {
                format_server_field_type(schema, named_inner_type.item, indentation_level)
            }
            GraphQLNonNullTypeAnnotation::List(list) => {
                format!(
                    "ReadonlyArray<{}>",
                    format_server_field_type(schema, *list.inner(), indentation_level)
                )
            }
        },
    }
}

fn format_server_field_type<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    field: ServerEntityId,
    indentation_level: u8,
) -> String {
    match field {
        ServerEntityId::Object(object_entity_id) => {
            // TODO this is bad; we should never create a type containing all of the fields
            // on a given object. This is currently used for input objects, and we should
            // consider how to do this is a not obviously broken manner.
            let mut s = "{\n".to_string();
            for (name, server_selectable_id) in schema
                .server_entity_data
                .server_object_entity_extra_info
                .get(&object_entity_id)
                .expect("Expected object_entity_id to exist in server_object_entity_available_selectables")
                .selectables
                .iter()
                .filter_map(
                    |(name, field_definition_location)| match field_definition_location {
                        DefinitionLocation::Server(s) => Some((name, *s)),
                        DefinitionLocation::Client(_) => None,
                    },
                )
            {
                let field_type = format_field_definition(
                    schema,
                    name,
                    server_selectable_id,
                    indentation_level + 1,
                );
                s.push_str(&field_type)
            }
            s.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));
            s
        }
        ServerEntityId::Scalar(scalar_entity_id) => schema
            .server_entity_data
            .server_scalar_entity(scalar_entity_id)
            .javascript_name
            .to_string(),
    }
}

fn format_field_definition<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    name: &SelectableName,
    server_selectable_id: ServerSelectableId,
    indentation_level: u8,
) -> String {
    let (is_optional, selection_type) = match schema.server_selectable(server_selectable_id) {
        SelectionType::Scalar(scalar_selectable) => (
            is_nullable(&scalar_selectable.target_scalar_entity),
            scalar_selectable
                .target_scalar_entity
                .clone()
                .map(&mut SelectionType::Scalar),
        ),
        SelectionType::Object(object_selectable) => (
            is_nullable(&object_selectable.target_object_entity),
            object_selectable
                .target_object_entity
                .clone()
                .map(&mut SelectionType::Object),
        ),
    };

    format!(
        "{}readonly {}{}: {},\n",
        "  ".repeat(indentation_level as usize),
        name,
        if is_optional { "?" } else { "" },
        format_type_annotation(schema, &selection_type, indentation_level + 1),
    )
}

fn is_nullable<T: Ord + Debug>(type_annotation: &TypeAnnotation<T>) -> bool {
    match type_annotation {
        TypeAnnotation::Union(union) => union.nullable,
        TypeAnnotation::Plural(_) => false,
        TypeAnnotation::Scalar(_) => false,
    }
}

fn format_type_annotation<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    type_annotation: &TypeAnnotation<ServerEntityId>,
    indentation_level: u8,
) -> String {
    match &type_annotation {
        TypeAnnotation::Scalar(scalar) => {
            format_server_field_type(schema, *scalar, indentation_level + 1)
        }
        TypeAnnotation::Union(union_type_annotation) => {
            if union_type_annotation.variants.is_empty() {
                panic!("Unexpected union with not enough variants.");
            }

            let mut s = String::new();
            if union_type_annotation.variants.len() > 1 || union_type_annotation.nullable {
                s.push('(');
                for (index, variant) in union_type_annotation.variants.iter().enumerate() {
                    if index != 0 {
                        s.push_str(" | ");
                    }

                    match variant {
                        UnionVariant::Scalar(scalar) => {
                            s.push_str(&format_server_field_type(
                                schema,
                                *scalar,
                                indentation_level + 1,
                            ));
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            s.push_str(&format_type_annotation(
                                schema,
                                type_annotation,
                                indentation_level + 1,
                            ));
                            s.push('>');
                        }
                    }
                }
                if union_type_annotation.nullable {
                    s.push_str(" | null");
                }
                s.push(')');
                s
            } else {
                let variant = union_type_annotation
                    .variants
                    .first()
                    .expect("Expected variant to exist");
                match variant {
                    UnionVariant::Scalar(scalar) => {
                        format_server_field_type(schema, *scalar, indentation_level + 1)
                    }
                    UnionVariant::Plural(type_annotation) => {
                        format!(
                            "ReadonlyArray<{}>",
                            format_server_field_type(
                                schema,
                                *type_annotation.inner(),
                                indentation_level + 1
                            )
                        )
                    }
                }
            }
        }
        TypeAnnotation::Plural(type_annotation) => {
            format!(
                "ReadonlyArray<{}>",
                format_server_field_type(schema, *type_annotation.inner(), indentation_level + 1)
            )
        }
    }
}
