use std::collections::BTreeMap;

use common_lang_types::{ArtifactPathAndContent, DocumentId, QueryText};
use intern::string_key::Intern;
use isograph_config::PersistedDocumentsOptions;

use crate::generate_artifacts::PERSISTED_DOCUMENT_FILE_NAME;

pub struct PersistedDocuments<'a> {
    pub options: &'a PersistedDocumentsOptions,
    pub documents: BTreeMap<DocumentId, QueryText>,
}

impl PersistedDocuments<'_> {
    pub fn path_and_content(self) -> ArtifactPathAndContent {
        let file_content = serde_json::to_string_pretty(&self.documents)
            .expect("expected persisted documents to be serializable");
        let file_name = self
            .options
            .file
            .as_ref()
            .map(|file| {
                file.to_str()
                    .expect("Expected path to be able to be stringified.")
                    .intern()
                    .into()
            })
            .unwrap_or(*PERSISTED_DOCUMENT_FILE_NAME);

        ArtifactPathAndContent {
            file_content,
            file_name,
            type_and_field: None,
        }
    }
}
