use crate::{ArtifactFileType, IsographObjectTypeName, SelectableFieldName};

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct ObjectTypeAndFieldName {
    pub type_name: IsographObjectTypeName,
    pub field_name: SelectableFieldName,
}

impl ObjectTypeAndFieldName {
    pub fn underscore_separated(&self) -> String {
        format!("{}__{}", self.type_name, self.field_name)
    }

    pub fn relative_path(
        &self,
        current_file_type_name: IsographObjectTypeName,
        file_type: ArtifactFileType,
    ) -> String {
        let ObjectTypeAndFieldName {
            type_name,
            field_name,
        } = *self;
        if type_name != current_file_type_name {
            format!("../../{type_name}/{field_name}/{}", file_type)
        } else {
            format!("../{field_name}/{}", file_type)
        }
    }
}
