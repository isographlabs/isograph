use std::collections::BTreeSet;

use common_lang_types::{
    ArtifactPathAndContent, IsographObjectTypeName, QueryOperationName, VariableName,
};
use intern::{string_key::Intern, Lookup};
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::{ClientFieldId, IsographSelectionVariant, ServerObjectId};
use isograph_schema::{
    create_merged_selection_map_for_field_and_insert_into_global_map,
    current_target_merged_selections, get_imperatively_loaded_artifact_info,
    get_reachable_variables, ClientFieldToCompletedMergeTraversalStateMap, FieldTraversalResult,
    FieldType, MergedSelectionMap, RootOperationName, RootRefetchedPath,
    ScalarClientFieldTraversalState, SchemaObject, ValidatedClientField, ValidatedSchema,
    ValidatedVariableDefinition,
};

use crate::{
    generate_artifacts::{
        generate_path, NormalizationAstText, QueryText, RefetchQueryArtifactImport, ENTRYPOINT,
        RESOLVER_OUTPUT_TYPE, RESOLVER_PARAM_TYPE, RESOLVER_READER,
    },
    imperatively_loaded_fields::get_artifact_for_imperatively_loaded_field,
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
    concrete_type: IsographObjectTypeName,
}

pub(crate) fn generate_entrypoint_artifacts(
    schema: &ValidatedSchema,
    entrypoint_id: ClientFieldId,
    encountered_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
    file_extensions: GenerateFileExtensionsOption,
) -> Vec<ArtifactPathAndContent> {
    let entrypoint = schema.client_field(entrypoint_id);

    let FieldTraversalResult {
        traversal_state,
        merged_selection_map,
        ..
    } = create_merged_selection_map_for_field_and_insert_into_global_map(
        schema,
        schema.server_field_data.object(entrypoint.parent_object_id),
        entrypoint.selection_set_for_parent_query(),
        encountered_client_field_map,
        FieldType::ClientField(entrypoint.id),
        &entrypoint.initial_variable_context(),
    );

    generate_entrypoint_artifacts_with_client_field_traversal_result(
        schema,
        entrypoint,
        &merged_selection_map,
        &traversal_state,
        encountered_client_field_map,
        entrypoint
            .variable_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &schema.find_mutation(),
        file_extensions,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_entrypoint_artifacts_with_client_field_traversal_result<'a>(
    schema: &ValidatedSchema,
    entrypoint: &ValidatedClientField,
    merged_selection_map: &MergedSelectionMap,
    traversal_state: &ScalarClientFieldTraversalState,
    encountered_client_field_map: &ClientFieldToCompletedMergeTraversalStateMap,
    variable_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    default_root_operation: &Option<(&ServerObjectId, &RootOperationName)>,
    file_extensions: GenerateFileExtensionsOption,
) -> Vec<ArtifactPathAndContent> {
    let query_name = entrypoint.name.into();
    // TODO when we do not call generate_entrypoint_artifact extraneously,
    // we can panic instead of using a default entrypoint type
    // TODO model this better so that the RootOperationName is somehow a
    // parameter
    let root_operation_name = schema
        .fetchable_types
        .get(&entrypoint.parent_object_id)
        .unwrap_or_else(|| {
            default_root_operation
                .map(|(_, operation_name)| operation_name)
                .unwrap_or_else(|| {
                    schema
                        .fetchable_types
                        .iter()
                        .next()
                        .expect("Expected at least one fetchable type to exist")
                        .1
                })
        });

    let parent_object = schema.server_field_data.object(entrypoint.parent_object_id);
    let query_text = generate_query_text(
        query_name,
        schema,
        merged_selection_map,
        variable_definitions,
        root_operation_name,
    );
    let refetch_paths_with_variables = traversal_state
        .refetch_paths
        .iter()
        .map(|((path, selection_variant), root_refetch_path)| {
            let current_target_merged_selections = match selection_variant {
                IsographSelectionVariant::Regular => {
                    current_target_merged_selections(&path.linked_fields, merged_selection_map)
                }
                IsographSelectionVariant::Loadable(_) => {
                    // Note: it would be cleaner to include a reference to the merged selection set here via
                    // the selection_variant variable, instead of by looking it up like this.
                    &encountered_client_field_map
                        .get(&FieldType::ClientField(
                            root_refetch_path.path_to_refetch_field_info.client_field_id,
                        ))
                        .expect(
                            "Expected field to have been encountered, \
                                since it is being used as a refetch field.",
                        )
                        .merged_selection_map
                }
            };

            let reachable_variables = get_reachable_variables(current_target_merged_selections);
            (
                root_refetch_path.clone(),
                current_target_merged_selections,
                reachable_variables,
            )
        })
        .collect::<Vec<_>>();

    let refetch_query_artifact_import =
        generate_refetch_query_artifact_import(&refetch_paths_with_variables, file_extensions);

    let normalization_ast_text =
        generate_normalization_ast_text(schema, merged_selection_map.values(), 0);

    let concrete_type = schema.server_field_data.object(
        if schema
            .fetchable_types
            .contains_key(&entrypoint.parent_object_id)
        {
            entrypoint.parent_object_id
        } else {
            *default_root_operation
                .map(|(operation_id, _)| operation_id)
                .unwrap_or_else(|| {
                    schema
                        .fetchable_types
                        .iter()
                        .next()
                        .expect("Expected at least one fetchable type to exist")
                        .0
                })
        },
    );

    let mut paths_and_contents = vec![EntrypointArtifactInfo {
        query_text,
        query_name,
        parent_type: parent_object,
        normalization_ast_text,
        refetch_query_artifact_import,
        concrete_type: concrete_type.name,
    }
    .path_and_content(file_extensions)];

    for (index, (root_refetch_path, nested_selection_map, reachable_variables)) in
        refetch_paths_with_variables.into_iter().enumerate()
    {
        let artifact_info = get_imperatively_loaded_artifact_info(
            schema,
            entrypoint,
            root_refetch_path,
            nested_selection_map,
            &reachable_variables,
            index,
        );

        paths_and_contents.push(get_artifact_for_imperatively_loaded_field(
            schema,
            artifact_info,
        ))
    }

    paths_and_contents
}

fn generate_refetch_query_artifact_import(
    root_refetched_paths: &[(
        RootRefetchedPath,
        &MergedSelectionMap,
        BTreeSet<VariableName>,
    )],
    file_extensions: GenerateFileExtensionsOption,
) -> RefetchQueryArtifactImport {
    // TODO name the refetch queries with the path, or something, instead of
    // with indexes.
    let mut output = String::new();
    let mut array_syntax = String::new();
    for (query_index, item) in root_refetched_paths.iter().enumerate() {
        let RootRefetchedPath {
            path_to_refetch_field_info,
            ..
        } = &item.0;
        output.push_str(&format!(
            "import refetchQuery{} from './__refetch__{}{}';\n",
            query_index,
            query_index,
            file_extensions.ts()
        ));
        let variable_names_str = variable_names_to_string(
            &item.2,
            // What are we doing here?
            path_to_refetch_field_info
                .imperatively_loaded_field_variant
                .top_level_schema_field_arguments
                .iter()
                .map(|x| x.name.item.lookup().intern().into()),
        );
        array_syntax.push_str(&format!(
            "  {{ artifact: refetchQuery{}, allowedVariables: {} }},\n",
            query_index, variable_names_str
        ));
    }
    output.push_str(&format!(
        "const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [{}{}];",
        if root_refetched_paths.is_empty() {
            ""
        } else {
            "\n"
        },
        array_syntax
    ));
    RefetchQueryArtifactImport(output)
}

impl EntrypointArtifactInfo<'_> {
    fn path_and_content(
        self,
        file_extensions: GenerateFileExtensionsOption,
    ) -> ArtifactPathAndContent {
        let EntrypointArtifactInfo {
            query_name,
            parent_type,
            ..
        } = &self;

        let directory = generate_path(parent_type.name, (*query_name).into());

        ArtifactPathAndContent {
            relative_directory: directory,
            file_content: self.file_contents(file_extensions),
            file_name_prefix: *ENTRYPOINT,
        }
    }

    fn file_contents(self, file_extensions: GenerateFileExtensionsOption) -> String {
        let EntrypointArtifactInfo {
            query_text,
            normalization_ast_text,
            refetch_query_artifact_import,
            query_name,
            parent_type,
            concrete_type,
        } = self;
        let ts_file_extension = file_extensions.ts();
        let entrypoint_params_typename = format!("{}__{}__param", parent_type.name, query_name);
        let entrypoint_output_type_name =
            format!("{}__{}__output_type", parent_type.name, query_name);

        let resolver_reader_file_name = *RESOLVER_READER;
        let param_type_file_name = *RESOLVER_PARAM_TYPE;
        let output_type_file_name = *RESOLVER_OUTPUT_TYPE;
        format!(
            "import type {{IsographEntrypoint, \
            NormalizationAst, RefetchQueryNormalizationArtifactWrapper}} from '@isograph/react';\n\
            import {{{entrypoint_params_typename}}} from './{param_type_file_name}{ts_file_extension}';\n\
            import {{{entrypoint_output_type_name}}} from './{output_type_file_name}{ts_file_extension}';\n\
            import readerResolver from './{resolver_reader_file_name}{ts_file_extension}';\n\
            {refetch_query_artifact_import}\n\n\
            const queryText = '{query_text}';\n\n\
            const normalizationAst: NormalizationAst = {normalization_ast_text};\n\
            const artifact: IsographEntrypoint<\n\
            {}{entrypoint_params_typename},\n\
            {}{entrypoint_output_type_name}\n\
            > = {{\n\
            {}kind: \"Entrypoint\",\n\
            {}networkRequestInfo: {{\n\
            {}  kind: \"NetworkRequestInfo\",\n\
            {}  queryText,\n\
            {}  normalizationAst,\n\
            {}}},\n\
            {}concreteType: \"{concrete_type}\",\n\
            {}readerWithRefetchQueries: {{\n\
            {}  kind: \"ReaderWithRefetchQueries\",\n\
            {}  nestedRefetchQueries,\n\
            {}  readerArtifact: readerResolver,\n\
            {}}},\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ",
        )
    }
}

fn variable_names_to_string(
    variable_names: &BTreeSet<VariableName>,
    field_variables: impl Iterator<Item = VariableName>,
) -> String {
    let mut s = "[".to_string();

    for variable in variable_names {
        s.push_str(&format!("\"{}\", ", variable));
    }
    for variable in field_variables {
        s.push_str(&format!("\"{}\", ", variable));
    }

    s.push(']');

    s
}
