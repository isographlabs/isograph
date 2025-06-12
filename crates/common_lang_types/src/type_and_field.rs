use crate::{ArtifactFilePrefix, SchemaServerObjectEntityName, SelectableName};

// TODO consider making this generic over the type of field_name. We sometimes know
// that the field is e.g. a scalar field
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct ObjectTypeAndFieldName {
    pub type_name: SchemaServerObjectEntityName,
    pub field_name: SelectableName,
}

impl ObjectTypeAndFieldName {
    pub fn underscore_separated(&self) -> String {
        format!("{}__{}", self.type_name, self.field_name)
    }

    pub fn relative_path(
        &self,
        current_file_type_name: SchemaServerObjectEntityName,
        file_type: ArtifactFilePrefix,
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
