use common_lang_types::ArtifactPathAndContent;
use intern::string_key::Intern;
use isograph_schema::{
    FieldMapItem, PrimaryFieldInfo, RefetchedPathsMap, ValidatedClientField, ValidatedSchema,
};

use crate::{
    generate_artifacts::{
        generate_output_type, generate_path, ClientFieldFunctionImportStatement, REFETCH_READER,
        RESOLVER_OUTPUT_TYPE,
    },
    import_statements::reader_imports_to_import_statement,
    reader_ast::generate_reader_ast,
};

pub(crate) fn generate_refetch_reader_artifact(
    schema: &ValidatedSchema,
    client_field: &ValidatedClientField,
    primary_field_info: Option<&PrimaryFieldInfo>,
    refetched_paths: &RefetchedPathsMap,
    was_selected_loadably: bool,
) -> ArtifactPathAndContent {
    let function_import_statement = match primary_field_info {
        Some(info) => {
            generate_function_import_statement_for_mutation_reader(&info.primary_field_field_map)
        }
        None => generate_function_import_statement_for_refetch_reader(),
    };
    let parent_type = schema
        .server_field_data
        .object(client_field.parent_object_id);

    let (reader_ast, reader_imports) = generate_reader_ast(
        schema,
        if was_selected_loadably {
            // TODO model this better
            client_field
                .refetch_strategy
                .as_ref()
                .expect(
                    "Expected refetch strategy. \
                    This is indicative of a bug in Isograph.",
                )
                .refetch_selection_set()
        } else {
            client_field.selection_set_for_parent_query()
        },
        0,
        refetched_paths,
        &client_field.initial_variable_context(),
    );

    let relative_directory = generate_path(parent_type.name, client_field.name);

    let reader_import_statement = reader_imports_to_import_statement(&reader_imports);

    let reader_content = format!(
            "import type {{ RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            {function_import_statement}\n\
            {reader_import_statement}\n\
            const readerAst: ReaderAst<unknown> = {reader_ast};\n\n\
            const artifact: RefetchReaderArtifact = {{\n\
            {}kind: \"RefetchReaderArtifact\",\n\
            {}// @ts-ignore\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  "
        );

    ArtifactPathAndContent {
        relative_directory: relative_directory.clone(),
        file_name_prefix: *REFETCH_READER,
        file_content: reader_content,
    }
}

pub(crate) fn generate_refetch_output_type_artifact(
    schema: &ValidatedSchema,
    client_field: &ValidatedClientField,
) -> ArtifactPathAndContent {
    let parent_type = schema
        .server_field_data
        .object(client_field.parent_object_id);
    let relative_directory = generate_path(parent_type.name, client_field.name);

    let client_field_output_type = generate_output_type(client_field);

    let output_type_text = {
        let parent_type_name = parent_type.name;
        let output_type = client_field_output_type;
        format!(
            "export type {}__{}__output_type = {};",
            parent_type_name, client_field.name, output_type
        )
    };
    let output_type_text = format!(
        "import type React from 'react';\n\
        import {{ RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
        {output_type_text}"
    );
    ArtifactPathAndContent {
        relative_directory: relative_directory.clone(),
        file_name_prefix: *RESOLVER_OUTPUT_TYPE,
        file_content: output_type_text,
    }
}

fn generate_function_import_statement_for_refetch_reader() -> ClientFieldFunctionImportStatement {
    let include_read_out_data = get_read_out_data(&[FieldMapItem {
        from: "id".intern().into(),
        to: "id".intern().into(),
    }]);
    let indent = "  ";
    // TODO we need to generate nested refetch queries, which may either be
    // passed from the original entrypoint or specific to the loadable field.
    //
    // It should probably be passed from the original entrypoint.
    let content = format!(
        "{include_read_out_data}\n\
        import {{ makeNetworkRequest, wrapResolvedValue, type IsographEnvironment, \
        type FragmentReference, type RefetchQueryNormalizationArtifactWrapper, \
        type DataId, type TopLevelReaderArtifact }} \
        from '@isograph/react';\n\
        import {{ type ItemCleanupPair }} from '@isograph/react-disposable-state';\n\
        const resolver = (\n\
        {indent}environment: IsographEnvironment,\n\
        {indent}artifact: RefetchQueryNormalizationArtifact,\n\
        {indent}readOutData: any,\n\
        {indent}filteredVariables: any,\n\
        {indent}rootId: DataId,\n\
        {indent}// TODO type this\n\
        {indent}readerArtifact: TopLevelReaderArtifact<any, any, any> | null,\n\
        {indent}nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],\n\
        ) => (): ItemCleanupPair<FragmentReference<any, any>> | undefined => {{\n\
        {indent}const variables = includeReadOutData(filteredVariables, readOutData);\n\
        {indent}const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(environment, artifact, variables);\n\
        {indent}if (readerArtifact == null) return;\n\
        {indent}const fragmentReference = {{\n\
        {indent}  kind: \"FragmentReference\",\n\
        {indent}  readerWithRefetchQueries: wrapResolvedValue({{\n\
        {indent}    kind: \"ReaderWithRefetchQueries\",\n\
        {indent}    readerArtifact,\n\
        {indent}    nestedRefetchQueries,\n\
        {indent}  }} as const),\n\
        {indent}  root: rootId,\n\
        {indent}  variables,\n\
        {indent}  networkRequest,\n\
        {indent}}} as const;\n\
        {indent}return [fragmentReference, disposeNetworkRequest];\n\
        }};\n"
    );
    ClientFieldFunctionImportStatement(content)
}

fn generate_function_import_statement_for_mutation_reader(
    field_map: &[FieldMapItem],
) -> ClientFieldFunctionImportStatement {
    let include_read_out_data = get_read_out_data(field_map);
    let indent = "  ";
    ClientFieldFunctionImportStatement(format!(
        "{include_read_out_data}\n\
        import {{ makeNetworkRequest, wrapResolvedValue, type IsographEnvironment, \
        type DataId, type TopLevelReaderArtifact, \
        type FragmentReference, \
        type RefetchQueryNormalizationArtifactWrapper \
        }} from '@isograph/react';\n\
        import {{ type ItemCleanupPair }} from '@isograph/react-disposable-state';\n\
        const resolver = (\n\
        {indent}environment: IsographEnvironment,\n\
        {indent}artifact: RefetchQueryNormalizationArtifact,\n\
        {indent}readOutData: any,\n\
        {indent}filteredVariables: any,\n\
        {indent}rootId: DataId,\n\
        {indent}// TODO type this\n\
        {indent}readerArtifact: TopLevelReaderArtifact<any, any, any>,\n\
        {indent}nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],\n\
        ) => (): ItemCleanupPair<FragmentReference<any, any>> | undefined => {{\n\
        {indent}const variables = includeReadOutData(filteredVariables, readOutData);\n\
        {indent}const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(environment, artifact, variables);\n\
        {indent}if (readerArtifact == null) return;\n\
        {indent}const fragmentReference = {{\n\
        {indent}  kind: \"FragmentReference\",\n\
        {indent}  readerWithRefetchQueries: wrapResolvedValue({{\n\
        {indent}    kind: \"ReaderWithRefetchQueries\",\n\
        {indent}    readerArtifact,\n\
        {indent}    nestedRefetchQueries,\n\
        {indent}  }} as const),\n\
        {indent}  root: rootId,\n\
        {indent}  variables,\n\
        {indent}  networkRequest,\n\
        {indent}}} as const;\n\
        {indent}return [fragmentReference, disposeNetworkRequest];\n\
        }};\n\
        ",
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
