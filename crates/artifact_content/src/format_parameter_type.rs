use std::fmt::Debug;

use common_lang_types::SelectableName;
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};

use isograph_lang_types::{SelectionType, TypeAnnotation, UnionVariant};
use isograph_schema::{
    IsographDatabase, MemoRefServerSelectable, NetworkProtocol, ServerEntityName,
    server_object_selectable_named, server_scalar_entity_javascript_name,
    server_scalar_selectable_named, server_selectables_map_for_entity,
};
use prelude::Postfix;

pub(crate) fn format_parameter_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    type_: GraphQLTypeAnnotation<ServerEntityName>,
    indentation_level: u8,
) -> String {
    match type_ {
        GraphQLTypeAnnotation::Named(named_inner_type) => {
            format!(
                "{} | null | void",
                format_server_field_type(db, named_inner_type.item, indentation_level)
            )
        }
        GraphQLTypeAnnotation::List(list) => {
            format!(
                "ReadonlyArray<{}> | null",
                format_server_field_type(db, *list.inner(), indentation_level)
            )
        }
        GraphQLTypeAnnotation::NonNull(non_null) => match *non_null {
            GraphQLNonNullTypeAnnotation::Named(named_inner_type) => {
                format_server_field_type(db, named_inner_type.item, indentation_level)
            }
            GraphQLNonNullTypeAnnotation::List(list) => {
                format!(
                    "ReadonlyArray<{}>",
                    format_server_field_type(db, *list.inner(), indentation_level)
                )
            }
        },
    }
}

fn format_server_field_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    field: ServerEntityName,
    indentation_level: u8,
) -> String {
    match field {
        ServerEntityName::Object(parent_object_entity_name) => {
            // TODO this is bad; we should never create a type containing all of the fields
            // on a given object. This is currently used for input objects, and we should
            // consider how to do this is a not obviously broken manner.
            let mut s = "{\n".to_string();

            for (name, server_selectable) in
                server_selectables_map_for_entity(db, parent_object_entity_name)
                    .as_ref()
                    .expect(
                        "Expected type system document to be valid. \
                        This is indicative of a bug in Isograph.",
                    )
            {
                let field_type = format_field_definition(
                    db,
                    name,
                    server_selectable.dereference(),
                    indentation_level + 1,
                );
                s.push_str(&field_type)
            }

            s.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));
            s
        }
        ServerEntityName::Scalar(scalar_entity_name) => {
            server_scalar_entity_javascript_name(db, scalar_entity_name)
                .as_ref()
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

fn format_field_definition<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: &SelectableName,
    server_selectable: MemoRefServerSelectable<TNetworkProtocol>,
    indentation_level: u8,
) -> String {
    let (is_optional, selection_type) = match server_selectable {
        SelectionType::Scalar(server_scalar_selectable) => {
            let server_scalar_selectable = server_scalar_selectable.lookup(db);
            let parent_object_entity_name = server_scalar_selectable.parent_object_entity_name;
            let server_scalar_selectable_name = server_scalar_selectable.name.item;
            let server_scalar_selectable = server_scalar_selectable_named(
                db,
                parent_object_entity_name,
                server_scalar_selectable_name,
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
            )
            .lookup(db);

            (
                is_nullable(&server_scalar_selectable.target_scalar_entity),
                server_scalar_selectable
                    .target_scalar_entity
                    .clone()
                    .map(&mut SelectionType::Scalar),
            )
        }
        SelectionType::Object(server_object_selectable) => {
            let server_object_selectable = server_object_selectable.lookup(db);
            let parent_object_entity_name = server_object_selectable.parent_object_entity_name;
            let server_object_selectable_name = server_object_selectable.name.item;
            let server_object_selectable = server_object_selectable_named(
                db,
                parent_object_entity_name,
                server_object_selectable_name,
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
            )
            .lookup(db);
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
        format_type_annotation(db, &selection_type, indentation_level + 1),
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
    db: &IsographDatabase<TNetworkProtocol>,
    type_annotation: &TypeAnnotation<ServerEntityName>,
    indentation_level: u8,
) -> String {
    match &type_annotation {
        TypeAnnotation::Scalar(scalar) => {
            format_server_field_type(db, *scalar, indentation_level + 1)
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
                                *scalar,
                                indentation_level + 1,
                            ));
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            s.push_str(&format_type_annotation(
                                db,
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
                        format_server_field_type(db, *scalar, indentation_level + 1)
                    }
                    UnionVariant::Plural(type_annotation) => {
                        format!(
                            "ReadonlyArray<{}>",
                            format_server_field_type(
                                db,
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
                format_server_field_type(db, *type_annotation.inner(), indentation_level + 1)
            )
        }
    }
}
