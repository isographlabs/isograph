use crate::{
    generate_artifacts::{
        ENTRYPOINT_FILE_NAME, NORMALIZATION_AST, NORMALIZATION_AST_FILE_NAME, QUERY_TEXT,
        QUERY_TEXT_FILE_NAME, RAW_RESPONSE_TYPE, RAW_RESPONSE_TYPE_FILE_NAME, RESOLVER_OUTPUT_TYPE,
        RESOLVER_PARAM_TYPE, RESOLVER_READER, RefetchQueryArtifactImport,
    },
    imperatively_loaded_fields::get_paths_and_contents_for_imperatively_loaded_field,
    normalization_ast_text::generate_normalization_ast_text,
    operation_text::{OperationText, generate_operation_text},
    persisted_documents::PersistedDocuments,
    raw_response_type::generate_raw_response_type,
};
use common_lang_types::{
    ArtifactPath, ArtifactPathAndContent, EntityName, EntityNameAndSelectableName,
    QueryOperationName, SelectableName,
};
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::{
    ClientScalarSelectableDirectiveSet, DefinitionLocationPostfix, EmptyDirectiveSet,
    EntrypointDirectiveSet, ScalarSelectionDirectiveSet, SelectionType, SelectionTypePostfix,
    VariableDeclaration, VariableNameWrapper,
};
use isograph_schema::{
    ClientFieldVariant, ClientScalarSelectable, CompilationProfile, EntrypointDeclarationInfo,
    FieldToCompletedMergeTraversalStateMap, FieldTraversalResult, Format, IsographDatabase,
    MergedSelectionMap, NetworkProtocol, NormalizationKey, RootOperationName, RootRefetchedPath,
    ScalarClientFieldTraversalState, ServerEntity, WrappedSelectionMapSelection,
    client_scalar_selectable_named, client_scalar_selectable_selection_set_for_parent_query,
    create_merged_selection_map_for_field_and_insert_into_global_map,
    current_target_merged_selections, fetchable_types, flattened_entity_named,
    get_reachable_variables, initial_variable_context,
};
use prelude::Postfix;
use std::collections::BTreeSet;

pub(crate) fn generate_entrypoint_artifacts<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    entrypoint_scalar_selectable_name: SelectableName,
    info: &EntrypointDeclarationInfo,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    file_extensions: GenerateFileExtensionsOption,
    persisted_documents: &mut Option<PersistedDocuments>,
) -> Vec<ArtifactPathAndContent> {
    let entrypoint = client_scalar_selectable_named(
        db,
        parent_object_entity_name,
        entrypoint_scalar_selectable_name,
    )
    .as_ref()
    .expect(
        "Expected parsing to have succeeded by this point. \
            This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect(
        "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
    )
    .lookup(db);

    let parent_object_entity = &flattened_entity_named(db, entrypoint.parent_entity_name)
        .as_ref()
        .expect(
            "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
        )
        .lookup(db);
    let FieldTraversalResult {
        traversal_state,
        merged_selection_map,
        ..
    } = create_merged_selection_map_for_field_and_insert_into_global_map(
        db,
        parent_object_entity,
        &client_scalar_selectable_selection_set_for_parent_query(
            db,
            entrypoint.parent_entity_name,
            entrypoint.name,
        )
        .expect("Expected selection set to be valid."),
        encountered_client_type_map,
        (
            entrypoint.parent_entity_name,
            entrypoint_scalar_selectable_name,
        )
            .scalar_selected()
            .client_defined(),
        &initial_variable_context(&entrypoint.scalar_selected()),
    );

    generate_entrypoint_artifacts_with_client_scalar_selectable_traversal_result(
        db,
        entrypoint,
        info.wrap_some(),
        &merged_selection_map,
        &traversal_state,
        encountered_client_type_map,
        entrypoint.variable_definitions.iter().collect(),
        &fetchable_types(db)
            .as_ref()
            .expect(
                "Expected parsing to have succeeded. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db)
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "mutation"),
        file_extensions,
        persisted_documents,
    )
}

