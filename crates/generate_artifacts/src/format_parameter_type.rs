use common_lang_types::SelectableFieldName;
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};

use isograph_lang_types::{
    DefinitionLocation, SelectableServerFieldId, SelectionType, ServerFieldId, TypeAnnotation,
    UnionVariant,
};
use isograph_schema::{ClientFieldOrPointerId, OutputFormat, ValidatedSchema};

pub(crate) fn format_parameter_type<TOutputFormat: OutputFormat>(
    schema: &ValidatedSchema<TOutputFormat>,
    type_: GraphQLTypeAnnotation<SelectableServerFieldId>,
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

fn format_server_field_type<TOutputFormat: OutputFormat>(
    schema: &ValidatedSchema<TOutputFormat>,
    field: SelectableServerFieldId,
    indentation_level: u8,
) -> String {
    match field {
        SelectableServerFieldId::Object(object_id) => {
            let mut s = "{\n".to_string();
            for (name, field_definition) in schema
                .server_field_data
                .object(object_id)
                .encountered_fields
                .iter()
                .filter(|x| matches!(
                    x.1,
                    DefinitionLocation::Server(server_field_id) if !schema.server_field(*server_field_id).is_discriminator),
                )
            {
                let field_type =
                    format_field_definition(schema, name, field_definition, indentation_level + 1);
                s.push_str(&field_type)
            }
            s.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));
            s
        }
        SelectableServerFieldId::Scalar(scalar_id) => schema
            .server_field_data
            .scalar(scalar_id)
            .javascript_name
            .to_string(),
    }
}

fn format_field_definition<TOutputFormat: OutputFormat>(
    schema: &ValidatedSchema<TOutputFormat>,
    name: &SelectableFieldName,
    type_: &DefinitionLocation<ServerFieldId, ClientFieldOrPointerId>,
    indentation_level: u8,
) -> String {
    match type_ {
        DefinitionLocation::Server(server_field_id) => {
            let type_annotation = match &schema.server_field(*server_field_id).associated_data {
                SelectionType::Object(associated_data) => associated_data
                    .type_name
                    .clone()
                    .map(&mut SelectionType::Object),
                SelectionType::Scalar(type_name) => {
                    type_name.clone().map(&mut SelectionType::Scalar)
                }
            };
            let is_optional = match &type_annotation {
                TypeAnnotation::Union(union) => union.nullable,
                TypeAnnotation::Plural(_) => false,
                TypeAnnotation::Scalar(_) => false,
            };

            format!(
                "{}readonly {}{}: {},\n",
                "  ".repeat(indentation_level as usize),
                name,
                if is_optional { "?" } else { "" },
                format_type_annotation(schema, &type_annotation, indentation_level + 1),
            )
        }
        DefinitionLocation::Client(_) => {
            panic!(
                "Unexpected object. Client fields are not supported as parameters, yet. \
                This is indicative of an unimplemented feature in Isograph."
            )
        }
    }
}

fn format_type_annotation<TOutputFormat: OutputFormat>(
    schema: &ValidatedSchema<TOutputFormat>,
    type_annotation: &TypeAnnotation<SelectableServerFieldId>,
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
                                type_annotation.inner(),
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
                format_server_field_type(schema, type_annotation.inner(), indentation_level + 1)
            )
        }
    }
}
