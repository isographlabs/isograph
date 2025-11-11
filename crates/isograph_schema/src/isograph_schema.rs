use std::{collections::HashMap, fmt::Debug, ops::Deref};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, JavascriptName, SelectableName,
    ServerObjectEntityName, ServerScalarEntityName, ServerScalarIdSelectableName,
    ServerScalarSelectableName, ServerSelectableName,
};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDirectiveSet, DefinitionLocation, EmptyDirectiveSet,
    ObjectSelection, ScalarSelection, SelectionType, SelectionTypeContainingSelections,
    VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, ClientSelectableId,
    EntrypointDeclarationInfo, IsographDatabase, NetworkProtocol, NormalizationKey,
    ObjectSelectable, ObjectSelectableId, ServerEntityName, ServerObjectEntityAvailableSelectables,
    ServerObjectSelectable, ServerScalarSelectable, UseRefetchFieldRefetchStrategy,
    create_additional_fields::{CreateAdditionalFieldsError, CreateAdditionalFieldsResult},
    server_object_selectable_named, server_selectable_named,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootOperationName(pub &'static str);

/// The in-memory representation of a schema.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Schema<TNetworkProtocol: NetworkProtocol> {
    pub client_scalar_selectables: HashMap<
        (ServerObjectEntityName, ClientScalarSelectableName),
        ClientScalarSelectable<TNetworkProtocol>,
    >,
    pub client_object_selectables: HashMap<
        (ServerObjectEntityName, ClientObjectSelectableName),
        ClientObjectSelectable<TNetworkProtocol>,
    >,
    pub entrypoints:
        HashMap<(ServerObjectEntityName, ClientScalarSelectableName), EntrypointDeclarationInfo>,
    pub server_entity_data: ServerEntityData,
}

impl<TNetworkProtocol: NetworkProtocol> Default for Schema<TNetworkProtocol> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    pub fn new() -> Self {
        Self {
            client_scalar_selectables: HashMap::new(),
            client_object_selectables: HashMap::new(),

            entrypoints: Default::default(),
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
        let memo_ref = server_selectable_named(db, current_entity_name, selection_name);

        let current_selectable = memo_ref.deref().as_ref().map_err(|e| e.clone())?;

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
    pub selectables: ServerObjectEntityAvailableSelectables,
    pub id_field: Option<ServerScalarIdSelectableName>,
}

// We keep track of available selectables and id fields outside of server_objects so that
// we don't need a server_object_entity_mut method, which is incompatible with pico.
//
// This type alias is a bit outdated. It should be gone soon anyway, though.
pub type ServerEntityData = HashMap<ServerObjectEntityName, ServerObjectEntityExtraInfo>;

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    // TODO this function should not exist
    pub fn insert_server_scalar_selectable(
        &mut self,
        server_scalar_selectable: ServerScalarSelectable<TNetworkProtocol>,
    ) -> CreateAdditionalFieldsResult<(), TNetworkProtocol> {
        let parent_object_entity_name = server_scalar_selectable.parent_object_entity_name;
        let next_scalar_name = server_scalar_selectable.name;

        let ServerObjectEntityExtraInfo { selectables, .. } = self
            .server_entity_data
            .entry(parent_object_entity_name)
            .or_default();

        if selectables
            .insert(
                next_scalar_name.item.into(),
                DefinitionLocation::Server(SelectionType::Scalar((
                    parent_object_entity_name,
                    server_scalar_selectable.name.item,
                ))),
            )
            .is_some()
        {
            return Err(CreateAdditionalFieldsError::DuplicateField {
                selectable_name: server_scalar_selectable.name.item.into(),
                parent_object_entity_name,
            });
        }

        Ok(())
    }

    // TODO this function should not exist
    pub fn insert_server_object_selectable(
        &mut self,
        server_object_selectable: ServerObjectSelectable<TNetworkProtocol>,
    ) -> CreateAdditionalFieldsResult<(), TNetworkProtocol> {
        let parent_object_entity_name = server_object_selectable.parent_object_entity_name;
        let next_object_name = server_object_selectable.name;

        if self
            .server_entity_data
            .entry(parent_object_entity_name)
            .or_default()
            .selectables
            .insert(
                next_object_name.item.into(),
                DefinitionLocation::Server(SelectionType::Object((
                    parent_object_entity_name,
                    server_object_selectable.name.item,
                ))),
            )
            .is_some()
        {
            return Err(CreateAdditionalFieldsError::DuplicateField {
                selectable_name: next_object_name.item.into(),
                parent_object_entity_name,
            });
        }

        Ok(())
    }

