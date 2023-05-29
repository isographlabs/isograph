use std::path::PathBuf;

use boulton_lang_types::{
    FieldSelection::{LinkedField, ScalarField},
    Selection,
};
use boulton_schema::{
    merge_selection_set, SchemaObject, SchemaTypeWithFields, ValidatedSchema,
    ValidatedSchemaResolverDefinitionInfo, ValidatedSelectionSetAndUnwraps,
};
use common_lang_types::{
    FieldDefinitionName, ObjectId, QueryOperationName, TypeWithFieldsId, TypeWithFieldsName,
    TypeWithoutFieldsId, WithSpan,
};
use thiserror::Error;

pub(crate) fn generate_query_artifacts(
    schema: &ValidatedSchema,
    project_root: &PathBuf,
) -> Result<String, PrintError> {
    let query_type = schema.query_type.expect("Expect Query to be defined");
    let query = schema.schema_data.object(query_type);

    write_artifacts(query_artifacts(query, schema, query_type), project_root)?;

    Ok("".into())
}

fn query_artifacts<'schema>(
    query: &'schema SchemaObject,
    schema: &'schema ValidatedSchema,
    query_type: ObjectId,
) -> impl Iterator<Item = Result<Artifact<'schema>, PrintError>> + 'schema {
    std::iter::from_fn(move || {
        for field_id in query.fields.iter() {
            let field = schema.field(*field_id);
            if field.parent_type_id == query_type.into() {
                if let Some(resolver_field) = field.field_type.as_resolver_field() {
                    Some(generate_query_text_for_resolver_on_query(
                        schema,
                        resolver_field,
                    ));
                }
                continue;
            }
        }
        None
    })
}

pub struct QueryText(pub String);

fn generate_query_text_for_resolver_on_query<'schema>(
    schema: &'schema ValidatedSchema,
    resolver_definition: &ValidatedSchemaResolverDefinitionInfo,
) -> Result<Artifact<'schema>, PrintError> {
    if let Some(ref selection_set) = resolver_definition.selection_set_and_unwraps {
        let mut query_text = String::new();
        let field = schema.field(resolver_definition.field_id);
        let query_name: QueryOperationName = field.name.into();

        write_query_text(&mut query_text, query_name, schema, selection_set)?;

        eprintln!("query_text: `{}`", query_text);

        Ok(Artifact::FetchableResolver(FetchableResolver {
            query_text: QueryText(query_text),
            query_name,
            parent_type: schema.schema_data.lookup_type_with_fields(
                schema
                    .query_type
                    .expect("expected query type to exist")
                    .into(),
            ),
        }))
    } else {
        // TODO convert to error
        todo!("Unsupported: resolvers on query with no selection set")
    }
}

pub enum Artifact<'schema> {
    FetchableResolver(FetchableResolver<'schema>),
    // Non-fetchable resolver
}

pub struct FetchableResolver<'schema> {
    pub query_text: QueryText,
    pub query_name: QueryOperationName,
    pub parent_type: SchemaTypeWithFields<'schema>,
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

fn generated_file_name(
    parent_type_name: TypeWithFieldsName,
    field_name: FieldDefinitionName,
) -> PathBuf {
    PathBuf::from(format!(
        "__generated_/{}__{}.boulton.js",
        parent_type_name, field_name
    ))
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

fn write_artifacts<'schema>(
    artifacts: impl Iterator<Item = Result<Artifact<'schema>, PrintError>> + 'schema,
    project_root: &PathBuf,
) -> Result<(), PrintError> {
    for artifact in artifacts {
        let artifact = artifact?;
        match artifact {
            Artifact::FetchableResolver(fetchable_resolver) => {
                let FetchableResolver {
                    query_text,
                    query_name,
                    parent_type,
                } = fetchable_resolver;
                // let generated_file_name = generated_file_name(parent_type, query_name.into())
                //     .display()
                //     .to_string();
                todo!()
            }
        }
    }
    Ok(())
}
