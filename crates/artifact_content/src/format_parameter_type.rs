use common_lang_types::{EntityName, SelectableName};

use isograph_lang_types::{SelectionType, TypeAnnotationDeclaration, UnionVariant};
use isograph_schema::{
    IsographDatabase, MemoRefServerSelectable, NetworkProtocol, server_entity_named,
    server_selectables_map_for_entity,
};
use prelude::Postfix;

pub(crate) fn format_parameter_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    type_: &TypeAnnotationDeclaration,
    indentation_level: u8,
) -> String {
    match type_ {
        TypeAnnotationDeclaration::Scalar(entity_name_wrapper) => {
            format_server_field_scalar_type(db, entity_name_wrapper.0, indentation_level)
        }
        TypeAnnotationDeclaration::Union(union_type_annotation) => {
            let mut s = String::new();
            let count = union_type_annotation.variants.len();
            for (index, variant) in union_type_annotation.variants.iter().enumerate() {
                let add_pipe = union_type_annotation.nullable || (index != count - 1);
                match variant {
                    UnionVariant::Scalar(entity_name_wrapper) => {
                        s.push_str(&format_server_field_scalar_type(
                            db,
                            entity_name_wrapper.0,
                            indentation_level,
                        ));
                    }
                    UnionVariant::Plural(p) => {
                        s.push_str("ReadonlyArray<");
                        s.push_str(&format_parameter_type(
                            db,
                            p.item.reference(),
                            indentation_level,
                        ));
                        s.push('>');
                    }
                }
                if add_pipe {
                    s.push_str(" | ");
                }
            }

            if union_type_annotation.nullable {
                s.push_str("null | void");
            }

            s
        }
        TypeAnnotationDeclaration::Plural(plural) => {
            let mut s = String::new();
            s.push_str("ReadonlyArray<");
            s.push_str(&format_parameter_type(
                db,
                plural.item.reference(),
                indentation_level,
            ));
            s.push('>');
            s
        }
    }
}

fn format_server_field_scalar_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: EntityName,
    indentation_level: u8,
) -> String {
    let entity = server_entity_named(db, entity_name)
        .as_ref()
        .expect(
            "Expected parsing to not have failed. \
            This is indicative of a bug in Isograph.",
        )
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

    match entity.lookup(db).selection_info {
        SelectionType::Object(_is_concrete) => {
            // TODO this is bad; we should never create a type containing all of the fields
            // on a given object. This is currently used for input objects, and we should
            // consider how to do this is a not obviously broken manner.
            let mut s = "{\n".to_string();

            for (name, server_selectable) in server_selectables_map_for_entity(db, entity_name)
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
        SelectionType::Scalar(s) => s.to_string(),
    }
}

fn format_field_definition<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    name: &SelectableName,
    server_selectable: MemoRefServerSelectable<TNetworkProtocol>,
    indentation_level: u8,
) -> String {
    let server_selectable = server_selectable.lookup(db);
    let is_optional = is_nullable(server_selectable.target_entity_name.reference());
    let target_type_annotation = server_selectable.target_entity_name.clone();

    format!(
        "{}readonly {}{}: {},\n",
        "  ".repeat(indentation_level as usize),
        name,
        if is_optional { "?" } else { "" },
        format_type_annotation(
            db,
            target_type_annotation.reference(),
            indentation_level + 1
        ),
    )
}

fn is_nullable(type_annotation: &TypeAnnotationDeclaration) -> bool {
    match type_annotation {
        TypeAnnotationDeclaration::Union(union) => union.nullable,
        TypeAnnotationDeclaration::Plural(_) => false,
        TypeAnnotationDeclaration::Scalar(_) => false,
    }
}

fn format_type_annotation<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    type_annotation: &TypeAnnotationDeclaration,
    indentation_level: u8,
) -> String {
    match type_annotation.reference() {
        TypeAnnotationDeclaration::Scalar(scalar) => {
            format_server_field_scalar_type(db, scalar.0, indentation_level + 1)
        }
        TypeAnnotationDeclaration::Union(union_type_annotation) => {
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
                            s.push_str(&format_server_field_scalar_type(
                                db,
                                scalar.0,
                                indentation_level + 1,
                            ));
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            s.push_str(&format_type_annotation(
                                db,
                                type_annotation.item.reference(),
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
                        format_server_field_scalar_type(db, scalar.0, indentation_level + 1)
                    }
                    UnionVariant::Plural(type_annotation) => {
                        format!(
                            "ReadonlyArray<{}>",
                            format_server_field_scalar_type(
                                db,
                                type_annotation.item.inner().0,
                                indentation_level + 1
                            )
                        )
                    }
                }
            }
        }
        TypeAnnotationDeclaration::Plural(type_annotation) => {
            format!(
                "ReadonlyArray<{}>",
                format_server_field_scalar_type(
                    db,
                    type_annotation.item.inner().0,
                    indentation_level + 1
                )
            )
        }
    }
}
