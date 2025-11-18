use std::{collections::HashMap, fmt::Debug};

use common_lang_types::{
    ClientScalarSelectableName, JavascriptName, SelectableName, ServerObjectEntityName,
    ServerScalarEntityName, ServerScalarIdSelectableName, ServerScalarSelectableName,
    ServerSelectableName,
};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, DefinitionLocation, ObjectSelection, ScalarSelection, SelectionType,
    SelectionTypeContainingSelections, VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    IsographDatabase, NetworkProtocol, NormalizationKey, ObjectSelectableId, ServerEntityName,
    ServerObjectSelectable, UseRefetchFieldRefetchStrategy,
    create_additional_fields::CreateAdditionalFieldsError, server_selectable_named,
};

lazy_static! {
    pub static ref ID_ENTITY_NAME: ServerScalarEntityName = "ID".intern().into();
    pub static ref STRING_ENTITY_NAME: ServerScalarEntityName = "String".intern().into();
    pub static ref INT_ENTITY_NAME: ServerScalarEntityName = "Int".intern().into();
    pub static ref FLOAT_ENTITY_NAME: ServerScalarEntityName = "Float".intern().into();
    pub static ref BOOLEAN_ENTITY_NAME: ServerScalarEntityName = "Boolean".intern().into();
    pub static ref ID_FIELD_NAME: ServerScalarSelectableName = "id".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RootOperationName(pub &'static str);

/// The in-memory representation of a schema.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Schema {
    pub server_entity_data: ServerEntityData,
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Self {
            server_entity_data: HashMap::new(),
        }
    }
}

pub fn get_object_selections_path<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    root_object_name: ServerObjectEntityName,
    selections: impl Iterator<Item = ServerSelectableName>,
) -> Result<
    Vec<ServerObjectSelectable<TNetworkProtocol>>,
    CreateAdditionalFieldsError<TNetworkProtocol>,
> {
    let mut path = vec![];
    let mut current_entity_name = root_object_name;

    for selection_name in selections {
        let current_selectable = server_selectable_named(db, current_entity_name, selection_name)
            .as_ref()
            .map_err(|e| e.clone())?;

        match current_selectable {
            Some(entity) => {
                let entity = entity.as_ref().map_err(|e| e.clone())?;
                match entity {
                    SelectionType::Scalar(_) => {
                        // TODO show a better error message
                        return Err(CreateAdditionalFieldsError::InvalidField {
                            field_arg: selection_name.lookup().to_string(),
                        });
                    }
                    SelectionType::Object(object) => {
                        // TODO don't clone. When memoized functions return references with 'db lifetime,
                        // this will be doable.
                        path.push(object.clone());
                        current_entity_name = *object.target_object_entity.inner();
                    }
                }
            }
            None => {
                return Err(CreateAdditionalFieldsError::PrimaryDirectiveFieldNotFound {
                    primary_object_entity_name: current_entity_name,
                    field_name: selection_name.unchecked_conversion(),
                });
            }
        };
    }

    Ok(path)
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ServerObjectEntityExtraInfo {
    pub id_field: Option<ServerScalarIdSelectableName>,
}

// We keep track of available selectables and id fields outside of server_objects so that
// we don't need a server_object_entity_mut method, which is incompatible with pico.
//
// This type alias is a bit outdated. It should be gone soon anyway, though.
pub type ServerEntityData = HashMap<ServerObjectEntityName, ServerObjectEntityExtraInfo>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathToRefetchField {
    pub linked_fields: Vec<NormalizationKey>,
    pub field_name: SelectionType<ClientScalarSelectableName, NameAndArguments>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameAndArguments {
    pub name: SelectableName,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

impl NameAndArguments {
    pub fn normalization_key(&self) -> NormalizationKey {
        if self.name == "id" {
            NormalizationKey::Id
        } else {
            NormalizationKey::ServerField(self.clone())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub type ValidatedUseRefetchFieldStrategy =
    UseRefetchFieldRefetchStrategy<ScalarSelectableId, ObjectSelectableId>;

pub type ScalarSelectableId = DefinitionLocation<
    (ServerObjectEntityName, ServerScalarSelectableName),
    (ServerObjectEntityName, ClientScalarSelectableName),
>;
