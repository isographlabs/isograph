use std::{collections::HashMap, path::PathBuf};

use common_lang_types::{PathAndContent, SelectableFieldName};
use isograph_schema::{
    create_merged_selection_set, FieldMapItem, ImperativelyLoadedFieldVariant, SchemaObject,
    ValidatedClientField, ValidatedSchema,
};

use crate::{
    artifact_file_contents::{
        get_output_type_text, nested_client_field_names_to_import_statement, READER,
        READER_OUTPUT_TYPE, READER_PARAM_TYPE,
    },
    eager_reader_artifact_info::generate_client_field_parameter_type,
    generate_artifacts::{
        generate_output_type, generate_path, ClientFieldFunctionImportStatement,
        ClientFieldOutputType, ClientFieldParameterType, NestedClientFieldImports, ReaderAst,
    },
    reader_ast::generate_reader_ast,
};

#[derive(Debug)]
pub(crate) struct RefetchReaderArtifactInfo<'schema> {
    pub parent_type: &'schema SchemaObject,
    pub(crate) client_field_name: SelectableFieldName,
    pub nested_client_field_artifact_imports: NestedClientFieldImports,
    pub client_field_output_type: ClientFieldOutputType,
    pub reader_ast: ReaderAst,
    pub client_field_parameter_type: ClientFieldParameterType,
    pub function_import_statement: ClientFieldFunctionImportStatement,
}

impl<'schema> RefetchReaderArtifactInfo<'schema> {
    pub fn path_and_content(self) -> Vec<PathAndContent> {
        let RefetchReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }

    pub(crate) fn file_contents(self, relative_directory: &PathBuf) -> Vec<PathAndContent> {
        let RefetchReaderArtifactInfo {
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
        let output_type_text = format!(
            "import {{ RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            {output_type_text}"
        );

        let parent_name = parent_type.name;
        let reader_param_type = format!("{parent_name}__{client_field_name}__param");

        let reader_content = format!(
            "import type {{RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
            import {{ {reader_param_type} }} from './param_type';\n\
            {function_import_statement}\n\
            {client_field_import_statement}\n\
            const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
            const artifact: RefetchReaderArtifact = {{\n\
            {}kind: \"RefetchReaderArtifact\",\n\
            {}// @ts-ignore\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  "
        );

        let param_type_content = format!(
            "{client_field_type_import_statement}\n\
            export type {reader_param_type} = {client_field_parameter_type};\n",
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
                file_content: output_type_text,
            },
        ]
    }
}

pub(crate) fn generate_refetch_reader_artifact_info<'schema>(
    schema: &'schema ValidatedSchema,
    client_field: &ValidatedClientField,
    variant: &ImperativelyLoadedFieldVariant,
) -> RefetchReaderArtifactInfo<'schema> {
    if let Some((selection_set, _)) = &client_field.selection_set_and_unwraps {
        let function_import_statement = match &variant.primary_field_info {
            Some(info) => generate_function_import_statement_for_mutation_reader(
                &info.primary_field_field_map,
            ),
            None => generate_function_import_statement_for_refetch_reader(),
        };
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
        RefetchReaderArtifactInfo {
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

fn generate_function_import_statement_for_refetch_reader() -> ClientFieldFunctionImportStatement {
    let content = format!(
        "import {{ makeNetworkRequest, type IsographEnvironment }} \
        from '@isograph/react';\n\
        const resolver = (\n\
        {}environment: IsographEnvironment,\n\
        {}artifact: RefetchQueryNormalizationArtifact,\n\
        {}variables: any\n\
        ) => () => \
        makeNetworkRequest(environment, artifact, variables);",
        "  ", "  ", "  "
    );
    ClientFieldFunctionImportStatement(content)
}

fn generate_function_import_statement_for_mutation_reader(
    field_map: &[FieldMapItem],
) -> ClientFieldFunctionImportStatement {
    let include_read_out_data = get_read_out_data(&field_map);
    ClientFieldFunctionImportStatement(format!(
        "{include_read_out_data}\n\
        import {{ makeNetworkRequest, type IsographEnvironment }} from '@isograph/react';\n\
        const resolver = (\n\
        {}environment: IsographEnvironment,\n\
        {}artifact: RefetchQueryNormalizationArtifact,\n\
        {}readOutData: any,\n\
        {}filteredVariables: any\n\
        ) => (mutationParams: any) => {{\n\
        {}const variables = includeReadOutData({{...filteredVariables, \
        ...mutationParams}}, readOutData);\n\
        {}makeNetworkRequest(environment, artifact, variables);\n\
        }};\n\
        ",
        "  ", "  ", "  ", "  ", "  ", "  "
    ))
}

fn get_read_out_data(field_map: &[FieldMapItem]) -> String {
    let spaces = "  ";
    let mut s = "const includeReadOutData = (variables: any, readOutData: any) => {\n".to_string();

    for item in field_map.iter() {
        // This is super hacky and due to the fact that argument names and field names are
        // treated differently, because that's how it is in the GraphQL spec.
        let split_to_arg = item.split_to_arg();
        let mut path_segments = Vec::with_capacity(1 + split_to_arg.to_field_names.len());
        path_segments.push(split_to_arg.to_argument_name);
        path_segments.extend(split_to_arg.to_field_names.into_iter());

        let last_index = path_segments.len() - 1;
        let mut path_so_far = "".to_string();
        for (index, path_segment) in path_segments.into_iter().enumerate() {
            let is_last = last_index == index;
            let path_segment_item = path_segment;

            if is_last {
                let from_value = item.from;
                s.push_str(&format!(
                    "{spaces}variables.{path_so_far}{path_segment_item} = \
                    readOutData.{from_value};\n"
                ));
            } else {
                s.push_str(&format!(
                    "{spaces}variables.{path_so_far}{path_segment_item} = \
                    variables.{path_so_far}{path_segment_item} ?? {{}};\n"
                ));
                path_so_far.push_str(&format!("{path_segment_item}."));
            }
        }
    }

    s.push_str(&format!("{spaces}return variables;\n}};\n"));
    s
}
