use std::collections::BTreeSet;

use common_lang_types::{
    ArtifactPathAndContent, ClientScalarSelectableName, ParentObjectEntityNameAndSelectableName,
    QueryOperationName, QueryText, ServerObjectEntityName, VariableName,
};
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::{
    DefinitionLocation, EmptyDirectiveSet, EntrypointDirectiveSet, ScalarSelectionDirectiveSet,
    SelectionType,
};
use isograph_schema::{
    create_merged_selection_map_for_field_and_insert_into_global_map,
    current_target_merged_selections, get_imperatively_loaded_artifact_info,
    get_reachable_variables, initial_variable_context, ClientScalarOrObjectSelectable,
    ClientScalarSelectable, EntrypointDeclarationInfo, FieldToCompletedMergeTraversalStateMap,
    FieldTraversalResult, Format, MergedSelectionMap, NetworkProtocol, RootOperationName,
    RootRefetchedPath, ScalarClientFieldTraversalState, Schema, ServerObjectEntity,
    ValidatedVariableDefinition, WrappedSelectionMapSelection,
};

use crate::{
    generate_artifacts::{
        NormalizationAstText, RefetchQueryArtifactImport, ENTRYPOINT_FILE_NAME, NORMALIZATION_AST,
        NORMALIZATION_AST_FILE_NAME, QUERY_TEXT, QUERY_TEXT_FILE_NAME, RESOLVER_OUTPUT_TYPE,
        RESOLVER_PARAM_TYPE, RESOLVER_READER,
    },
    imperatively_loaded_fields::get_artifact_for_imperatively_loaded_field,
    normalization_ast_text::generate_normalization_ast_text,
    operation_text::{generate_operation_text, OperationText},
    persisted_documents::PersistedDocuments,
};

#[derive(Debug)]
struct EntrypointArtifactInfo<'schema, TNetworkProtocol: NetworkProtocol> {
    query_name: QueryOperationName,
    parent_type: &'schema ServerObjectEntity<TNetworkProtocol>,
    query_text: QueryText,
    operation_text: OperationText,
    normalization_ast_text: NormalizationAstText,
    refetch_query_artifact_import: RefetchQueryArtifactImport,
    concrete_type: ServerObjectEntityName,
    directive_set: EntrypointDirectiveSet,
}

