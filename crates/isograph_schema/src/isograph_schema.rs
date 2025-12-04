use std::fmt::Debug;

use common_lang_types::{
    Diagnostic, DiagnosticResult, JavascriptName, SelectableName, ServerObjectEntityName,
    ServerScalarEntityName, VariableName,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, DefinitionLocation, ObjectSelection, ScalarSelection, SelectionType,
    SelectionTypeContainingSelections, VariableDefinition,
};
use lazy_static::lazy_static;
use prelude::{ErrClone, Postfix};

use crate::{
    IsographDatabase, NetworkProtocol, NormalizationKey, ObjectSelectableId, ServerEntityName,
    ServerObjectSelectable, server_selectable_named,
};

lazy_static! {
    pub static ref ID_ENTITY_NAME: ServerScalarEntityName = "ID".intern().into();
    pub static ref STRING_ENTITY_NAME: ServerScalarEntityName = "String".intern().into();
    pub static ref INT_ENTITY_NAME: ServerScalarEntityName = "Int".intern().into();
    pub static ref FLOAT_ENTITY_NAME: ServerScalarEntityName = "Float".intern().into();
    pub static ref BOOLEAN_ENTITY_NAME: ServerScalarEntityName = "Boolean".intern().into();
    pub static ref ID_FIELD_NAME: SelectableName = "id".intern().into();
    pub static ref ID_VARIABLE_NAME: VariableName = "id".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
    pub static ref BOOLEAN_JAVASCRIPT_TYPE: JavascriptName = "boolean".intern().into();
    pub static ref NUMBER_JAVASCRIPT_TYPE: JavascriptName = "number".intern().into();
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RootOperationName(pub &'static str);

pub fn get_object_selections_path<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    root_object_name: ServerObjectEntityName,
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
                        current_entity_name = *object.target_object_entity.inner();
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

pub type ValidatedSelection =
    SelectionTypeContainingSelections<ScalarSelectableId, ObjectSelectableId>;

pub type ValidatedObjectSelection = ObjectSelection<ScalarSelectableId, ObjectSelectableId>;

pub type ValidatedScalarSelection = ScalarSelection<ScalarSelectableId>;

pub type ValidatedVariableDefinition = VariableDefinition<ServerEntityName>;

pub type ScalarSelectableId = DefinitionLocation<
    (ServerObjectEntityName, SelectableName),
    (ServerObjectEntityName, SelectableName),
>;
