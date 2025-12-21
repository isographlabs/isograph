use std::fmt::Debug;
use std::sync::LazyLock;

use common_lang_types::{
    Diagnostic, DiagnosticResult, EntityName, JavascriptName, SelectableName, VariableName,
};
use intern::string_key::Intern;
use isograph_lang_types::{ArgumentKeyAndValue, SelectionType};
use prelude::{ErrClone, Postfix};

use crate::{
    IsographDatabase, NetworkProtocol, NormalizationKey, ServerObjectSelectable,
    server_selectable_named,
};

pub static ID_ENTITY_NAME: LazyLock<EntityName> = LazyLock::new(|| "ID".intern().into());
pub static STRING_ENTITY_NAME: LazyLock<EntityName> = LazyLock::new(|| "String".intern().into());
pub static INT_ENTITY_NAME: LazyLock<EntityName> = LazyLock::new(|| "Int".intern().into());
pub static FLOAT_ENTITY_NAME: LazyLock<EntityName> = LazyLock::new(|| "Float".intern().into());
pub static BOOLEAN_ENTITY_NAME: LazyLock<EntityName> = LazyLock::new(|| "Boolean".intern().into());
pub static ID_FIELD_NAME: LazyLock<SelectableName> = LazyLock::new(|| "id".intern().into());
pub static ID_VARIABLE_NAME: LazyLock<VariableName> = LazyLock::new(|| "id".intern().into());
pub static STRING_JAVASCRIPT_TYPE: LazyLock<JavascriptName> =
    LazyLock::new(|| "string".intern().into());
pub static BOOLEAN_JAVASCRIPT_TYPE: LazyLock<JavascriptName> =
    LazyLock::new(|| "boolean".intern().into());
pub static NUMBER_JAVASCRIPT_TYPE: LazyLock<JavascriptName> =
    LazyLock::new(|| "number".intern().into());

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RootOperationName(pub &'static str);

pub fn get_object_selections_path<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    root_object_name: EntityName,
    selections: impl Iterator<Item = SelectableName>,
) -> DiagnosticResult<Vec<ServerObjectSelectable<TNetworkProtocol>>> {
    let mut path = vec![];
    let mut current_entity_name = root_object_name;

    for selection_name in selections {
        let current_selectable =
            server_selectable_named(db, current_entity_name, selection_name).clone_err()?;

        match current_selectable {
            Some(entity) => {
                match entity {
                    SelectionType::Scalar(_) => {
                        // TODO show a better error message
                        return Diagnostic::new(
                            format!("Invalid field `{selection_name}` in @exposeField directive"),
                            // TODO have a location
                            None,
                        )
                        .wrap_err();
                    }
                    SelectionType::Object(object) => {
                        let object = object.lookup(db);
                        path.push(object.clone().note_todo("We should not clone here!!!"));
                        current_entity_name = object.target_entity_name.item.inner().0;
                    }
                }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// This struct is indicative of poor data modeling.
pub enum ServerObjectSelectableVariant {
    LinkedField,
    InlineFragment,
}
