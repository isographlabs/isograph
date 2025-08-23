use std::collections::BTreeMap;

use common_lang_types::{ArtifactPathAndContent, OperationId, QueryText};
use indexmap::IndexMap;
use intern::string_key::Intern;
use isograph_config::PersistedDocumentsOptions;
use serde::Serialize;

use crate::generate_artifacts::PERSISTED_DOCUMENT_FILE_NAME;

pub struct PersistedDocuments<'a> {
    pub options: &'a PersistedDocumentsOptions,
    pub documents: BTreeMap<OperationId, QueryText>,
}

impl PersistedDocuments<'_> {
    pub fn path_and_content(self) -> ArtifactPathAndContent {
        let file_content = serde_json::to_string_pretty(&self)
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

impl Serialize for PersistedDocuments<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut entries: Vec<_> = self.documents.iter().collect();
        entries.sort_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)));

        let ordered: IndexMap<_, _> = entries.into_iter().collect();

        ordered.serialize(serializer)
    }
}
