use serde::{Deserialize, Serialize};

use crate::{ArtifactFilePrefix, EntityName, SelectableName};

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy, Serialize, Deserialize)]
pub struct EntityNameAndSelectableName {
    pub parent_entity_name: EntityName,
    pub selectable_name: SelectableName,
}

impl EntityNameAndSelectableName {
    pub fn underscore_separated(&self) -> String {
        format!("{}__{}", self.parent_entity_name, self.selectable_name)
    }

    pub fn relative_path(
        &self,
        current_file_type_name: EntityName,
        file_type: ArtifactFilePrefix,
    ) -> String {
        let EntityNameAndSelectableName {
            parent_entity_name: type_name,
            selectable_name: field_name,
        } = *self;
        if type_name != current_file_type_name {
            format!("../../{type_name}/{field_name}/{file_type}")
        } else {
            format!("../{field_name}/{file_type}")
        }
    }

    pub fn new(parent_object_entity_name: EntityName, selectable_name: SelectableName) -> Self {
        Self {
            parent_entity_name: parent_object_entity_name,
            selectable_name,
        }
    }
}
