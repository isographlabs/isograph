use common_lang_types::{ArtifactPathAndContent, ParentObjectEntityNameAndSelectableName};

use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::SelectionType;
use isograph_schema::{
    ClientScalarOrObjectSelectable, ClientScalarSelectable, FieldMapItem, IsographDatabase,
    NetworkProtocol, RefetchedPathsMap, Schema, initial_variable_context,
};

use crate::{
    generate_artifacts::{
        ClientFieldFunctionImportStatement, REFETCH_READER_FILE_NAME,
        RESOLVER_OUTPUT_TYPE_FILE_NAME, generate_output_type,
    },
    import_statements::reader_imports_to_import_statement,
    reader_ast::generate_reader_ast,
};

pub(crate) fn generate_refetch_reader_artifact<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    client_field: &ClientScalarSelectable<TNetworkProtocol>,
    refetched_paths: &RefetchedPathsMap,
    was_selected_loadably: bool,
    file_extensions: GenerateFileExtensionsOption,
    field_map: &[FieldMapItem],
) -> ArtifactPathAndContent {
    let read_out_data = get_read_out_data(field_map);
    let function_import_statement = generate_function_import_statement(read_out_data);

    let empty_selection_set = vec![];

    let (reader_ast, reader_imports) = generate_reader_ast(
        db,
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
                .unwrap_or(&empty_selection_set)
        } else {
            client_field.selection_set_for_parent_query()
        },
        0,
        refetched_paths,
        &initial_variable_context(&SelectionType::Scalar(client_field)),
    );

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

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
        file_name: *REFETCH_READER_FILE_NAME,
        file_content: reader_content,
        type_and_field: Some(ParentObjectEntityNameAndSelectableName {
            parent_object_entity_name: client_field.parent_object_entity_name,
            selectable_name: client_field.name.item.into(),
        }),
    }
}

pub(crate) fn generate_refetch_output_type_artifact<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_field: &ClientScalarSelectable<TNetworkProtocol>,
) -> ArtifactPathAndContent {
    let client_field_output_type = generate_output_type(db, client_field);

    let output_type_text = {
        let output_type = client_field_output_type;
        format!(
            "export type {}__{}__output_type = {};",
            client_field.parent_object_entity_name, client_field.name.item, output_type
        )
    };
    let output_type_text = format!(
        "import type React from 'react';\n\
        import {{ RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
        {output_type_text}"
    );
    ArtifactPathAndContent {
        file_name: *RESOLVER_OUTPUT_TYPE_FILE_NAME,
        file_content: output_type_text,
        type_and_field: Some(ParentObjectEntityNameAndSelectableName {
            parent_object_entity_name: client_field.parent_object_entity_name,
            selectable_name: client_field.name.item.into(),
        }),
    }
}

fn generate_function_import_statement(read_out_data: String) -> ClientFieldFunctionImportStatement {
    let indent = "  ";
    // TODO: use better type than Link<any>
    ClientFieldFunctionImportStatement(format!(
        "{read_out_data}\n\
        import {{ makeNetworkRequest, wrapResolvedValue, type IsographEnvironment, \
        type Link, type TopLevelReaderArtifact, \
        type FragmentReference, \
        type RefetchQueryNormalizationArtifactWrapper \
        }} from '@isograph/react';\n\
        import {{ type ItemCleanupPair }} from '@isograph/react-disposable-state';\n\
        const resolver = (\n\
        {indent}environment: IsographEnvironment,\n\
        {indent}artifact: RefetchQueryNormalizationArtifact,\n\
        {indent}readOutData: any,\n\
        {indent}filteredVariables: any,\n\
        {indent}rootLink: Link<any>,\n\
        {indent}// If readerArtifact is null, the return value is undefined.\n\
        {indent}// TODO reflect this in the types.\n\
        {indent}readerArtifact: TopLevelReaderArtifact<any, any, any> | null,\n\
        {indent}nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],\n\
        ) => (): ItemCleanupPair<FragmentReference<any, any>> | undefined => {{\n\
        {indent}const variables = includeReadOutData(filteredVariables, readOutData);\n\
        {indent}const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(environment, artifact, variables, null, null);\n\
        {indent}if (readerArtifact == null) return;\n\
        {indent}const fragmentReference = {{\n\
        {indent}  kind: \"FragmentReference\",\n\
        {indent}  readerWithRefetchQueries: wrapResolvedValue({{\n\
        {indent}    kind: \"ReaderWithRefetchQueries\",\n\
        {indent}    readerArtifact,\n\
        {indent}    nestedRefetchQueries,\n\
        {indent}  }} as const),\n\
        {indent}  root: rootLink,\n\
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
                    "{spaces}variables.{path_so_far}{path_segment_item} ??= {{}};\n"
                ));
                path_so_far.push_str(&format!("{path_segment_item}."));
            }
        }
    }

    s.push_str(&format!("{spaces}return variables;\n}};\n"));
    s
}
