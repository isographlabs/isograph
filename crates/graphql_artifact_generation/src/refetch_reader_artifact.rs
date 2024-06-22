use std::{collections::BTreeSet, path::PathBuf};

use common_lang_types::{ArtifactPathAndContent, SelectableFieldName};
use intern::string_key::Intern;
use isograph_schema::{
    FieldMapItem, ImperativelyLoadedFieldVariant, ScalarClientFieldTraversalState, SchemaObject,
    ValidatedClientField, ValidatedSchema,
};

use crate::{
    generate_artifacts::{
        generate_client_field_parameter_type, generate_output_type, generate_path,
        ClientFieldFunctionImportStatement, ClientFieldOutputType, ClientFieldParameterType,
        ReaderAst, REFETCH_READER, RESOLVER_OUTPUT_TYPE, RESOLVER_PARAM_TYPE,
    },
    import_statements::{
        param_type_imports_to_import_statement, reader_imports_to_import_statement,
        ParamTypeImports, ReaderImports,
    },
    reader_ast::generate_reader_ast,
};

#[derive(Debug)]
struct RefetchReaderArtifactInfo<'schema> {
    parent_type: &'schema SchemaObject,
    client_field_name: SelectableFieldName,
    reader_imports: ReaderImports,
    param_type_imports: ParamTypeImports,
    client_field_output_type: ClientFieldOutputType,
    reader_ast: ReaderAst,
    client_field_parameter_type: ClientFieldParameterType,
    function_import_statement: ClientFieldFunctionImportStatement,
}

impl<'schema> RefetchReaderArtifactInfo<'schema> {
    fn path_and_content(self) -> Vec<ArtifactPathAndContent> {
        let RefetchReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }

    fn file_contents(self, relative_directory: &PathBuf) -> Vec<ArtifactPathAndContent> {
        let RefetchReaderArtifactInfo {
            function_import_statement,
            client_field_parameter_type,
            client_field_output_type,
            reader_ast,
            reader_imports,
            parent_type,
            client_field_name,
            param_type_imports,
            ..
        } = self;

        let reader_import_statement = reader_imports_to_import_statement(&reader_imports);
        let param_type_import_statement =
            param_type_imports_to_import_statement(&param_type_imports);

        let output_type_text = {
            let parent_type_name = parent_type.name;
            let field_name = client_field_name;
            let output_type = client_field_output_type;
            format!(
                "export type {}__{}__output_type = {};",
                parent_type_name, field_name, output_type
            )
        };
        let output_type_text = format!(
            "import {{ RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            {output_type_text}"
        );

        let parent_name = parent_type.name;
        let reader_param_type = format!("{parent_name}__{client_field_name}__param");
        let param_type_file_name = *RESOLVER_PARAM_TYPE;

        let reader_content = format!(
            "import type {{RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
            import {{ {reader_param_type} }} from './{param_type_file_name}';\n\
            {function_import_statement}\n\
            {reader_import_statement}\n\
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
            "{param_type_import_statement}\n\
            export type {reader_param_type} = {client_field_parameter_type};\n",
        );

        vec![
            ArtifactPathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *REFETCH_READER,
                file_content: reader_content,
            },
            ArtifactPathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *RESOLVER_PARAM_TYPE,
                file_content: param_type_content,
            },
            ArtifactPathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *RESOLVER_OUTPUT_TYPE,
                file_content: output_type_text,
            },
        ]
    }
}

pub(crate) fn generate_refetch_reader_artifact(
    schema: &ValidatedSchema,
    client_field: &ValidatedClientField,
    variant: &ImperativelyLoadedFieldVariant,
    scalar_client_field_traversal_state: &ScalarClientFieldTraversalState,
) -> Vec<ArtifactPathAndContent> {
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

        let (reader_ast, reader_imports) = generate_reader_ast(
            schema,
            selection_set,
            0,
            &scalar_client_field_traversal_state.refetch_paths,
        );

        let mut param_type_imports = BTreeSet::new();
        let client_field_parameter_type = generate_client_field_parameter_type(
            schema,
            &selection_set,
            parent_type.into(),
            &mut param_type_imports,
            0,
        );
        let client_field_output_type = generate_output_type(client_field);
        RefetchReaderArtifactInfo {
            parent_type: parent_type.into(),
            client_field_name: client_field.name,
            reader_ast,
            reader_imports,
            function_import_statement,
            client_field_output_type,
            client_field_parameter_type,
            param_type_imports,
        }
        .path_and_content()
    } else {
        panic!("Unsupported: client fields not on query with no selection set")
    }
}

fn generate_function_import_statement_for_refetch_reader() -> ClientFieldFunctionImportStatement {
    let include_read_out_data = get_read_out_data(&vec![FieldMapItem {
        from: "id".intern().into(),
        to: "id".intern().into(),
    }]);
    let indent = "  ";
    let content = format!(
        "{include_read_out_data}\n\
        import {{ makeNetworkRequest, type IsographEnvironment }} \
        from '@isograph/react';\n\
        const resolver = (\n\
        {indent}environment: IsographEnvironment,\n\
        {indent}artifact: RefetchQueryNormalizationArtifact,\n\
        {indent}readOutData: any,\n\
        {indent}filteredVariables: any\n\
        ) => () => {{\n\
        {indent}const variables = includeReadOutData(filteredVariables, readOutData);\n\
        {indent}return makeNetworkRequest(environment, artifact, variables);\n\
        }};\n"
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
