use std::fmt::Debug;
use std::ops::Deref;

use common_lang_types::SelectableName;
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};

use isograph_lang_types::{DefinitionLocation, SelectionType, TypeAnnotation, UnionVariant};
use isograph_schema::{
    IsographDatabase, NetworkProtocol, Schema, ServerEntityName, ServerSelectableId,
    server_object_selectable_named, server_scalar_entity_javascript_name,
};

pub(crate) fn format_parameter_type<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    type_: GraphQLTypeAnnotation<ServerEntityName>,
    indentation_level: u8,
) -> String {
    match type_ {
        GraphQLTypeAnnotation::Named(named_inner_type) => {
            format!(
                "{} | null | void",
                format_server_field_type(db, schema, named_inner_type.item, indentation_level)
            )
        }
        GraphQLTypeAnnotation::List(list) => {
            format!(
                "ReadonlyArray<{}> | null",
                format_server_field_type(db, schema, *list.inner(), indentation_level)
            )
        }
        GraphQLTypeAnnotation::NonNull(non_null) => match *non_null {
            GraphQLNonNullTypeAnnotation::Named(named_inner_type) => {
                format_server_field_type(db, schema, named_inner_type.item, indentation_level)
            }
            GraphQLNonNullTypeAnnotation::List(list) => {
                format!(
                    "ReadonlyArray<{}>",
                    format_server_field_type(db, schema, *list.inner(), indentation_level)
                )
            }
        },
    }
}

fn format_server_field_type<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    field: ServerEntityName,
    indentation_level: u8,
) -> String {
    match field {
        ServerEntityName::Object(object_entity_name) => {
            // TODO this is bad; we should never create a type containing all of the fields
            // on a given object. This is currently used for input objects, and we should
            // consider how to do this is a not obviously broken manner.
            let mut s = "{\n".to_string();
            for (name, server_selectable_id) in schema
                .server_entity_data
                .get(&object_entity_name)
                .expect("Expected object_entity_name to exist in server_object_entity_available_selectables")
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
        db,
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
        ServerEntityName::Scalar(scalar_entity_name) => {
            server_scalar_entity_javascript_name(db, scalar_entity_name)
                .to_owned()
                .expect(
                    "Expected parsing to not have failed. \
                    This is indicative of a bug in Isograph.",
                )
                .expect(
                    "Expected entity to exist. \
                    This is indicative of a bug in Isograph.",
                )
                .to_string()
        }
    }
}

fn format_field_definition<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    name: &SelectableName,
    server_selectable_id: ServerSelectableId,
    indentation_level: u8,
) -> String {
    let (is_optional, selection_type) = match server_selectable_id {
        SelectionType::Scalar((parent_object_entity_name, server_scalar_selectable_name)) => {
            let server_scalar_selectable = schema
                .server_scalar_selectable(parent_object_entity_name, server_scalar_selectable_name)
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                );
            (
                is_nullable(&server_scalar_selectable.target_scalar_entity),
                server_scalar_selectable
                    .target_scalar_entity
                    .clone()
                    .map(&mut SelectionType::Scalar),
            )
        }
        SelectionType::Object((parent_object_entity_name, server_object_selectable_name)) => {
            let memo_ref = server_object_selectable_named(
                db,
                parent_object_entity_name,
                server_object_selectable_name.into(),
            );
            let server_object_selectable = memo_ref
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
            (
                is_nullable(&server_object_selectable.target_object_entity),
                server_object_selectable
                    .target_object_entity
                    .clone()
                    .map(&mut SelectionType::Object),
            )
        }
    };

    format!(
        "{}readonly {}{}: {},\n",
        "  ".repeat(indentation_level as usize),
        name,
        if is_optional { "?" } else { "" },
        format_type_annotation(db, schema, &selection_type, indentation_level + 1),
    )
}

fn is_nullable<T: Ord + Debug>(type_annotation: &TypeAnnotation<T>) -> bool {
    match type_annotation {
        TypeAnnotation::Union(union) => union.nullable,
        TypeAnnotation::Plural(_) => false,
        TypeAnnotation::Scalar(_) => false,
    }
}

fn format_type_annotation<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    type_annotation: &TypeAnnotation<ServerEntityName>,
    indentation_level: u8,
) -> String {
    match &type_annotation {
        TypeAnnotation::Scalar(scalar) => {
            format_server_field_type(db, schema, *scalar, indentation_level + 1)
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
                                db,
                                schema,
                                *scalar,
                                indentation_level + 1,
                            ));
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            s.push_str(&format_type_annotation(
                                db,
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
                        format_server_field_type(db, schema, *scalar, indentation_level + 1)
                    }
                    UnionVariant::Plural(type_annotation) => {
                        format!(
                            "ReadonlyArray<{}>",
                            format_server_field_type(
                                db,
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
                format_server_field_type(
                    db,
                    schema,
                    *type_annotation.inner(),
                    indentation_level + 1
                )
            )
        }
    }
}
