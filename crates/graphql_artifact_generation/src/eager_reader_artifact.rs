use intern::Lookup;
use std::{collections::HashMap, path::PathBuf, str::FromStr};

use common_lang_types::{PathAndContent, SelectableFieldName};
use isograph_schema::{
    create_merged_selection_set, SchemaObject, UserWrittenClientFieldInfo,
    UserWrittenComponentVariant, ValidatedClientField, ValidatedSchema,
};

use crate::{
    generate_artifacts::{
        generate_client_field_parameter_type, generate_output_type, generate_path,
        get_output_type_text, nested_client_field_names_to_import_statement,
        ClientFieldFunctionImportStatement, ClientFieldOutputType, ClientFieldParameterType,
        NestedClientFieldImports, ReaderAst, RESOLVER_OUTPUT_TYPE, RESOLVER_PARAM_TYPE,
        RESOLVER_READER,
    },
    reader_ast::generate_reader_ast,
};

pub fn generate_eager_reader_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    client_field: &ValidatedClientField,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
    info: UserWrittenClientFieldInfo,
) -> Vec<PathAndContent> {
    let user_written_component_variant = info.user_written_component_variant;
    if let Some((selection_set, _)) = &client_field.selection_set_and_unwraps {
        let parent_type = schema
            .server_field_data
            .object(client_field.parent_object_id);
        let mut nested_client_field_artifact_imports = HashMap::new();

        let (_merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            // TODO here we are assuming that the client field is only on the Query type.
            // That restriction should be loosened.
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
        let function_import_statement =
            generate_function_import_statement(project_root, artifact_directory, info);
        EagerReaderArtifactInfo {
            parent_type: parent_type.into(),
            client_field_name: client_field.name,
            reader_ast,
            nested_client_field_artifact_imports,
            function_import_statement,
            client_field_output_type,
            client_field_parameter_type,
            user_written_component_variant,
        }
        .path_and_content()
    } else {
        panic!("Unsupported: client fields not on query with no selection set")
    }
}

#[derive(Debug)]
struct EagerReaderArtifactInfo<'schema> {
    parent_type: &'schema SchemaObject,
    client_field_name: SelectableFieldName,
    nested_client_field_artifact_imports: NestedClientFieldImports,
    client_field_output_type: ClientFieldOutputType,
    reader_ast: ReaderAst,
    client_field_parameter_type: ClientFieldParameterType,
    function_import_statement: ClientFieldFunctionImportStatement,
    // TODO be generic over this type, since it is a GraphQL-ism?
    user_written_component_variant: UserWrittenComponentVariant,
}

impl<'schema> EagerReaderArtifactInfo<'schema> {
    fn path_and_content(self) -> Vec<PathAndContent> {
        let EagerReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }

    fn file_contents(self, relative_directory: &PathBuf) -> Vec<PathAndContent> {
        let EagerReaderArtifactInfo {
            function_import_statement,
            client_field_parameter_type,
            client_field_output_type,
            reader_ast,
            nested_client_field_artifact_imports,
            parent_type,
            client_field_name,
            user_written_component_variant,
            ..
        } = self;

        let (client_field_import_statement, client_field_type_import_statement) =
            nested_client_field_names_to_import_statement(
                nested_client_field_artifact_imports,
                parent_type.name,
            );

        let parent_name = parent_type.name;
        let reader_param_type = format!("{parent_name}__{client_field_name}__param");
        let output_type_text = get_output_type_text(
            &function_import_statement,
            parent_type.name,
            client_field_name,
            client_field_output_type,
        );

        let (reader_content, final_output_type_text) =
            if let UserWrittenComponentVariant::Eager = user_written_component_variant {
                let reader_output_type = format!("{parent_name}__{client_field_name}__outputType");
                let param_type_file_name = *RESOLVER_PARAM_TYPE;
                let output_type_file_name = *RESOLVER_OUTPUT_TYPE;
                (
                    format!(
                        "import type {{EagerReaderArtifact, ReaderAst, \
                        RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
                        import {{ {reader_param_type} }} from './{param_type_file_name}';\n\
                        import {{ {reader_output_type} }} from './{output_type_file_name}';\n\
                        {function_import_statement}\n\
                        {client_field_import_statement}\n\
                        const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
                        const artifact: EagerReaderArtifact<\n\
                        {}{reader_param_type},\n\
                        {}{reader_output_type}\n\
                        > = {{\n\
                        {}kind: \"EagerReaderArtifact\",\n\
                        {}resolver,\n\
                        {}readerAst,\n\
                        }};\n\n\
                        export default artifact;\n",
                        "  ", "  ", "  ", "  ", "  ",
                    ),
                    output_type_text,
                )
            } else {
                let component_name = format!("{}.{}", parent_name, client_field_name);
                let param_type_file_name = *RESOLVER_PARAM_TYPE;
                (
                    format!(
                        "import type {{ComponentReaderArtifact, ExtractSecondParam, \
                        ReaderAst, RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
                        import {{ {reader_param_type} }} from './{param_type_file_name}';\n\
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
                    ),
                    format!(
                        "import type {{ExtractSecondParam, RefetchQueryNormalizationArtifact}} \
                        from '@isograph/react';\n\
                        {output_type_text}\n",
                    ),
                )
            };

        let param_type_content = format!(
            "{client_field_type_import_statement}\n\
            export type {reader_param_type} = {client_field_parameter_type};\n",
        );

        vec![
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *RESOLVER_READER,
                file_content: reader_content,
            },
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *RESOLVER_PARAM_TYPE,
                file_content: param_type_content,
            },
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *RESOLVER_OUTPUT_TYPE,
                file_content: final_output_type_text,
            },
        ]
    }
}

fn generate_function_import_statement(
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
    user_written_client_field_info: UserWrittenClientFieldInfo,
) -> ClientFieldFunctionImportStatement {
    let const_export_name = user_written_client_field_info.const_export_name;
    let path_to_client_field = project_root.join(
        PathBuf::from_str(user_written_client_field_info.file_path.lookup())
            .expect("paths should be legal here. This is indicative of a bug in Isograph."),
    );
    let relative_path =
        // artifact directory includes __isograph, so artifact_directory.join("Type/Field")
        // is a directory "two levels deep" within the artifact_directory.
        //
        // So diff_paths(path_to_client_field, artifact_directory.join("Type/Field"))
        // is a lazy way of saying "make a relative path from two levels deep in the artifact
        // dir to the client field".
        //
        // Since we will always go ../../../ the Type/Field part will never show up
        // in the output.
        //
        // Anyway, TODO do better.
        pathdiff::diff_paths(path_to_client_field, artifact_directory.join("Type/Field"))
            .expect("Relative path should work");
    ClientFieldFunctionImportStatement(format!(
        "import {{ {const_export_name} as resolver }} from '{}';",
        relative_path.to_str().expect(
            "This path should be stringifiable. This probably is indicative of a bug in Relay."
        )
    ))
}
