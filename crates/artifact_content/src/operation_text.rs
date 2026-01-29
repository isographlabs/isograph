use common_lang_types::{EntityName, QueryExtraInfo, QueryOperationName, derive_display};
use intern::string_key::Intern;
use isograph_config::PersistedDocumentsHashAlgorithm;
use isograph_lang_types::VariableDeclaration;
use isograph_schema::{
    CompilationProfile, Format, IsographDatabase, MergedSelectionMap, NetworkProtocol,
};
use md5::{Digest, Md5};
use sha2::Sha256;

use crate::persisted_documents::PersistedDocuments;

#[derive(Debug)]
pub(crate) struct OperationText(pub String);
derive_display!(OperationText);

pub(crate) fn generate_operation_text<'a, TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    query_name: QueryOperationName,
    merged_selection_map: &MergedSelectionMap,
    query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
    root_entity: EntityName,
    persisted_documents: &mut Option<PersistedDocuments>,
    indentation_level: u8,
) -> OperationText {
    let indent = "  ".repeat((indentation_level + 1) as usize);
    match persisted_documents {
        Some(pd) => {
            let query_text = TCompilationProfile::NetworkProtocol::generate_query_text(
                db,
                root_entity,
                query_name,
                merged_selection_map,
                query_variables,
                Format::Compact,
            );
            let operation_id = hash(query_text.0.as_str(), pd.options.algorithm)
                .intern()
                .into();
            pd.documents.insert(operation_id, query_text);
            let query_extra_info = if pd.options.include_extra_info {
                TCompilationProfile::NetworkProtocol::generate_query_extra_info(
                    query_name,
                    root_entity,
                    indentation_level + 1,
                )
            } else {
                QueryExtraInfo("null".to_string())
            };
            OperationText(format!(
                "{{\n\
                {indent}  kind: \"PersistedOperation\",\n\
                {indent}  operationId: \"{operation_id}\",\n\
                {indent}  extraInfo: {query_extra_info},\n\
                {indent}}}"
            ))
        }
        None => OperationText(format!(
            "{{\n\
            {indent}  kind: \"Operation\",\n\
            {indent}  text: queryText,\n\
            {indent}}}"
        )),
    }
}

pub fn hash(data: &str, algorithm: PersistedDocumentsHashAlgorithm) -> String {
    match algorithm {
        PersistedDocumentsHashAlgorithm::Md5 => {
            let mut md5 = Md5::new();
            md5.update(data);
            hex::encode(md5.finalize())
        }
        PersistedDocumentsHashAlgorithm::Sha256 => {
            let mut sha256 = Sha256::new();
            sha256.update(data);
            hex::encode(sha256.finalize())
        }
    }
}