#[expect(clippy::too_many_arguments)]
pub(crate) fn generate_entrypoint_artifacts_with_client_scalar_selectable_traversal_result<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    entrypoint: &ClientScalarSelectable<TCompilationProfile>,
    info: Option<&EntrypointDeclarationInfo>,
    merged_selection_map: &MergedSelectionMap,
    traversal_state: &ScalarClientFieldTraversalState,
    encountered_client_type_map: &FieldToCompletedMergeTraversalStateMap,
    variable_definitions: Vec<&VariableDeclaration>,
    // TODO this implements copy, don't take reference
    default_root_operation: &Option<(&EntityName, &RootOperationName)>,
    file_extensions: GenerateFileExtensionsOption,
    persisted_documents: &mut Option<PersistedDocuments>,
) -> Vec<ArtifactPathAndContent> {
    let query_name = entrypoint.name.into();
    // TODO when we do not call generate_entrypoint_artifact extraneously,
    // we can panic instead of using a default entrypoint type
    // TODO model this better so that the RootOperationName is somehow a
    // parameter
    let fetchable_types_map = fetchable_types(db)
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
                This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let root_operation_name = fetchable_types_map
        .get(&entrypoint.parent_entity_name)
        .unwrap_or_else(|| {
            default_root_operation
                .map(|(_, operation_name)| operation_name)
                .unwrap_or_else(|| {
                    fetchable_types_map
                        .values()
                        .next()
                        .expect("Expected at least one fetchable type to exist")
                })
        });

    let parent_object_entity = &flattened_entity_named(db, entrypoint.parent_entity_name)
        .as_ref()
        .expect(
            "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let reachable_variables =
        get_used_variable_definitions(merged_selection_map, variable_definitions);

    let query_text = TCompilationProfile::NetworkProtocol::generate_query_text(
        db,
        query_name,
        merged_selection_map,
        reachable_variables.iter().copied(),
        root_operation_name,
        Format::Pretty,
    );
    let refetch_paths_with_variables = traversal_state
        .refetch_paths
        .iter()
        .map(|((path, selection_variant), root_refetch_path)| {
            let current_target_merged_selections = match path.field_name.reference() {
                SelectionType::Object(name_and_arguments) => {
                    let mut linked_fields = path.linked_fields.clone();
                    linked_fields.push(NormalizationKey::ClientPointer(name_and_arguments.clone()));

                    current_target_merged_selections(&linked_fields, merged_selection_map)
                }
                SelectionType::Scalar(_) => {
                    match selection_variant {
                        ScalarSelectionDirectiveSet::Updatable(_)
                        | ScalarSelectionDirectiveSet::None(_) => current_target_merged_selections(
                            &path.linked_fields,
                            merged_selection_map,
                        ),
                        ScalarSelectionDirectiveSet::Loadable(_) => {
                            // Note: it would be cleaner to include a reference to the merged selection set here via
                            // the selection_variant variable, instead of by looking it up like this.
                            &encountered_client_type_map
                                .get(
                                    &root_refetch_path
                                        .path_to_refetch_field_info
                                        .client_selectable_id
                                        .client_defined(),
                                )
                                .expect(
                                    "Expected field to have been encountered, \
                                    since it is being used as a refetch field.",
                                )
                                .merged_selection_map
                        }
                    }
                }
            };

            let reachable_variables =
                get_reachable_variables(current_target_merged_selections).collect();
            (
                root_refetch_path.clone(),
                current_target_merged_selections,
                reachable_variables,
            )
        })
        .collect::<Vec<_>>();

    let refetch_query_artifact_import =
        generate_refetch_query_artifact_import(&refetch_paths_with_variables, file_extensions);

    let normalization_ast_text = generate_normalization_ast_text(merged_selection_map.values(), 1);

    let concrete_type_entity_name =
        if fetchable_types_map.contains_key(&entrypoint.parent_entity_name) {
            entrypoint.parent_entity_name
        } else {
            *default_root_operation
                .map(|(operation_id, _)| operation_id)
                .unwrap_or_else(|| {
                    fetchable_types_map
                        .keys()
                        .next()
                        .expect("Expected at least one fetchable type to exist")
                })
        };
    let concrete_object_entity = &flattened_entity_named(db, concrete_type_entity_name)
        .as_ref()
        .expect(
            "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let operation_text = generate_operation_text(
        db,
        query_name,
        merged_selection_map,
        reachable_variables.iter().copied(),
        root_operation_name,
        concrete_object_entity.name.item,
        persisted_documents,
        1,
    );

    let directive_set = info
        .map(|info| info.directive_set)
        .unwrap_or(EntrypointDirectiveSet::None(EmptyDirectiveSet {}));

    let field_name = query_name.unchecked_conversion();
    let type_name = parent_object_entity.name;

    let entrypoint_file_content = entrypoint_file_content(
        file_extensions,
        query_name,
        &operation_text,
        parent_object_entity,
        &refetch_query_artifact_import,
        entrypoint.name,
        concrete_object_entity.name.item,
        &directive_set,
        match entrypoint.variant.reference() {
            ClientFieldVariant::UserWritten(info) => {
                info.client_scalar_selectable_directive_set.clone().expect(
                    "Expected directive set to have been validated by now. \
                    This is indicative of a bug in Isograph.",
                )
            }
            ClientFieldVariant::ImperativelyLoadedField(_) => {
                ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
            }
            ClientFieldVariant::Link => {
                ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
            }
        },
    );

    let raw_response_type = generate_raw_response_type(db, merged_selection_map, 0);

    let mut path_and_contents = Vec::with_capacity(refetch_paths_with_variables.len() + 3);
    path_and_contents.push(ArtifactPathAndContent {
        file_content: format!("export default '{query_text}';").into(),
        artifact_path: ArtifactPath {
            file_name: *QUERY_TEXT_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: type_name.item,
                selectable_name: field_name,
            }
            .wrap_some(),
        },
    });
    path_and_contents.push(ArtifactPathAndContent {
        file_content: format!(
            "import type {{NormalizationAst}} from '@isograph/react';\n\
            const normalizationAst: NormalizationAst = {{\n\
            {}kind: \"NormalizationAst\",\n\
            {}selections: {normalization_ast_text},\n\
            }};\n\
            export default normalizationAst;\n",
            "  ", "  "
        )
        .into(),
        artifact_path: ArtifactPath {
            file_name: *NORMALIZATION_AST_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: type_name.item,
                selectable_name: field_name,
            }
            .wrap_some(),
        },
    });
    path_and_contents.push(ArtifactPathAndContent {
        file_content: format!(
            "export type {}__{}__{} = {raw_response_type}\n",
            type_name, field_name, *RAW_RESPONSE_TYPE
        )
        .into(),
        artifact_path: ArtifactPath {
            file_name: *RAW_RESPONSE_TYPE_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: type_name.item,
                selectable_name: field_name,
            }
            .wrap_some(),
        },
    });
    path_and_contents.push(ArtifactPathAndContent {
        file_content: entrypoint_file_content.into(),
        artifact_path: ArtifactPath {
            file_name: *ENTRYPOINT_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: type_name.item,
                selectable_name: field_name,
            }
            .wrap_some(),
        },
    });

    path_and_contents.extend(
        refetch_paths_with_variables
            .into_iter()
            .enumerate()
            .flat_map(
                |(index, (root_refetch_path, nested_selection_map, reachable_variables))| {
                    get_paths_and_contents_for_imperatively_loaded_field(
                        db,
                        file_extensions,
                        persisted_documents,
                        entrypoint,
                        root_refetch_path,
                        nested_selection_map,
                        &reachable_variables,
                        index,
                    )
                },
            ),
    );

    path_and_contents
}

