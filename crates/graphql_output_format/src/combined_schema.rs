use common_lang_types::{
    ArtifactFileName, ArtifactPathAndContent, DescriptionValue, SelectableFieldName,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{ClientFieldId, ClientPointerId, SelectionType, ServerFieldId};
use isograph_schema::{ClientFieldVariant, ClientType, FieldType, UserWrittenComponentVariant};
use lazy_static::lazy_static;
use std::fmt::Write;

use crate::{
    GraphQLSchemaOriginalDefinitionType, GraphqlSchemaObject, GraphqlSchemaScalar,
    ValidatedGraphqlSchema,
};

lazy_static! {
    pub static ref COMBINED_GRAPHQL_SCHEMA_FILE_NAME: ArtifactFileName =
        "combined-graphql-schema.graphql".intern().into();
}

pub(crate) fn build_combined_graphql_schema(
    schema: &ValidatedGraphqlSchema,
) -> ArtifactPathAndContent {
    let mut schema_content = initial_schema_content();

    for object in schema.server_field_data.server_objects.iter() {
        write_object(schema, &mut schema_content, object).expect("Expected writing to work");
    }

    for scalar in schema.server_field_data.server_scalars.iter() {
        // TODO avoid doing this
        if scalar.name.item == "NullDoesNotExistIfThisIsPrintedThisIsABug".intern().into() {
            continue;
        }
        write_scalar(&mut schema_content, scalar).expect("Expected writing scalar to work");
    }

    ArtifactPathAndContent {
        type_and_field: None,
        file_content: schema_content,
        file_name: *COMBINED_GRAPHQL_SCHEMA_FILE_NAME,
    }
}

fn initial_schema_content() -> String {
    String::from(
        "\"\"\"\n\
        A scalar representing a client field defined with the \n\
        iso function.\n\
        \"\"\"\n\
        scalar IsographClientField\n\n\
        \"\"\"\n\
        A scalar representing a client pointer defined with the \n\
        iso function.\n\
        \"\"\"\n\
        scalar IsographClientField\n\n\
        \"\"\"\n\
        A scalar representing a client field defined with the\n\
        iso function and annotated with @component.\n\
        \"\"\"\n\
        scalar IsographClientComponentField\n\n\
        \"\"\"\n\
        A scalar representing an imperatively loaded field.\n\
        This type is deprecated. Use @loadable fields instead.\n\
        \"\"\"\n\
        scalar IsographImperativelyLoadedField\n\n\
        \"\"\"\n\
        A scalar representing a pointer to an Isograph object (specifically,\n\
        a pointer to the object on which the link field was selected.)\n\
        \"\"\"\n\
        scalar IsographLinkField\n\n\
        ",
    )
}

fn write_object(
    schema: &ValidatedGraphqlSchema,
    schema_content: &mut String,
    object: &GraphqlSchemaObject,
) -> std::fmt::Result {
    let padding = "    ";
    let (colloquial_type_name, keyword) = object
        .output_associated_data
        .original_definition_type
        .colloquial_type_name();

    let object_name = object.name;

    let extra_description_content = match object.output_associated_data.original_definition_type {
        GraphQLSchemaOriginalDefinitionType::InputObject => None,
        GraphQLSchemaOriginalDefinitionType::Object => None,
        _ => Some(format!(
            "Note: {object_name} is represented here as an object. \n\
                However, in the original GraphQL schema, {object_name} \n\
                was defined as {colloquial_type_name} using the keyword `{keyword}`."
        )),
    };

    let keyword = if extra_description_content.is_some() {
        "type"
    } else {
        keyword
    };

    let object_description =
        formatted_description(object.description, "", extra_description_content);

    writeln!(
        schema_content,
        "{object_description}{keyword} {} {{",
        object.name
    )?;
    let typename = "__typename".intern().into();
    for (field_name, field_type) in object.encountered_fields.iter() {
        // HACK: skip typenames and fields starting with __
        if field_name == &typename || field_name.lookup().starts_with("__") {
            continue;
        }
        match field_type {
            FieldType::ServerField(server_field_id) => {
                write_server_field(schema, schema_content, padding, server_field_id, field_name)?;
            }
            FieldType::ClientField(client_type) => match client_type {
                ClientType::ClientField(client_field_id) => {
                    write_client_field(
                        schema,
                        schema_content,
                        padding,
                        client_field_id,
                        field_name,
                    )?;
                }
                ClientType::ClientPointer(pointer_id) => {
                    write_client_pointer(schema, schema_content, padding, pointer_id, field_name)?;
                }
            },
        }
    }

    writeln!(schema_content, "}}\n")?;

    Ok(())
}

fn write_client_field(
    schema: &ValidatedGraphqlSchema,
    schema_content: &mut String,
    padding: &str,
    client_field_id: &ClientFieldId,
    field_name: &SelectableFieldName,
) -> Result<(), std::fmt::Error> {
    let field = schema.client_field(*client_field_id);
    let (formatted_type, extra_content) = match field.variant {
        ClientFieldVariant::UserWritten(user_written_client_field_info) => {
            let extra_content = Some(format!(
                "{padding}Defined as `export const {}` in `{}`",
                user_written_client_field_info.const_export_name,
                user_written_client_field_info.file_path
            ));
            match user_written_client_field_info.user_written_component_variant {
                UserWrittenComponentVariant::Eager => ("IsographClientField!", extra_content),
                UserWrittenComponentVariant::Component => {
                    ("IsographClientComponentField!", extra_content)
                }
            }
        }
        ClientFieldVariant::ImperativelyLoadedField(_) => {
            ("IsographImperativelyLoadedField!", None)
        }
        ClientFieldVariant::Link => ("IsographLinkField!", None),
    };
    let description = formatted_description(field.description, padding, extra_content);
    writeln!(
        schema_content,
        "{description}{padding}{field_name}: {formatted_type}"
    )?;
    Ok(())
}

fn write_client_pointer(
    schema: &ValidatedGraphqlSchema,
    schema_content: &mut String,
    padding: &str,
    client_pointer_id: &ClientPointerId,
    field_name: &SelectableFieldName,
) -> Result<(), std::fmt::Error> {
    let field = schema.client_pointer(*client_pointer_id);
    let formatted_type = "IsographClientPointer!";
    let description = formatted_description(field.description, padding, None);
    writeln!(
        schema_content,
        "{description}{padding}{field_name}: {formatted_type}"
    )?;
    Ok(())
}

fn write_server_field(
    schema: &ValidatedGraphqlSchema,
    schema_content: &mut String,
    padding: &str,
    server_field_id: &ServerFieldId,
    field_name: &SelectableFieldName,
) -> Result<(), std::fmt::Error> {
    let server_field = schema.server_field(*server_field_id);
    let (type_annotation, description) = match &server_field.associated_data {
        SelectionType::Object(object) => {
            let mut description = None;
            let type_annotation = object.type_name.clone().map(&mut |target_object_id| {
                let target_object = schema.server_field_data.object(target_object_id);
                description = target_object.description;
                target_object.name.lookup()
            });
            (type_annotation, description)
        }
        SelectionType::Scalar(scalar) => {
            let mut description = None;
            let type_annotation = scalar.clone().map(&mut |target_scalar_id| {
                let target_scalar = schema.server_field_data.scalar(target_scalar_id);
                description = target_scalar.description.map(|x| x.item);
                target_scalar.name.item.lookup()
            });
            (type_annotation, description)
        }
    };
    let formatted_type = type_annotation.print_graphql();
    let description = formatted_description(description, padding, None);
    writeln!(
        schema_content,
        "{description}{padding}{field_name}: {formatted_type}"
    )?;
    Ok(())
}

fn formatted_description(
    description: Option<DescriptionValue>,
    padding: &str,
    extra_description_content: Option<String>,
) -> String {
    if description.is_none() && extra_description_content.is_none() {
        return "".to_string();
    }

    let mut description_output = format!("{padding}\"\"\"\n");
    if let Some(description) = description {
        description_output.push_str(padding);
        description_output.push_str(description.lookup());
    }

    let both_present = description.is_some() && extra_description_content.is_some();
    if both_present {
        description_output.push_str(&format!("\n{padding}-------\n"));
    }

    if let Some(extra_description_content) = extra_description_content {
        description_output.push_str(&extra_description_content);
    }

    description_output.push_str(&format!("\n{padding}\"\"\"\n"));

    description_output
}

fn write_scalar(schema_content: &mut String, scalar: &GraphqlSchemaScalar) -> std::fmt::Result {
    let description = formatted_description(scalar.description.map(|x| x.item), "", None);
    let scalar_name = scalar.name.item;
    writeln!(schema_content, "{description}scalar {scalar_name}")
}
