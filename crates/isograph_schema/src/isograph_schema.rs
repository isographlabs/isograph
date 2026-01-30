use std::fmt::Debug;

use common_lang_types::{EntityName, SelectableName, VariableName};
use intern::string_key::Intern;
use isograph_lang_types::{ArgumentKeyAndValue, SelectionType};
use lazy_static::lazy_static;

use crate::NormalizationKey;

lazy_static! {
    pub static ref ID_ENTITY_NAME: EntityName = "ID".intern().into();
    pub static ref STRING_ENTITY_NAME: EntityName = "String".intern().into();
    pub static ref INT_ENTITY_NAME: EntityName = "Int".intern().into();
    pub static ref FLOAT_ENTITY_NAME: EntityName = "Float".intern().into();
    pub static ref BOOLEAN_ENTITY_NAME: EntityName = "Boolean".intern().into();
    pub static ref ID_FIELD_NAME: SelectableName = "id".intern().into();
    pub static ref ID_VARIABLE_NAME: VariableName = "id".intern().into();
    pub static ref TEXT_ENTITY_NAME: EntityName = "TEXT".intern().into();
    pub static ref INTEGER_ENTITY_NAME: EntityName = "INTEGER".intern().into();
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RootOperationName(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathToRefetchField {
    pub linked_fields: Vec<NormalizationKey>,
    pub field_name: SelectionType<SelectableName, NameAndArguments>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameAndArguments {
    pub name: SelectableName,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

impl NameAndArguments {
    pub fn normalization_key(&self) -> NormalizationKey {
        if self.name == *ID_FIELD_NAME {
            NormalizationKey::Id
        } else {
            NormalizationKey::ServerField(self.clone())
        }
    }
}
