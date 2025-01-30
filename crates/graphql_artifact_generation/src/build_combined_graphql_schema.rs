use common_lang_types::ArtifactPathAndContent;
use isograph_schema::{SchemaObject, ValidatedSchema};

pub(crate) fn build_combined_graphql_schema(schema: &ValidatedSchema) -> ArtifactPathAndContent {
    let mut schema_content = String::new();

    for object in schema.server_field_data.server_objects.iter() {
        write_object(&mut schema_content, &object)
    }

    ArtifactPathAndContent {
        type_and_field: None,
        file_name_prefix: todo!(),
        file_content: todo!(),
    }
}

fn write_object(schema_content: &mut String, object: &SchemaObject) {
    writeln!(schema_content, "type {} {{ }}", object.name)
}
