use std::{collections::BTreeSet, ops::Deref};

use common_lang_types::{
    ArtifactPathAndContent, ParentObjectEntityNameAndSelectableName, VariableName,
};
use intern::string_key::Intern;
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::VariableDefinition;
use isograph_schema::{
    ClientScalarOrObjectSelectable, ClientScalarSelectable, ClientSelectable, Format,
    ImperativelyLoadedFieldVariant, IsographDatabase, MergedSelectionMap, NetworkProtocol,
    PathToRefetchFieldInfo, REFETCH_FIELD_NAME, RootRefetchedPath, Schema, ServerEntityName,
    WrappedSelectionMapSelection, fetchable_types, selection_map_wrapped,
};

use crate::{
    generate_artifacts::QUERY_TEXT, normalization_ast_text::generate_normalization_ast_text,
    operation_text::generate_operation_text, persisted_documents::PersistedDocuments,
};

#[expect(clippy::too_many_arguments)]
pub(crate) fn get_paths_and_contents_for_imperatively_loaded_field<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    file_extensions: GenerateFileExtensionsOption,
    persisted_documents: &mut Option<PersistedDocuments>,
    entrypoint: &ClientScalarSelectable<TNetworkProtocol>,
    root_refetch_path: RootRefetchedPath,
    nested_selection_map: &MergedSelectionMap,
    reachable_variables: &BTreeSet<VariableName>,
    index: usize,
) -> Vec<ArtifactPathAndContent> {
    let RootRefetchedPath {
        path_to_refetch_field_info,
        ..
    } = root_refetch_path;
    let PathToRefetchFieldInfo {
        wrap_refetch_field_with_inline_fragment: refetch_field_parent_object_entity_name,
        imperatively_loaded_field_variant,
        client_selectable_id,
    } = path_to_refetch_field_info;

    let client_selectable = schema.client_selectable(client_selectable_id).expect(
        "Expected selectable to exist. \
        This is indicative of a bug in Isograph.",
    );

    let client_selectable: &ClientSelectable<TNetworkProtocol> = &client_selectable;
    let ImperativelyLoadedFieldVariant {
        client_selection_name,
        root_object_entity_name,
        mut subfields_or_inline_fragments,
        top_level_schema_field_arguments,
        ..
    } = imperatively_loaded_field_variant;

    let normalization_ast_wrapped_selection_map = selection_map_wrapped(
        nested_selection_map.clone(),
        subfields_or_inline_fragments.clone(),
    );

    if let Some(refetch_field_parent_object_entity_name) = refetch_field_parent_object_entity_name {
        // This could be Pet
        subfields_or_inline_fragments.insert(
            0,
            WrappedSelectionMapSelection::InlineFragment(refetch_field_parent_object_entity_name),
        );
    }

    // TODO we need to extend this with variables used in subfields_or_inline_fragments
    let mut definitions_of_used_variables =
        get_used_variable_definitions(reachable_variables, client_selectable);

    for variable_definition in top_level_schema_field_arguments.iter() {
        definitions_of_used_variables.push(VariableDefinition {
            name: variable_definition.name,
            type_: variable_definition.type_.clone(),
            default_value: variable_definition.default_value.clone(),
        });
    }

    let root_parent_object = entrypoint.parent_object_entity_name();

    let root_operation_name = fetchable_types(db)
        .deref()
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .get(&root_object_entity_name)
        .cloned()
        .expect(
            "Expected root type to be fetchable here. \
            This is indicative of a bug in Isograph.",
        );

    let query_name = format!("{root_parent_object}__{client_selection_name}")
        .intern()
        .into();

    let query_text_selection_map_wrapped =
        selection_map_wrapped(nested_selection_map.clone(), subfields_or_inline_fragments);
    let root_fetchable_field = entrypoint.name();

    let query_text = TNetworkProtocol::generate_query_text(
        db,
        query_name,
        &query_text_selection_map_wrapped,
        definitions_of_used_variables.iter(),
        &root_operation_name,
        Format::Pretty,
    );

    let operation_text = generate_operation_text(
        db,
        query_name,
        &query_text_selection_map_wrapped,
        definitions_of_used_variables.iter(),
        &root_operation_name,
        root_object_entity_name,
        persisted_documents,
        1,
    );

    let normalization_ast_text = generate_normalization_ast_text(
        schema,
        normalization_ast_wrapped_selection_map.values(),
        1,
    );

    let file_name_prefix = format!("{}__{}.ts", *REFETCH_FIELD_NAME, index)
        .intern()
        .into();

    let query_text_file_name = format!(
        "{}__{}__{}{}",
        *REFETCH_FIELD_NAME,
        *QUERY_TEXT,
        index,
        file_extensions.ts()
    );

    let query_text_file_name_with_extension =
        format!("{}__{}__{}.ts", *REFETCH_FIELD_NAME, *QUERY_TEXT, index)
            .intern()
            .into();

    let imperatively_loaded_field_file_contents = format!(
        "import type {{ IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
        import queryText from './{query_text_file_name}';\n\n\
        const normalizationAst: NormalizationAst = {{\n\
        {}kind: \"NormalizationAst\",\n\
        {}selections: {normalization_ast_text},\n\
        }};\n\
        const artifact: RefetchQueryNormalizationArtifact = {{\n\
        {}kind: \"RefetchQuery\",\n\
        {}networkRequestInfo: {{\n\
        {}  kind: \"NetworkRequestInfo\",\n\
        {}  operation: {operation_text},\n\
        {}  normalizationAst,\n\
        {}}},\n\
        {}concreteType: \"{root_object_entity_name}\",\n\
        }};\n\n\
        export default artifact;\n",
        "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ",
    );

    vec![
        ArtifactPathAndContent {
            file_content: format!("export default '{query_text}';"),
            file_name: query_text_file_name_with_extension,
            type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                parent_object_entity_name: root_parent_object,
                selectable_name: root_fetchable_field.into(),
            }),
        },
        ArtifactPathAndContent {
            file_content: imperatively_loaded_field_file_contents,
            file_name: file_name_prefix,
            type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                parent_object_entity_name: root_parent_object,
                selectable_name: root_fetchable_field.into(),
            }),
        },
    ]
}

fn get_used_variable_definitions<TNetworkProtocol: NetworkProtocol + 'static>(
    reachable_variables: &BTreeSet<VariableName>,
    entrypoint: &ClientSelectable<TNetworkProtocol>,
) -> Vec<VariableDefinition<ServerEntityName>> {
    reachable_variables
        .iter()
        .flat_map(|variable_name| {
            // HACK
            if *variable_name == "id" {
                None
            } else {
                Some(
                    entrypoint
                        .variable_definitions()
                        .iter()
                        .find(|definition| definition.item.name.item == *variable_name)
                        .unwrap_or_else(|| {
                            panic!(
                                "Did not find matching variable definition. \
                                This might not be validated yet. For now, each client field \
                                containing a __refetch field must re-defined all used variables. \
                                Client field {} is missing variable definition {}",
                                entrypoint.name(),
                                variable_name
                            )
                        })
                        .item
                        .clone(),
                )
            }
        })
        .collect::<Vec<_>>()
}
