use std::{collections::HashMap, path::PathBuf};

use common_lang_types::{ConstExportName, FilePath, PathAndContent, SelectableFieldName};
use isograph_schema::{
    create_merged_selection_set, SchemaObject, ValidatedClientField, ValidatedSchema,
};

use crate::{
    artifact_file_contents::{
        get_output_type_text, nested_client_field_names_to_import_statement, READER,
        READER_OUTPUT_TYPE, READER_PARAM_TYPE,
    },
    eager_reader_artifact_info::generate_client_field_parameter_type,
    generate_artifacts::{
        generate_function_import_statement_for_eager_or_component, generate_output_type,
        generate_path, ClientFieldFunctionImportStatement, ClientFieldOutputType,
        ClientFieldParameterType, NestedClientFieldImports, ReaderAst,
    },
    reader_ast::generate_reader_ast,
};

pub(crate) fn generate_component_reader_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    client_field: &ValidatedClientField,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
    component_name_and_path: (ConstExportName, FilePath),
) -> ComponentReaderArtifactInfo<'schema> {
    if let Some((selection_set, _)) = &client_field.selection_set_and_unwraps {
        let parent_type = schema
            .server_field_data
            .object(client_field.parent_object_id);
        let mut nested_client_field_artifact_imports = HashMap::new();

        let (_merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            schema
                .server_field_data
                .object(client_field.parent_object_id)
                .into(),
            selection_set,
            None,
            None,
            client_field,
        );

        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            0,
            &mut nested_client_field_artifact_imports,
            &root_refetched_paths,
        );

        let client_field_parameter_type = generate_client_field_parameter_type(
            schema,
            &selection_set,
            parent_type.into(),
            &mut nested_client_field_artifact_imports,
            0,
        );
        let client_field_output_type = generate_output_type(client_field);
        let function_import_statement = generate_function_import_statement_for_eager_or_component(
            project_root,
            artifact_directory,
            component_name_and_path,
        );
        ComponentReaderArtifactInfo {
            parent_type: parent_type.into(),
            client_field_name: client_field.name,
            reader_ast,
            nested_client_field_artifact_imports,
            function_import_statement,
            client_field_output_type,
            client_field_parameter_type,
        }
    } else {
        panic!("Unsupported: client fields not on query with no selection set")
    }
}

#[derive(Debug)]
pub(crate) struct ComponentReaderArtifactInfo<'schema> {
    pub parent_type: &'schema SchemaObject,
    pub(crate) client_field_name: SelectableFieldName,
    pub nested_client_field_artifact_imports: NestedClientFieldImports,
    pub client_field_output_type: ClientFieldOutputType,
    pub reader_ast: ReaderAst,
    pub client_field_parameter_type: ClientFieldParameterType,
    pub function_import_statement: ClientFieldFunctionImportStatement,
}

impl<'schema> ComponentReaderArtifactInfo<'schema> {
    pub fn path_and_content(self) -> Vec<PathAndContent> {
        let ComponentReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }

    pub(crate) fn file_contents(self, relative_directory: &PathBuf) -> Vec<PathAndContent> {
        let ComponentReaderArtifactInfo {
            function_import_statement,
            client_field_parameter_type,
            client_field_output_type,
            reader_ast,
            nested_client_field_artifact_imports,
            parent_type,
            client_field_name,
            ..
        } = self;

        let (client_field_import_statement, client_field_type_import_statement) =
            nested_client_field_names_to_import_statement(
                nested_client_field_artifact_imports,
                parent_type.name,
            );

        let output_type_text = get_output_type_text(
            &function_import_statement,
            parent_type.name,
            client_field_name,
            client_field_output_type,
        );

        let parent_name = parent_type.name;
        let reader_param_type = format!("{parent_name}__{client_field_name}__param");
        let component_name = format!("{}.{}", parent_name, client_field_name);

        let reader_content = format!(
            "import type {{ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
            import {{ {reader_param_type} }} from './param_type';\n\
            {function_import_statement}\n\
            {client_field_import_statement}\n\
            const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
            const artifact: ComponentReaderArtifact<\n\
            {}{reader_param_type},\n\
            {}ExtractSecondParam<typeof resolver>\n\
            > = {{\n\
            {}kind: \"ComponentReaderArtifact\",\n\
            {}componentName: \"{component_name}\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ",
        );

        let param_type_content = format!(
            "{client_field_type_import_statement}\n\
            export type {reader_param_type} = {client_field_parameter_type};\n",
        );

        let output_type_content = format!(
            "import type {{ExtractSecondParam, RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
            {output_type_text}\n",
        );

        vec![
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *READER,
                file_content: reader_content,
            },
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *READER_PARAM_TYPE,
                file_content: param_type_content,
            },
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *READER_OUTPUT_TYPE,
                file_content: output_type_content,
            },
        ]
    }
}