fn get_used_variable_definitions<'a>(
    merged_selection_map: &MergedSelectionMap,
    variable_definitions: Vec<&'a VariableDeclaration>,
) -> BTreeSet<&'a VariableDeclaration> {
    get_reachable_variables(merged_selection_map)
        .map(|variable_name| {
            *variable_definitions
                .iter()
                .find(|definition| definition.name.item == variable_name)
                .unwrap_or_else(|| panic!(
                    "Expected to find a variable defined at the root with name {variable_name}.\n\
                    This is indicative of a bug in Isograph."
                ))
        })
        .collect()
}

fn generate_refetch_query_artifact_import(
    root_refetched_paths: &[(
        RootRefetchedPath,
        &MergedSelectionMap,
        BTreeSet<VariableNameWrapper>,
    )],
    file_extensions: GenerateFileExtensionsOption,
) -> RefetchQueryArtifactImport {
    // TODO name the refetch queries with the path, or something, instead of
    // with indexes.
    let mut output = String::new();
    let mut array_syntax = String::new();
    for (query_index, (root_refetched_path, _merged_selection_set, variable_names)) in
        root_refetched_paths.iter().enumerate()
    {
        output.push_str(&format!(
            "import refetchQuery{} from './__refetch__{}{}';\n",
            query_index,
            query_index,
            file_extensions.ts()
        ));

        let variable_names_str = variable_names_to_string(
            variable_names,
            get_used_variables_for_refetch_query_import(
                &root_refetched_path
                    .path_to_refetch_field_info
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

#[expect(clippy::too_many_arguments)]
fn entrypoint_file_content<TCompilationProfile: CompilationProfile>(
    file_extensions: GenerateFileExtensionsOption,
    query_name: QueryOperationName,
    operation_text: &OperationText,
    parent_type: &ServerEntity<TCompilationProfile>,
    refetch_query_artifact_import: &RefetchQueryArtifactImport,
    field_name: SelectableName,
    concrete_type: EntityName,
    directive_set: &EntrypointDirectiveSet,
    field_directive_set: ClientScalarSelectableDirectiveSet,
) -> String {
    let ts_file_extension = file_extensions.ts();
    let entrypoint_params_typename = format!("{}__{}__param", parent_type.name, query_name);
    let entrypoint_output_type_name = format!("{}__{}__output_type", parent_type.name, query_name);
    let raw_response_type_name = format!(
        "{}__{}__{}",
        parent_type.name, query_name, *RAW_RESPONSE_TYPE
    );
    let resolver_reader_file_name = *RESOLVER_READER;
    let param_type_file_name = *RESOLVER_PARAM_TYPE;
    let output_type_file_name = *RESOLVER_OUTPUT_TYPE;
    let raw_response_type_file_name = *RAW_RESPONSE_TYPE;
    let query_text_file_name = *QUERY_TEXT;
    let normalization_text_file_name = *NORMALIZATION_AST;
    let indent = "  ";

    let (normalization_ast_type_name, normalization_ast_import, normalization_ast_code) = {
        let file_path = format!("'./{normalization_text_file_name}{ts_file_extension}'");
        match directive_set {
            EntrypointDirectiveSet::LazyLoad(directive_set)
                if directive_set.lazy_load.normalization =>
            {
                (
                    "NormalizationAstLoader",
                    "".to_string(),
                    format!(
                        "{indent}  normalizationAst: {{\n\
                         {indent}    kind: \"NormalizationAstLoader\",\n\
                         {indent}    loader: () => import({file_path}).then(module => module.default),\n\
                         {indent}  }},"
                    ),
                )
            }
            _ => (
                "NormalizationAst",
                format!("import normalizationAst from {file_path};\n"),
                format!("{indent}  normalizationAst,"),
            ),
        }
    };

    let (reader_import, reader_code) = {
        let reader_resolver_file_path =
            format!("'./{resolver_reader_file_name}{ts_file_extension}'");
        match directive_set {
            EntrypointDirectiveSet::LazyLoad(directive_set) if directive_set.lazy_load.reader => {
                let reader_artifact_kind =
                    if let ClientScalarSelectableDirectiveSet::None(_) = field_directive_set {
                        "EagerReaderArtifact"
                    } else {
                        "ComponentReaderArtifact"
                    };

                (
                    "".to_string(),
                    format!(
                        "{indent}readerWithRefetchQueries: {{\n\
                     {indent}  kind: \"ReaderWithRefetchQueriesLoader\",\n\
                     {indent}  fieldName: \"{field_name}\",\n\
                     {indent}  readerArtifactKind: \"{reader_artifact_kind}\",\n\
                     {indent}  loader: () => import({reader_resolver_file_path})\n\
                     {indent}    .then(module => ({{\n\
                     {indent}      kind: \"ReaderWithRefetchQueries\",\n\
                     {indent}      nestedRefetchQueries,\n\
                     {indent}      readerArtifact: module.default,\n\
                     {indent}    }}))\n\
                     {indent}}}"
                    ),
                )
            }
            _ => (
                format!("import readerResolver from {reader_resolver_file_path};\n"),
                format!(
                    "{indent}readerWithRefetchQueries: {{\n\
                     {indent}  kind: \"ReaderWithRefetchQueries\",\n\
                     {indent}  nestedRefetchQueries,\n\
                     {indent}  readerArtifact: readerResolver,\n\
                     {indent}}},"
                ),
            ),
        }
    };

    format!(
        "import type {{IsographEntrypoint, \
        {normalization_ast_type_name}, RefetchQueryNormalizationArtifactWrapper}} from '@isograph/react';\n\
        import {{{entrypoint_params_typename}}} from './{param_type_file_name}{ts_file_extension}';\n\
        import {{{entrypoint_output_type_name}}} from './{output_type_file_name}{ts_file_extension}';\n\
        import type {{{raw_response_type_name}}} from './{raw_response_type_file_name}{ts_file_extension}';\n\
        {reader_import}\
        import queryText from './{query_text_file_name}{ts_file_extension}';\n\
        {normalization_ast_import}\
        {refetch_query_artifact_import}\n\n\
        const artifact: IsographEntrypoint<\n\
        {indent}{entrypoint_params_typename},\n\
        {indent}{entrypoint_output_type_name},\n\
        {indent}{normalization_ast_type_name},\n\
        {indent}{raw_response_type_name}\n\
        > = {{\n\
        {indent}kind: \"Entrypoint\",\n\
        {indent}networkRequestInfo: {{\n\
        {indent}  kind: \"NetworkRequestInfo\",\n\
        {indent}  operation: {operation_text},\n\
        {normalization_ast_code}\n\
        {indent}}},\n\
        {indent}concreteType: \"{concrete_type}\",\n\
        {reader_code}\n\
        }};\n\n\
        export default artifact;\n",
    )
}

fn variable_names_to_string(
    variable_names: &BTreeSet<VariableNameWrapper>,
    field_variables: impl Iterator<Item = VariableNameWrapper>,
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

fn get_used_variables_for_refetch_query_import(
    inline_fragments_or_linked_fields: &[WrappedSelectionMapSelection],
) -> BTreeSet<VariableNameWrapper> {
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