pub(crate) fn generate_entrypoint_artifacts<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    entrypoint_scalar_selectable_name: ClientScalarSelectableName,
    info: &EntrypointDeclarationInfo,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    file_extensions: GenerateFileExtensionsOption,
    persisted_documents: &mut Option<PersistedDocuments>,
) -> Vec<ArtifactPathAndContent> {
    let entrypoint = schema
        .client_field(parent_object_entity_name, entrypoint_scalar_selectable_name)
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    let FieldTraversalResult {
        traversal_state,
        merged_selection_map,
        ..
    } = create_merged_selection_map_for_field_and_insert_into_global_map(
        schema,
        schema
            .server_entity_data
            .server_object_entity(entrypoint.parent_object_entity_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            ),
        entrypoint.selection_set_for_parent_query(),
        encountered_client_type_map,
        DefinitionLocation::Client(SelectionType::Scalar((
            entrypoint.parent_object_entity_name,
            entrypoint_scalar_selectable_name,
        ))),
        &initial_variable_context(&SelectionType::Scalar(entrypoint)),
    );

    generate_entrypoint_artifacts_with_client_field_traversal_result(
        schema,
        entrypoint,
        Some(info),
        &merged_selection_map,
        &traversal_state,
        encountered_client_type_map,
        entrypoint
            .variable_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &schema.find_mutation(),
        file_extensions,
        persisted_documents,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_entrypoint_artifacts_with_client_field_traversal_result<
    'a,
    TNetworkProtocol: NetworkProtocol,
>(
    schema: &Schema<TNetworkProtocol>,
    entrypoint: &ClientScalarSelectable<TNetworkProtocol>,
    info: Option<&EntrypointDeclarationInfo>,
    merged_selection_map: &MergedSelectionMap,
    traversal_state: &ScalarClientFieldTraversalState,
    encountered_client_type_map: &FieldToCompletedMergeTraversalStateMap,
    variable_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + Clone + 'a,
    default_root_operation: &Option<(&ServerObjectEntityName, &RootOperationName)>,
    file_extensions: GenerateFileExtensionsOption,
    persisted_documents: &mut Option<PersistedDocuments>,
) -> Vec<ArtifactPathAndContent> {
    let query_name = entrypoint.name.into();
    // TODO when we do not call generate_entrypoint_artifact extraneously,
    // we can panic instead of using a default entrypoint type
    // TODO model this better so that the RootOperationName is somehow a
    // parameter
    let root_operation_name = schema
        .fetchable_types
        .get(&entrypoint.parent_object_entity_name)
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

    let parent_object = schema
        .server_entity_data
        .server_object_entity(entrypoint.parent_object_entity_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );
    let query_text = TNetworkProtocol::generate_query_text(
        query_name,
        schema,
        merged_selection_map,
        variable_definitions.clone(),
        root_operation_name,
        Format::Pretty,
    );
    let refetch_paths_with_variables = traversal_state
        .refetch_paths
        .iter()
        .map(|((path, selection_variant), root_refetch_path)| {
            let current_target_merged_selections = match selection_variant {
                ScalarSelectionDirectiveSet::Updatable(_)
                | ScalarSelectionDirectiveSet::None(_) => {
                    current_target_merged_selections(&path.linked_fields, merged_selection_map)
                }
                ScalarSelectionDirectiveSet::Loadable(_) => {
                    // Note: it would be cleaner to include a reference to the merged selection set here via
                    // the selection_variant variable, instead of by looking it up like this.
                    &encountered_client_type_map
                        .get(&DefinitionLocation::Client(SelectionType::Scalar((
                            root_refetch_path
                                .path_to_refetch_field_info
                                .refetch_field_parent_object_entity_name,
                            root_refetch_path
                                .path_to_refetch_field_info
                                .client_field_name,
                        ))))
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
        generate_normalization_ast_text(schema, merged_selection_map.values(), 1);

    let concrete_type = schema
        .server_entity_data
        .server_object_entity(
            if schema
                .fetchable_types
                .contains_key(&entrypoint.parent_object_entity_name)
            {
                entrypoint.parent_object_entity_name
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
        )
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

    let operation_text = generate_operation_text(
        query_name,
        schema,
        merged_selection_map,
        variable_definitions,
        root_operation_name,
        concrete_type.name,
        persisted_documents,
        1,
    );

    let mut paths_and_contents = EntrypointArtifactInfo {
        query_text,
        operation_text,
        query_name,
        parent_type: parent_object,
        normalization_ast_text,
        refetch_query_artifact_import,
        concrete_type: concrete_type.name,
        directive_set: info
            .map(|info| info.directive_set)
            .unwrap_or(EntrypointDirectiveSet::None(EmptyDirectiveSet {})),
    }
    .path_and_content(file_extensions);

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

        paths_and_contents.extend(get_artifact_for_imperatively_loaded_field(
            schema,
            artifact_info,
            file_extensions,
            persisted_documents,
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
            get_used_variables(
                &path_to_refetch_field_info
                    .imperatively_loaded_field_variant
                    .subfields_or_inline_fragments,
            )
            .into_iter(),
        );
        array_syntax.push_str(&format!(
            "  {{ artifact: refetchQuery{query_index}, allowedVariables: {variable_names_str} }},\n"
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

impl<TNetworkProtocol: NetworkProtocol> EntrypointArtifactInfo<'_, TNetworkProtocol> {
    fn path_and_content(
        self,
        file_extensions: GenerateFileExtensionsOption,
    ) -> Vec<ArtifactPathAndContent> {
        let EntrypointArtifactInfo {
            query_name,
            parent_type,
            query_text,
            operation_text,
            normalization_ast_text,
            refetch_query_artifact_import,
            concrete_type,
            directive_set,
        } = &self;
        let field_name = (*query_name).into();
        let type_name = parent_type.name;

        let entrypoint_file_content = entrypoint_file_content(
            file_extensions,
            query_name,
            operation_text,
            parent_type,
            refetch_query_artifact_import,
            concrete_type,
            directive_set,
        );

        vec![
            ArtifactPathAndContent {
                file_content: format!("export default '{query_text}';"),
                file_name: *QUERY_TEXT_FILE_NAME,
                type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                    type_name,
                    field_name,
                }),
            },
            ArtifactPathAndContent {
                file_content: format!(
                    "import type {{NormalizationAst}} from '@isograph/react';\n\
                    const normalizationAst: NormalizationAst = {{\n\
                    {}kind: \"NormalizationAst\",\n\
                    {}selections: {normalization_ast_text},\n\
                    }};\n\
                    export default normalizationAst;\n",
                    "  ", "  "
                ),
                file_name: *NORMALIZATION_AST_FILE_NAME,
                type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                    type_name,
                    field_name,
                }),
            },
            ArtifactPathAndContent {
                file_content: entrypoint_file_content,
                file_name: *ENTRYPOINT_FILE_NAME,
                type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                    type_name,
                    field_name,
                }),
            },
        ]
    }
}

fn entrypoint_file_content<TNetworkProtocol: NetworkProtocol>(
    file_extensions: GenerateFileExtensionsOption,
    query_name: &QueryOperationName,
    operation_text: &OperationText,
    parent_type: &ServerObjectEntity<TNetworkProtocol>,
    refetch_query_artifact_import: &RefetchQueryArtifactImport,
    concrete_type: &ServerObjectEntityName,
    directive_set: &EntrypointDirectiveSet,
) -> String {
    let ts_file_extension = file_extensions.ts();
    let entrypoint_params_typename = format!("{}__{}__param", parent_type.name, query_name);
    let entrypoint_output_type_name = format!("{}__{}__output_type", parent_type.name, query_name);
    let resolver_reader_file_name = *RESOLVER_READER;
    let param_type_file_name = *RESOLVER_PARAM_TYPE;
    let output_type_file_name = *RESOLVER_OUTPUT_TYPE;
    let query_text_file_name = *QUERY_TEXT;
    let normalization_text_file_name = *NORMALIZATION_AST;
    let indent = "  ";

    let (normalization_ast_type_name, normalization_ast_import, normalization_ast_code) = {
        let file_path = format!("'./{normalization_text_file_name}{ts_file_extension}'");
        match directive_set {
            EntrypointDirectiveSet::LazyLoad(directive_set) if directive_set.lazy_load.normalization => (
                "NormalizationAstLoader",
                "".to_string(),
                format!(
                    "{indent}  normalizationAst: {{\n\
                     {indent}    kind: \"NormalizationAstLoader\",\n\
                     {indent}    loader: () => import({file_path}).then(module => module.default),\n\
                     {indent}  }},"
                ),
            ),
            _ => (
                "NormalizationAst",
                format!("import normalizationAst from {file_path};\n"),
                format!("{indent}  normalizationAst,"),
            ),
        }
    };

    format!(
        "import type {{IsographEntrypoint, \
        {normalization_ast_type_name}, RefetchQueryNormalizationArtifactWrapper}} from '@isograph/react';\n\
        import {{{entrypoint_params_typename}}} from './{param_type_file_name}{ts_file_extension}';\n\
        import {{{entrypoint_output_type_name}}} from './{output_type_file_name}{ts_file_extension}';\n\
        import readerResolver from './{resolver_reader_file_name}{ts_file_extension}';\n\
        import queryText from './{query_text_file_name}{ts_file_extension}';\n\
        {normalization_ast_import}\
        {refetch_query_artifact_import}\n\n\
        const artifact: IsographEntrypoint<\n\
        {indent}{entrypoint_params_typename},\n\
        {indent}{entrypoint_output_type_name},\n\
        {indent}{normalization_ast_type_name}\n\
        > = {{\n\
        {indent}kind: \"Entrypoint\",\n\
        {indent}networkRequestInfo: {{\n\
        {indent}  kind: \"NetworkRequestInfo\",\n\
        {indent}  operation: {operation_text},\n\
        {normalization_ast_code}\n\
        {indent}}},\n\
        {indent}concreteType: \"{concrete_type}\",\n\
        {indent}readerWithRefetchQueries: {{\n\
        {indent}  kind: \"ReaderWithRefetchQueries\",\n\
        {indent}  nestedRefetchQueries,\n\
        {indent}  readerArtifact: readerResolver,\n\
        {indent}}},\n\
        }};\n\n\
        export default artifact;\n",
    )
}

fn variable_names_to_string(
    variable_names: &BTreeSet<VariableName>,
    field_variables: impl Iterator<Item = VariableName>,
) -> String {
    let mut s = "[".to_string();

    for variable in variable_names {
        s.push_str(&format!("\"{variable}\", "));
    }
    for variable in field_variables {
        s.push_str(&format!("\"{variable}\", "));
    }

    s.push(']');

    s
}

fn get_used_variables(
    inline_fragments_or_linked_fields: &[WrappedSelectionMapSelection],
) -> BTreeSet<VariableName> {
    // TODO return impl iterator
    let mut variables = BTreeSet::new();

    for item in inline_fragments_or_linked_fields {
        match item {
            WrappedSelectionMapSelection::LinkedField { arguments, .. } => {
                for arg in arguments {
                    variables.extend(arg.value.variables());
                }
            }
            WrappedSelectionMapSelection::InlineFragment(_) => {}
        }
    }

    variables
}
