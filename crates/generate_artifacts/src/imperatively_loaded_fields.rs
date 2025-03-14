use common_lang_types::{
    ArtifactPathAndContent, ClientScalarSelectableName, IsographObjectTypeName,
    ObjectTypeAndFieldName, QueryText,
};
use intern::string_key::Intern;
use isograph_config::{GenerateFileExtensionsOption, PersistedDocumentsOptions};
use isograph_lang_types::RefetchQueryIndex;
use isograph_schema::{
    Format, ImperativelyLoadedFieldArtifactInfo, OutputFormat, ValidatedSchema, REFETCH_FIELD_NAME,
};

use crate::{
    entrypoint_artifact::PersistedDocumentEntry,
    generate_artifacts::{NormalizationAstText, OperationText, QueryTextImport, QUERY_TEXT},
    normalization_ast_text::generate_normalization_ast_text,
    operation_text::generate_operation_text,
};

#[derive(Debug)]
pub(crate) struct ImperativelyLoadedEntrypointArtifactInfo {
    pub normalization_ast_text: NormalizationAstText,
    pub root_fetchable_field: ClientScalarSelectableName,
    pub root_fetchable_field_parent_object: IsographObjectTypeName,
    pub refetch_query_index: RefetchQueryIndex,
    pub query_text_import: QueryTextImport,
    pub operation_text: OperationText,
    pub concrete_type: IsographObjectTypeName,
}

impl ImperativelyLoadedEntrypointArtifactInfo {
    pub fn path_and_content(self) -> ArtifactPathAndContent {
        let ImperativelyLoadedEntrypointArtifactInfo {
            root_fetchable_field,
            root_fetchable_field_parent_object,
            refetch_query_index,
            ..
        } = &self;

        let file_name_prefix = format!("{}__{}.ts", *REFETCH_FIELD_NAME, refetch_query_index.0)
            .intern()
            .into();

        let type_name = *root_fetchable_field_parent_object;
        let field_name = *root_fetchable_field;

        ArtifactPathAndContent {
            file_content: self.file_contents(),
            file_name: file_name_prefix,
            type_and_field: Some(ObjectTypeAndFieldName {
                type_name,
                field_name: field_name.into(),
            }),
        }
    }
}

impl ImperativelyLoadedEntrypointArtifactInfo {
    pub(crate) fn file_contents(self) -> String {
        let ImperativelyLoadedEntrypointArtifactInfo {
            normalization_ast_text: normalization_ast,
            query_text_import,
            operation_text,
            concrete_type,
            ..
        } = self;

        format!(
            "import type {{ IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            {query_text_import}\
            const normalizationAst: NormalizationAst = {{\n\
            {}kind: \"NormalizationAst\",\n\
            {}selections: {normalization_ast},\n\
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
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",

        )
    }
}

pub(crate) fn get_artifact_for_imperatively_loaded_field<TOutputFormat: OutputFormat>(
    schema: &ValidatedSchema<TOutputFormat>,
    imperatively_loaded_field_artifact_info: ImperativelyLoadedFieldArtifactInfo,
    file_extensions: GenerateFileExtensionsOption,
    persisted_documents_options: Option<&PersistedDocumentsOptions>,
) -> (Vec<ArtifactPathAndContent>, Vec<PersistedDocumentEntry>) {
    let ImperativelyLoadedFieldArtifactInfo {
        merged_selection_set,
        root_fetchable_field,
        root_parent_object,
        refetch_query_index,
        variable_definitions,
        root_operation_name,
        query_name,
        concrete_type,
    } = imperatively_loaded_field_artifact_info;

    let format = if persisted_documents_options.is_some() {
        Format::Compact
    } else {
        Format::Pretty
    };
    let query_text = TOutputFormat::generate_query_text(
        query_name,
        schema,
        &merged_selection_set,
        variable_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &root_operation_name,
        format,
    );

    let normalization_ast_text =
        generate_normalization_ast_text(schema, merged_selection_set.values(), 1);

    let mut paths_and_contents = vec![];
    let persist = persisted_documents_options.is_some();
    if !persist {
        paths_and_contents.push(get_query_text_path_and_content(
            &query_text,
            root_fetchable_field,
            root_parent_object,
            &refetch_query_index,
        ));
    }
    let query_text_import =
        generate_query_text_import(&refetch_query_index, file_extensions, persist);
    let (operation_text, persistent_documents) = generate_operation_text(
        query_name,
        concrete_type,
        query_text,
        persisted_documents_options,
        1,
    );

    paths_and_contents.push(
        ImperativelyLoadedEntrypointArtifactInfo {
            normalization_ast_text,
            query_text_import,
            operation_text,
            root_fetchable_field,
            root_fetchable_field_parent_object: root_parent_object,
            refetch_query_index,
            concrete_type,
        }
        .path_and_content(),
    );

    (paths_and_contents, persistent_documents)
}

fn generate_query_text_import(
    refetch_query_index: &RefetchQueryIndex,
    file_extensions: GenerateFileExtensionsOption,
    persist: bool,
) -> QueryTextImport {
    let ts_file_extension = file_extensions.ts();
    let query_text_file_name = format!(
        "{}__{}__{}.ts",
        *REFETCH_FIELD_NAME, *QUERY_TEXT, refetch_query_index.0,
    );
    let output = if persist {
        "".to_string()
    } else {
        format!("import queryText from './{query_text_file_name}{ts_file_extension}';\n")
    };
    QueryTextImport(output)
}

fn get_query_text_path_and_content(
    query_text: &QueryText,
    root_fetchable_field: ClientScalarSelectableName,
    type_name: IsographObjectTypeName,
    refetch_query_index: &RefetchQueryIndex,
) -> ArtifactPathAndContent {
    let field_name = root_fetchable_field.into();
    let file_name = format!(
        "{}__{}__{}.ts",
        *REFETCH_FIELD_NAME, *QUERY_TEXT, refetch_query_index.0,
    )
    .intern()
    .into();
    ArtifactPathAndContent {
        file_content: format!("export default '{}';", query_text),
        file_name,
        type_and_field: Some(ObjectTypeAndFieldName {
            type_name,
            field_name,
        }),
    }
}
