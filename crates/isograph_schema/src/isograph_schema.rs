use std::fmt::Debug;

use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, SelectableName, VariableName};
use intern::string_key::Intern;
use isograph_lang_types::{ArgumentKeyAndValue, SelectionType};
use lazy_static::lazy_static;
use prelude::{ErrClone, Postfix};

use crate::{
    CompilationProfile, IsographDatabase, NormalizationKey, ServerSelectable, server_entity_named,
    server_selectable_named,
};

lazy_static! {
    pub static ref ID_ENTITY_NAME: EntityName = "ID".intern().into();
    pub static ref STRING_ENTITY_NAME: EntityName = "String".intern().into();
    pub static ref INT_ENTITY_NAME: EntityName = "Int".intern().into();
    pub static ref FLOAT_ENTITY_NAME: EntityName = "Float".intern().into();
    pub static ref BOOLEAN_ENTITY_NAME: EntityName = "Boolean".intern().into();
    pub static ref ID_FIELD_NAME: SelectableName = "id".intern().into();
    pub static ref ID_VARIABLE_NAME: VariableName = "id".intern().into();
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RootOperationName(pub &'static str);

pub fn get_object_selections_path<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    root_object_name: EntityName,
    selections: impl Iterator<Item = SelectableName>,
) -> DiagnosticResult<Vec<ServerSelectable<TCompilationProfile>>> {
    let mut path = vec![];
    let mut current_entity_name = root_object_name;

    for selection_name in selections {
        let current_selectable =
            server_selectable_named(db, current_entity_name, selection_name).clone_err()?;

        match current_selectable {
            Some(s) => {
                let selectable = s.lookup(db);
                let target_entity_name = selectable.target_entity_name.inner().0;
                let entity = server_entity_named(db, target_entity_name)
                    .clone_err()?
                    .ok_or_else(|| {
                        Diagnostic::new(
                            format!("Invalid field `{selection_name}` in @exposeField directive"),
                            // TODO have a location
                            None,
                        )
                    })?
                    .lookup(db);

                // TODO is this already validated?
                entity.selection_info.as_object().ok_or_else(|| {
                    Diagnostic::new(
                        "Expected selectable to be an object".to_string(),
                        // TODO have a location
                        None,
                    )
                })?;

                path.push(selectable.clone().note_todo("We should not clone here!!!"));
                current_entity_name = target_entity_name;
            }
            None => {
                return Diagnostic::new(
                    format!(
                        "Error when processing @exposeField directive \
                        on type `{current_entity_name}`. \
                        The field `{selection_name}` is not found."
                    ),
                    // TODO have a location
                    None,
                )
                .wrap_err();
            }
        };
    }

    path.wrap_ok()
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
// This struct is indicative of poor data modeling.
pub enum ServerObjectSelectableVariant {
    LinkedField,
    InlineFragment,
}
