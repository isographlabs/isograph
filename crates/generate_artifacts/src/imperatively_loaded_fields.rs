use std::collections::BTreeSet;

use common_lang_types::{
    ArtifactPathAndContent, ParentObjectEntityNameAndSelectableName, VariableName,
};
use intern::string_key::Intern;
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::{RefetchQueryIndex, VariableDefinition};
use isograph_schema::{
    ClientScalarOrObjectSelectable, ClientScalarSelectable, ClientSelectable, Format,
    ImperativelyLoadedFieldVariant, MergedSelectionMap, NetworkProtocol, PathToRefetchFieldInfo,
    REFETCH_FIELD_NAME, RootRefetchedPath, Schema, ServerEntityName, WrappedSelectionMapSelection,
    selection_map_wrapped,
};

use crate::{
    generate_artifacts::QUERY_TEXT, normalization_ast_text::generate_normalization_ast_text,
    operation_text::generate_operation_text, persisted_documents::PersistedDocuments,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn get_paths_and_contents_for_imperatively_loaded_field<
    TNetworkProtocol: NetworkProtocol,
>(
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

    let client_selectable = schema.client_type(client_selectable_id).expect(
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
        let refetch_field_parent_type_name = schema
            .server_entity_data
            .server_object_entity(refetch_field_parent_object_entity_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .name;
        // This could be Pet
        subfields_or_inline_fragments.insert(
            0,
            WrappedSelectionMapSelection::InlineFragment(refetch_field_parent_type_name.item),
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

    let wrapped_selection_map =
        selection_map_wrapped(nested_selection_map.clone(), subfields_or_inline_fragments);

    let root_parent_object = schema
        .server_entity_data
        .server_object_entity(entrypoint.parent_object_entity_name())
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .name
        .item;

    let root_operation_name = schema
        .fetchable_types
        .get(&root_object_entity_name)
        .expect(
            "Expected root type to be fetchable here.\
            This is indicative of a bug in Isograph.",
        )
        .clone();

    let query_name = format!("{root_parent_object}__{client_selection_name}")
        .intern()
        .into();

    let merged_selection_set = wrapped_selection_map;
    let variable_definitions = definitions_of_used_variables;
    let root_fetchable_field = entrypoint.name();
    let refetch_query_index = RefetchQueryIndex(index as u32);
    let concrete_type = schema
        .server_entity_data
        .server_object_entity(root_object_entity_name)
        .expect(
            "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
        )
        .name
        .item;

    let query_text = TNetworkProtocol::generate_query_text(
        query_name,
        schema,
        &merged_selection_set,
        variable_definitions.iter(),
        &root_operation_name,
        Format::Pretty,
    );

    let operation_text = generate_operation_text(
        query_name,
        schema,
        &merged_selection_set,
        variable_definitions.iter(),
        &root_operation_name,
        concrete_type,
        persisted_documents,
        1,
    );

    let normalization_ast_text = generate_normalization_ast_text(
        schema,
        normalization_ast_wrapped_selection_map.values(),
        1,
    );

    let file_name_prefix = format!("{}__{}.ts", *REFETCH_FIELD_NAME, refetch_query_index.0)
        .intern()
        .into();

    let query_text_file_name = format!(
        "{}__{}__{}{}",
        *REFETCH_FIELD_NAME,
        *QUERY_TEXT,
        refetch_query_index.0,
        file_extensions.ts()
    );

    let query_text_file_name_with_extension = format!(
        "{}__{}__{}.ts",
        *REFETCH_FIELD_NAME, *QUERY_TEXT, refetch_query_index.0
    )
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
        {}concreteType: \"{concrete_type}\",\n\
        }};\n\n\
        export default artifact;\n",
        "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ", "  ",
    );

    vec![
        ArtifactPathAndContent {
            file_content: format!("export default '{query_text}';"),
            file_name: query_text_file_name_with_extension,
            type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                type_name: root_parent_object,
                field_name: root_fetchable_field.into(),
            }),
        },
        ArtifactPathAndContent {
            file_content: imperatively_loaded_field_file_contents,
            file_name: file_name_prefix,
            type_and_field: Some(ParentObjectEntityNameAndSelectableName {
                type_name: root_parent_object,
                field_name: root_fetchable_field.into(),
            }),
        },
    ]
}

fn get_used_variable_definitions<TNetworkProtocol: NetworkProtocol>(
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