    pub fn client_scalar_selectable(
        &self,
        parent_type_name: ServerObjectEntityName,
        client_field_name: ClientScalarSelectableName,
    ) -> Option<&ClientScalarSelectable<TNetworkProtocol>> {
        self.client_scalar_selectables
            .get(&(parent_type_name, client_field_name))
    }

    // TODO this function should not exist
    pub fn client_scalar_selectable_mut(
        &mut self,
        parent_type_name: ServerObjectEntityName,
        client_field_name: ClientScalarSelectableName,
    ) -> Option<&mut ClientScalarSelectable<TNetworkProtocol>> {
        self.client_scalar_selectables
            .get_mut(&(parent_type_name, client_field_name))
    }

    pub fn object_selectable(
        &self,
        db: &IsographDatabase<TNetworkProtocol>,
        field_id: ObjectSelectableId,
    ) -> Option<ObjectSelectable<'_, TNetworkProtocol>> {
        match field_id {
            DefinitionLocation::Server((
                parent_object_entity_name,
                server_object_selectable_name,
            )) => {
                let memo_ref = server_object_selectable_named(
                    db,
                    parent_object_entity_name,
                    server_object_selectable_name.into(),
                );
                let server_object_selectable = memo_ref
                    .deref()
                    .as_ref()
                    .expect(
                        "Expected validation to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    )
                    .clone();

                server_object_selectable.map(DefinitionLocation::Server)
            }
            DefinitionLocation::Client((
                parent_object_entity_name,
                client_object_selectable_name,
            )) => self
                .client_object_selectable(parent_object_entity_name, client_object_selectable_name)
                .map(DefinitionLocation::Client),
        }
    }

    pub fn client_object_selectable(
        &self,
        parent_object_entity_name: ServerObjectEntityName,
        client_object_selectable_name: ClientObjectSelectableName,
    ) -> Option<&ClientObjectSelectable<TNetworkProtocol>> {
        self.client_object_selectables
            .get(&(parent_object_entity_name, client_object_selectable_name))
    }

    // TODO this function should not exist
    pub fn client_object_selectable_mut(
        &mut self,
        parent_object_entity_name: ServerObjectEntityName,
        client_object_selectable_name: ClientObjectSelectableName,
    ) -> Option<&mut ClientObjectSelectable<TNetworkProtocol>> {
        self.client_object_selectables
            .get_mut(&(parent_object_entity_name, client_object_selectable_name))
    }

    pub fn client_selectable(
        &self,
        client_selectable_id: ClientSelectableId,
    ) -> Option<
        SelectionType<
            &ClientScalarSelectable<TNetworkProtocol>,
            &ClientObjectSelectable<TNetworkProtocol>,
        >,
    > {
        match client_selectable_id {
            SelectionType::Scalar((parent_entity_object_name, client_field_name)) => self
                .client_scalar_selectable(parent_entity_object_name, client_field_name)
                .map(SelectionType::Scalar),
            SelectionType::Object((parent_entity_object_name, client_pointer_name)) => self
                .client_object_selectable(parent_entity_object_name, client_pointer_name)
                .map(SelectionType::Object),
        }
    }

    #[expect(clippy::type_complexity)]
    pub fn user_written_client_types(
        &self,
    ) -> impl Iterator<
        Item = (
            SelectionType<
                (ServerObjectEntityName, ClientScalarSelectableName),
                (ServerObjectEntityName, ClientObjectSelectableName),
            >,
            SelectionType<
                &ClientScalarSelectable<TNetworkProtocol>,
                &ClientObjectSelectable<TNetworkProtocol>,
            >,
            ClientFieldDirectiveSet,
        ),
    > {
        self.client_scalar_selectables
            .values()
            .flat_map(|field| match field.variant {
                ClientFieldVariant::Link => None,
                ClientFieldVariant::UserWritten(info) => Some((
                    SelectionType::Scalar((field.parent_object_entity_name, field.name.item)),
                    SelectionType::Scalar(field),
                    info.client_field_directive_set,
                )),
                ClientFieldVariant::ImperativelyLoadedField(_) => None,
            })
            .chain(self.client_object_selectables.values().map(|pointer| {
                (
                    SelectionType::Object((pointer.parent_object_entity_name, pointer.name.item)),
                    SelectionType::Object(pointer),
                    ClientFieldDirectiveSet::None(EmptyDirectiveSet {}),
                )
            }))
    }
}

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
