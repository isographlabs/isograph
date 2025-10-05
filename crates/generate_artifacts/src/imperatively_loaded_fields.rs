use std::collections::BTreeSet;

use common_lang_types::{
    ArtifactPathAndContent, ParentObjectEntityNameAndSelectableName, VariableName,
};
use intern::string_key::Intern;
use isograph_config::GenerateFileExtensionsOption;
use isograph_schema::{
    ClientScalarSelectable, Format, ImperativelyLoadedFieldArtifactInfo, MergedSelectionMap,
    NetworkProtocol, PathToRefetchFieldInfo, REFETCH_FIELD_NAME, RootRefetchedPath, Schema,
    process_imperatively_loaded_field,
};

use crate::{
    generate_artifacts::QUERY_TEXT, normalization_ast_text::generate_normalization_ast_text,
    operation_text::generate_operation_text, persisted_documents::PersistedDocuments,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn get_artifact_for_imperatively_loaded_field<TNetworkProtocol: NetworkProtocol>(
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
        refetch_field_parent_object_entity_name,
        imperatively_loaded_field_variant,
        client_selectable_id,
        ..
    } = path_to_refetch_field_info;

    let client_selectable = schema.client_type(client_selectable_id).expect(
        "Expected selectable to exist. \
        This is indicative of a bug in Isograph.",
    );

    let ImperativelyLoadedFieldArtifactInfo {
        merged_selection_set,
        root_fetchable_field,
        root_parent_object,
        refetch_query_index,
        variable_definitions,
        root_operation_name,
        query_name,
        concrete_type,
    } = process_imperatively_loaded_field(
        schema,
        imperatively_loaded_field_variant,
        refetch_field_parent_object_entity_name,
        nested_selection_map,
        entrypoint,
        index,
        reachable_variables,
        &client_selectable,
    );

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

    let normalization_ast_text =
        generate_normalization_ast_text(schema, merged_selection_set.values(), 1);

    let file_name_prefix = format!("{}__{}.ts", *REFETCH_FIELD_NAME, refetch_query_index.0)
        .intern()
        .into();

    let query_text_file_name = format!(
        "{}__{}__{}.ts",
        *REFETCH_FIELD_NAME, *QUERY_TEXT, refetch_query_index.0
    )
    .intern()
    .into();

    let ts_file_extension = file_extensions.ts();

    let imperatively_loaded_field_file_contents = format!(
        "import type {{ IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
        import queryText from './{query_text_file_name}{ts_file_extension}';\n\n\
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
            file_name: query_text_file_name,
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
