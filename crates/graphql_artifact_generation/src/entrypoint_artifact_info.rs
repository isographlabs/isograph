use std::collections::HashSet;

use common_lang_types::{PathAndContent, QueryOperationName, VariableName};
use isograph_lang_types::ClientFieldId;
use isograph_schema::{
    create_merged_selection_set, ImperativelyLoadedFieldArtifactInfo, RootRefetchedPath,
    SchemaObject, ValidatedSchema,
};

use crate::{
    generate_artifacts::{
        generate_path, NormalizationAstText, QueryText, RefetchQueryArtifactImport, ENTRYPOINT,
    },
    normalization_ast_text::generate_normalization_ast_text,
    query_text::generate_query_text,
};

#[derive(Debug)]
struct EntrypointArtifactInfo<'schema> {
    query_name: QueryOperationName,
    parent_type: &'schema SchemaObject,
    query_text: QueryText,
    normalization_ast_text: NormalizationAstText,
    refetch_query_artifact_import: RefetchQueryArtifactImport,
}

pub(crate) fn generate_entrypoint_artifact(
    schema: &ValidatedSchema,
    client_field_id: ClientFieldId,
    artifact_queue: &mut Vec<ImperativelyLoadedFieldArtifactInfo>,
    encountered_client_field_ids: &mut HashSet<ClientFieldId>,
) -> PathAndContent {
    let fetchable_client_field = schema.client_field(client_field_id);
    if let Some((ref selection_set, _)) = fetchable_client_field.selection_set_and_unwraps {
        let query_name = fetchable_client_field.name.into();

        let (merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            schema
                .server_field_data
                .object(fetchable_client_field.parent_object_id),
            selection_set,
            Some(artifact_queue),
            Some(encountered_client_field_ids),
            &fetchable_client_field,
        );

        // TODO when we do not call generate_entrypoint_artifact extraneously,
        // we can panic instead of using a default entrypoint type
        // TODO model this better so that the RootOperationName is somehow a
        // parameter
        let root_operation_name = schema
            .fetchable_types
            .get(&fetchable_client_field.parent_object_id)
            .unwrap_or_else(|| {
                schema
                    .fetchable_types
                    .iter()
                    .next()
                    .expect("Expected at least one fetchable type to exist")
                    .1
            });

        let parent_object = schema
            .server_field_data
            .object(fetchable_client_field.parent_object_id);
        let query_text = generate_query_text(
            query_name,
            schema,
            &merged_selection_set,
            &fetchable_client_field.variable_definitions,
            root_operation_name,
        );
        let refetch_query_artifact_imports =
            generate_refetch_query_artifact_imports(&root_refetched_paths);

        let normalization_ast_text =
            generate_normalization_ast_text(schema, &merged_selection_set, 0);

        EntrypointArtifactInfo {
            query_text,
            query_name,
            parent_type: parent_object.into(),
            normalization_ast_text,
            refetch_query_artifact_import: refetch_query_artifact_imports,
        }
        .path_and_content()
    } else {
        // TODO convert to error
        todo!("Unsupported: client fields on query with no selection set")
    }
}

fn generate_refetch_query_artifact_imports(
    root_refetched_paths: &[RootRefetchedPath],
) -> RefetchQueryArtifactImport {
    // TODO name the refetch queries with the path, or something, instead of
    // with indexes.
    let mut output = String::new();
    let mut array_syntax = String::new();
    for (query_index, RootRefetchedPath { variables, .. }) in
        root_refetched_paths.iter().enumerate()
    {
        output.push_str(&format!(
            "import refetchQuery{} from './__refetch__{}';\n",
            query_index, query_index,
        ));
        let variable_names_str = variable_names_to_string(&variables);
        array_syntax.push_str(&format!(
            "{{ artifact: refetchQuery{}, allowedVariables: {} }}, ",
            query_index, variable_names_str
        ));
    }
    output.push_str(&format!(
        "const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [{}];",
        array_syntax
    ));
    RefetchQueryArtifactImport(output)
}

impl<'schema> EntrypointArtifactInfo<'schema> {
    fn path_and_content(self) -> PathAndContent {
        let EntrypointArtifactInfo {
            query_name,
            parent_type,
            ..
        } = &self;

        let directory = generate_path(parent_type.name, (*query_name).into());

        PathAndContent {
            relative_directory: directory,
            file_content: self.file_contents(),
            file_name_prefix: *ENTRYPOINT,
        }
    }

    fn file_contents(self) -> String {
        let EntrypointArtifactInfo {
            query_text,
            normalization_ast_text,
            refetch_query_artifact_import,
            query_name,
            parent_type,
        } = self;
        let entrypoint_params_typename = format!("{}__{}__param", parent_type.name, query_name);
        let entrypoint_output_type_name =
            format!("{}__{}__outputType", parent_type.name, query_name);
        format!(
            "import type {{IsographEntrypoint, \
            NormalizationAst, RefetchQueryNormalizationArtifactWrapper}} from '@isograph/react';\n\
            import {{{entrypoint_params_typename}}} from './param_type';\n\
            import {{{entrypoint_output_type_name}}} from './output_type';\n\
            import readerResolver from './reader';\n\
            {refetch_query_artifact_import}\n\n\
            const queryText = '{query_text}';\n\n\
            const normalizationAst: NormalizationAst = {normalization_ast_text};\n\
            const artifact: IsographEntrypoint<\n\
            {}{entrypoint_params_typename},\n\
            {}{entrypoint_output_type_name}\n\
            > = {{\n\
            {}kind: \"Entrypoint\",\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}nestedRefetchQueries,\n\
            {}readerArtifact: readerResolver,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ", "  ",
        )
    }
}

fn variable_names_to_string(variable_names: &[VariableName]) -> String {
    let mut s = "[".to_string();

    for variable in variable_names {
        s.push_str(&format!("\"{}\", ", variable));
    }

    s.push(']');

    s
}
