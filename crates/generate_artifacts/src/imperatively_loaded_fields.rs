use common_lang_types::{
    ArtifactPathAndContent, ClientScalarSelectableName, IsographObjectTypeName,
    ObjectTypeAndFieldName, QueryText,
};
use intern::string_key::Intern;
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::RefetchQueryIndex;
use isograph_schema::{
    ImperativelyLoadedFieldArtifactInfo, OutputFormat, Schema, REFETCH_FIELD_NAME,
};

use crate::{
    generate_artifacts::{NormalizationAstText, QUERY_TEXT},
    normalization_ast_text::generate_normalization_ast_text,
};

#[derive(Debug)]
pub(crate) struct ImperativelyLoadedEntrypointArtifactInfo {
    pub normalization_ast_text: NormalizationAstText,
    pub query_text: QueryText,
    pub root_fetchable_field: ClientScalarSelectableName,
    pub root_fetchable_field_parent_object: IsographObjectTypeName,
    pub refetch_query_index: RefetchQueryIndex,
    pub concrete_type: IsographObjectTypeName,
}

impl ImperativelyLoadedEntrypointArtifactInfo {
    pub fn path_and_content(
        self,
        file_extensions: GenerateFileExtensionsOption,
    ) -> Vec<ArtifactPathAndContent> {
        let ImperativelyLoadedEntrypointArtifactInfo {
            root_fetchable_field,
            root_fetchable_field_parent_object,
            refetch_query_index,
            query_text,
            ..
        } = &self;

        let file_name_prefix = format!("{}__{}.ts", *REFETCH_FIELD_NAME, refetch_query_index.0)
            .intern()
            .into();

        let query_text_file_name = format!(
            "{}__{}__{}.ts",
            *REFETCH_FIELD_NAME, *QUERY_TEXT, refetch_query_index.0
        )
        .intern()
        .into();

        let type_name = *root_fetchable_field_parent_object;
        let field_name = *root_fetchable_field;

        vec![
            ArtifactPathAndContent {
                file_content: format!("export default '{}';", query_text),
                file_name: query_text_file_name,
                type_and_field: Some(ObjectTypeAndFieldName {
                    type_name,
                    field_name: field_name.into(),
                }),
            },
            ArtifactPathAndContent {
                file_content: self.file_contents(file_extensions),
                file_name: file_name_prefix,
                type_and_field: Some(ObjectTypeAndFieldName {
                    type_name,
                    field_name: field_name.into(),
                }),
            },
        ]
    }
}

impl ImperativelyLoadedEntrypointArtifactInfo {
    pub(crate) fn file_contents(self, file_extensions: GenerateFileExtensionsOption) -> String {
        let ImperativelyLoadedEntrypointArtifactInfo {
            normalization_ast_text: normalization_ast,
            concrete_type,
            refetch_query_index,
            ..
        } = self;
        let ts_file_extension = file_extensions.ts();
        let query_text_file_name = format!(
            "{}__{}__{}",
            *REFETCH_FIELD_NAME, *QUERY_TEXT, refetch_query_index.0,
        );

        format!(
            "import type {{ IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            import queryText from './{query_text_file_name}{ts_file_extension}';\n\n\
            const normalizationAst: NormalizationAst = {{\n\
            {}kind: \"NormalizationAst\",\n\
            {}selections: {normalization_ast},\n\
            }};\n\
            const artifact: RefetchQueryNormalizationArtifact = {{\n\
            {}kind: \"RefetchQuery\",\n\
            {}networkRequestInfo: {{\n\
            {}  kind: \"NetworkRequestInfo\",\n\
            {}  queryText,\n\
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
    schema: &Schema<TOutputFormat>,
    imperatively_loaded_field_artifact_info: ImperativelyLoadedFieldArtifactInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> Vec<ArtifactPathAndContent> {
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

    let query_text = TOutputFormat::generate_query_text(
        query_name,
        schema,
        &merged_selection_set,
        variable_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &root_operation_name,
    );

    let normalization_ast_text =
        generate_normalization_ast_text(schema, merged_selection_set.values(), 1);

    ImperativelyLoadedEntrypointArtifactInfo {
        normalization_ast_text,
        query_text,
        root_fetchable_field,
        root_fetchable_field_parent_object: root_parent_object,
        refetch_query_index,
        concrete_type,
    }
    .path_and_content(file_extensions)
}
