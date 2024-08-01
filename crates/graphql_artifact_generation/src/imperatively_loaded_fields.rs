use common_lang_types::{ArtifactPathAndContent, IsographObjectTypeName, SelectableFieldName};
use intern::string_key::Intern;
use isograph_lang_types::RefetchQueryIndex;
use isograph_schema::{ImperativelyLoadedFieldArtifactInfo, ValidatedSchema, REFETCH_FIELD_NAME};

use crate::{
    generate_artifacts::{generate_path, NormalizationAstText, QueryText},
    normalization_ast_text::generate_normalization_ast_text,
    query_text::generate_query_text,
};

#[derive(Debug)]
pub(crate) struct ImperativelyLoadedEntrypointArtifactInfo {
    pub normalization_ast_text: NormalizationAstText,
    pub query_text: QueryText,
    pub root_fetchable_field: SelectableFieldName,
    pub root_fetchable_field_parent_object: IsographObjectTypeName,
    pub refetch_query_index: RefetchQueryIndex,
}

impl ImperativelyLoadedEntrypointArtifactInfo {
    pub fn path_and_content(self) -> ArtifactPathAndContent {
        let ImperativelyLoadedEntrypointArtifactInfo {
            root_fetchable_field,
            root_fetchable_field_parent_object,
            refetch_query_index,
            ..
        } = &self;

        let relative_directory =
            generate_path(*root_fetchable_field_parent_object, *root_fetchable_field);
        let file_name_prefix = format!("{}__{}", *REFETCH_FIELD_NAME, refetch_query_index.0)
            .intern()
            .into();

        ArtifactPathAndContent {
            file_content: self.file_contents(),
            relative_directory,
            file_name_prefix,
        }
    }
}

impl ImperativelyLoadedEntrypointArtifactInfo {
    pub(crate) fn file_contents(self) -> String {
        let ImperativelyLoadedEntrypointArtifactInfo {
            normalization_ast_text: normalization_ast,
            query_text,
            ..
        } = self;

        format!(
            "import type {{ IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            const queryText = '{query_text}';\n\n\
            const normalizationAst: NormalizationAst = {normalization_ast};\n\
            const artifact: RefetchQueryNormalizationArtifact = {{\n\
            {}kind: \"RefetchQuery\",\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ",
            "  ",
            "  ",

        )
    }
}

pub(crate) fn get_artifact_for_imperatively_loaded_field(
    schema: &ValidatedSchema,
    imperatively_loaded_field_artifact_info: ImperativelyLoadedFieldArtifactInfo,
) -> ArtifactPathAndContent {
    let ImperativelyLoadedFieldArtifactInfo {
        merged_selection_set,
        root_fetchable_field,
        root_parent_object,
        refetch_query_index,
        variable_definitions,
        root_operation_name,
        query_name,
    } = imperatively_loaded_field_artifact_info;

    let query_text = generate_query_text(
        query_name,
        schema,
        &merged_selection_set,
        variable_definitions
            .iter()
            .map(|variable_definition| &variable_definition.item),
        &root_operation_name,
    );

    let normalization_ast_text =
        generate_normalization_ast_text(schema, merged_selection_set.values(), 0);

    ImperativelyLoadedEntrypointArtifactInfo {
        normalization_ast_text,
        query_text,
        root_fetchable_field,
        root_fetchable_field_parent_object: root_parent_object,
        refetch_query_index,
    }
    .path_and_content()
}
