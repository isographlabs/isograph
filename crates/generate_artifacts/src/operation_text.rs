use common_lang_types::{DocumentId, IsographObjectTypeName, QueryOperationName, QueryText};
use intern::string_key::Intern;
use isograph_config::{PersistedDocumentsHashAlgorithm, PersistedDocumentsOptions};
use md5::{Digest, Md5};
use sha2::Sha256;

use crate::generate_artifacts::OperationText;

pub(crate) fn generate_operation_text(
    query_name: QueryOperationName,
    operation_name: IsographObjectTypeName,
    query_text: QueryText,
    persisted_documents_options: Option<&PersistedDocumentsOptions>,
    indentation_level: u8,
) -> (OperationText, Vec<(DocumentId, QueryText)>) {
    let indent = "  ".repeat((indentation_level + 1) as usize);
    let mut persisted_documents = vec![];
    let (document_id_str, query_text_str) = if let Some(options) = persisted_documents_options {
        let document_id = hash(query_text.0.as_str(), options.algorithm)
            .intern()
            .into();
        persisted_documents.push((document_id, query_text));
        (format!("\"{document_id}\""), "null".to_string())
    } else {
        ("null".to_string(), "queryText".to_string())
    };
    (
        OperationText(format!(
            "{{\n\
            {indent}  kind: \"Operation\",\n\
            {indent}  documentId: {document_id_str},\n\
            {indent}  operationName: \"{query_name}\",\n\
            {indent}  operationKind: \"{operation_name}\",\n\
            {indent}  text: {query_text_str},\n\
            {indent}}}"
        )),
        persisted_documents,
    )
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
