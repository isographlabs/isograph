use std::path::PathBuf;

use boulton_lang_types::{
    FieldSelection::{LinkedField, ScalarField},
    Selection,
};
use boulton_schema::{
    merge_selection_set, ValidatedSchema, ValidatedSchemaResolverDefinitionInfo,
    ValidatedSelectionSetAndUnwraps,
};
use common_lang_types::{
    FieldDefinitionName, QueryOperationName, ResolverDefinitionPath, TypeWithFieldsId,
    TypeWithoutFieldsId, WithSpan,
};
use thiserror::Error;

pub(crate) fn print_all_query_texts(schema: &ValidatedSchema) -> Result<String, PrintError> {
    let query_type = schema.query_type.expect("Expect Query to be defined");
    let query = schema.schema_data.object(query_type);

    // Operations (i.e. what would've been written with the query/subscription/mutation
    // keywords in GraphQL) are (for now) all resolver fields on Query.
    for field_id in query.fields.iter() {
        let field = schema.field(*field_id);
        if field.parent_type_id == query_type.into() {
            if let Some(resolver_field) = field.field_type.as_resolver_field() {
                let _text = generate_query_text_for_resolver_on_query(schema, resolver_field)?;
            }
            continue;
        }
    }

    Ok("".into())
}

fn generate_query_text_for_resolver_on_query(
    schema: &ValidatedSchema,
    resolver_definition: &ValidatedSchemaResolverDefinitionInfo,
) -> Result<String, PrintError> {
    if let Some(ref selection_set) = resolver_definition.selection_set_and_unwraps {
        let mut query_text = String::new();
        let path = resolver_definition.resolver_definition_path;
        let field = schema.field(resolver_definition.field_id);
        let _output_file = generated_file_name(path, field.name);
        let query_name: QueryOperationName = field.name.into();

        write_query_text(&mut query_text, query_name, schema, selection_set)?;

        eprintln!("query_text: `{}`", query_text);

        Ok(query_text)
    } else {
        todo!("Unsupported: resolvers on query with no selection set")
    }
}

fn write_query_text(
    query_text: &mut String,
    query_name: QueryOperationName,
    schema: &ValidatedSchema,
    selection_set: &ValidatedSelectionSetAndUnwraps,
) -> Result<(), PrintError> {
    query_text.push_str(&format!("query {} {{\n", query_name));
    write_selections(
        query_text,
        schema,
        // TODO do not do this here, instead do it during validation, and topologically sort first
        &merge_selection_set(
            schema,
            schema
                .schema_data
                .object(schema.query_type.expect("expect query type to exist"))
                .into(),
            &selection_set,
        ),
        1,
    )?;
    query_text.push_str("}");
    Ok(())
}

#[derive(Debug, Error)]
pub enum PrintError {}

fn generated_file_name(path: ResolverDefinitionPath, field_name: FieldDefinitionName) -> PathBuf {
    PathBuf::from(format!("__generated_/{}__{}.boulton.js", path, field_name))
}

fn write_selections(
    query_text: &mut String,
    schema: &ValidatedSchema,
    items: &Vec<WithSpan<Selection<TypeWithoutFieldsId, TypeWithFieldsId>>>,
    indentation_level: u8,
) -> Result<(), PrintError> {
    for item in items.iter() {
        query_text.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
        match &item.item {
            Selection::Field(field) => match field {
                ScalarField(scalar_field) => {
                    if let Some(alias) = scalar_field.alias {
                        query_text.push_str(&format!("{}: ", alias));
                    }
                    let name = scalar_field.name.item;
                    query_text.push_str(&format!("{},\n", name));
                }
                LinkedField(linked_field) => {
                    if let Some(alias) = linked_field.alias {
                        query_text.push_str(&format!("{}: ", alias));
                    }
                    let name = linked_field.name.item;
                    query_text.push_str(&format!("{} {{\n", name));
                    write_selections(
                        query_text,
                        schema,
                        &linked_field.selection_set_and_unwraps.selection_set,
                        indentation_level + 1,
                    )?;
                    query_text
                        .push_str(&format!("{}}},\n", "  ".repeat(indentation_level as usize)));
                }
            },
        }
    }
    Ok(())
}
