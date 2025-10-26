use crate::{ArtifactFilePrefix, SelectableName, ServerObjectEntityName};

// TODO consider making this generic over the type of field_name. We sometimes know
// that the field is e.g. a scalar field
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct ParentObjectEntityNameAndSelectableName {
    pub parent_object_entity_name: ServerObjectEntityName,
    pub selectable_name: SelectableName,
}

impl ParentObjectEntityNameAndSelectableName {
    pub fn underscore_separated(&self) -> String {
        format!(
            "{}__{}",
            self.parent_object_entity_name, self.selectable_name
        )
    }

    pub fn relative_path(
        &self,
        current_file_type_name: ServerObjectEntityName,
        file_type: ArtifactFilePrefix,
    ) -> String {
        let ParentObjectEntityNameAndSelectableName {
            parent_object_entity_name: type_name,
            selectable_name: field_name,
        } = *self;
        if type_name != current_file_type_name {
            format!("../../{type_name}/{field_name}/{file_type}")
        } else {
            format!("../{field_name}/{file_type}")
        }
    }
}
