use common_lang_types::{derive_display, IsographObjectTypeName, QueryOperationName};
use intern::string_key::Intern;
use isograph_config::PersistedDocumentsHashAlgorithm;
use isograph_schema::{
    Format, MergedSelectionMap, OutputFormat, RootOperationName, ValidatedSchema,
    ValidatedVariableDefinition,
};
use md5::{Digest, Md5};
use sha2::Sha256;

use crate::persisted_documents::PersistedDocuments;

#[derive(Debug)]
pub(crate) struct OperationText(pub String);
derive_display!(OperationText);

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_operation_text<'a, TOutputFormat: OutputFormat>(
    query_name: QueryOperationName,
    schema: &ValidatedSchema<TOutputFormat>,
    merged_selection_map: &MergedSelectionMap,
    query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    root_operation_name: &RootOperationName,
    operation_name: IsographObjectTypeName,
    persisted_documents: &mut Option<PersistedDocuments>,
    indentation_level: u8,
) -> OperationText {
    let indent = "  ".repeat((indentation_level + 1) as usize);
    let (document_id_str, query_text_str) = if let Some(persisted_documents) = persisted_documents {
        let query_text = TOutputFormat::generate_query_text(
            query_name,
            schema,
            merged_selection_map,
            query_variables,
            root_operation_name,
            Format::Compact,
        );
        let document_id = hash(query_text.0.as_str(), persisted_documents.options.algorithm)
            .intern()
            .into();
        persisted_documents
            .documents
            .insert(document_id, query_text);
        (format!("\"{document_id}\""), "null".to_string())
    } else {
        ("null".to_string(), "queryText".to_string())
    };
    OperationText(format!(
        "{{\n\
        {indent}  kind: \"Operation\",\n\
        {indent}  documentId: {document_id_str},\n\
        {indent}  operationName: \"{query_name}\",\n\
        {indent}  operationKind: \"{operation_name}\",\n\
        {indent}  text: {query_text_str},\n\
        {indent}}}"
    ))
}

fn hash(data: &str, algorithm: PersistedDocumentsHashAlgorithm) -> String {
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
