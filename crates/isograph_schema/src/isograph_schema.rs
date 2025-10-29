use std::{collections::HashMap, fmt::Debug, ops::Deref};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, JavascriptName, ObjectSelectableName,
    SelectableName, ServerObjectEntityName, ServerObjectSelectableName, ServerScalarEntityName,
    ServerScalarIdSelectableName, ServerScalarSelectableName, UnvalidatedTypeName,
};
use graphql_lang_types::GraphQLNamedTypeAnnotation;
use intern::Lookup;
use intern::string_key::Intern;
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDirectiveSet, DefinitionLocation, EmptyDirectiveSet,
    ObjectSelection, ScalarSelection, SelectionType, SelectionTypeContainingSelections,
    VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, ClientSelectableId,
    EntrypointDeclarationInfo, IsographDatabase, NetworkProtocol, NormalizationKey,
    ObjectSelectable, ObjectSelectableId, ScalarSelectable, Selectable, SelectableId,
    ServerEntityName, ServerObjectEntityAvailableSelectables, ServerObjectSelectable,
    ServerScalarSelectable, ServerSelectableId, UseRefetchFieldRefetchStrategy,
    create_additional_fields::{CreateAdditionalFieldsError, CreateAdditionalFieldsResult},
    server_object_entity_named,
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
pub struct Schema<TNetworkProtocol: NetworkProtocol + 'static> {
    pub server_scalar_selectables: HashMap<
        (ServerObjectEntityName, ServerScalarSelectableName),
        ServerScalarSelectable<TNetworkProtocol>,
    >,
    pub server_object_selectables: HashMap<
        (ServerObjectEntityName, ServerObjectSelectableName),
        ServerObjectSelectable<TNetworkProtocol>,
    >,
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

impl<TNetworkProtocol: NetworkProtocol + 'static> Default for Schema<TNetworkProtocol> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static> Schema<TNetworkProtocol> {
    pub fn new() -> Self {
        Self {
            server_scalar_selectables: HashMap::new(),
            server_object_selectables: HashMap::new(),
            client_scalar_selectables: HashMap::new(),
            client_object_selectables: HashMap::new(),

            entrypoints: Default::default(),
            server_entity_data: ServerEntityData {
                server_object_entity_extra_info: HashMap::new(),
            },
        }
    }

    pub fn get_object_selections_path(
        &self,
        db: &IsographDatabase<TNetworkProtocol>,
        root_object_name: ServerObjectEntityName,
        selections: impl Iterator<Item = ObjectSelectableName>,
    ) -> Result<
        Vec<&ServerObjectSelectable<TNetworkProtocol>>,
        CreateAdditionalFieldsError<TNetworkProtocol>,
    > {
        let mut current_entity_memo_ref = server_object_entity_named(db, root_object_name);

        let mut current_selectables = &self
            .server_entity_data
            .server_object_entity_extra_info
            .get(&root_object_name)
            .expect(
                "Expected root_object_entity_name to exist \
                in server_object_entity_available_selectables",
            )
            .selectables;

        let mut path = vec![];

        for selection_name in selections {
            match current_selectables.get(&selection_name.into()) {
                Some(entity) => match entity.transpose() {
                    SelectionType::Scalar(_) => {
                        // TODO show a better error message
                        return Err(CreateAdditionalFieldsError::InvalidField {
                            field_arg: selection_name.lookup().to_string(),
                        });
                    }
                    SelectionType::Object(object) => {
                        let target_object_entity_name = match object {
                            DefinitionLocation::Server((
                                parent_object_entity_name,
                                server_object_selectable_name,
                            )) => {
                                let selectable = self
                                    .server_object_selectable(
                                        *parent_object_entity_name,
                                        *server_object_selectable_name,
                                    )
                                    .expect(
                                        "Expected selectable to exist. \
                                        This is indicative of a bug in Isograph.",
                                    );
                                path.push(selectable);
                                selectable.target_object_entity.inner()
                            }

                            DefinitionLocation::Client(_) => {
                                // TODO better error, or support client fields
                                return Err(CreateAdditionalFieldsError::InvalidField {
                                    field_arg: selection_name.lookup().to_string(),
                                });
                            }
                        };

                        current_entity_memo_ref =
                            server_object_entity_named(db, *target_object_entity_name);

                        current_selectables = &self
                            .server_entity_data
                            .server_object_entity_extra_info
                            .get(target_object_entity_name)
                            .expect(
                                "Expected target_object_entity_name to exist \
                                in server_object_entity_available_selectables",
                            )
                            .selectables;
                    }
                },
                None => {
                    return Err(CreateAdditionalFieldsError::PrimaryDirectiveFieldNotFound {
                        primary_object_entity_name: current_entity_memo_ref
                            .deref()
                            .as_ref()
                            .expect(
                                "Expected validation to have succeeded. \
                                This is indicative of a bug in Isograph.",
                            )
                            .as_ref()
                            .expect(
                                "Expected entity to exist. \
                                This is indicative of a bug in Isograph.",
                            )
                            .item
                            .name
                            .item,
                        field_name: selection_name.unchecked_conversion(),
                    });
                }
            };
        }

        Ok(path)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ServerObjectEntityExtraInfo {
    pub selectables: ServerObjectEntityAvailableSelectables,
    pub id_field: Option<ServerScalarIdSelectableName>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerEntityData {
    // We keep track of available selectables and id fields outside of server_objects so that
    // we don't need a server_object_entity_mut method, which is incompatible with pico.
    pub server_object_entity_extra_info:
        HashMap<ServerObjectEntityName, ServerObjectEntityExtraInfo>,
}

impl<TNetworkProtocol: NetworkProtocol + 'static> Schema<TNetworkProtocol> {
    pub fn server_scalar_selectable(
        &self,
        parent_object_entity_name: ServerObjectEntityName,
        server_scalar_selectable_name: ServerScalarSelectableName,
    ) -> Option<&ServerScalarSelectable<TNetworkProtocol>> {
        self.server_scalar_selectables
            .get(&(parent_object_entity_name, server_scalar_selectable_name))
    }

    pub fn server_object_selectable(
        &self,
        parent_object_entity_name: ServerObjectEntityName,
        server_object_selectable_name: ServerObjectSelectableName,
    ) -> Option<&ServerObjectSelectable<TNetworkProtocol>> {
        self.server_object_selectables
            .get(&(parent_object_entity_name, server_object_selectable_name))
    }

    pub fn server_selectable(
        &'_ self,
        server_selectable_id: ServerSelectableId,
    ) -> Option<
        SelectionType<
            &ServerScalarSelectable<TNetworkProtocol>,
            &ServerObjectSelectable<TNetworkProtocol>,
        >,
    > {
        match server_selectable_id {
            SelectionType::Scalar((parent_object_entity_name, server_scalar_selectable_name)) => {
                self.server_scalar_selectable(
                    parent_object_entity_name,
                    server_scalar_selectable_name,
                )
                .map(SelectionType::Scalar)
            }
            SelectionType::Object((parent_object_entity_name, server_object_selectable_name)) => {
                self.server_object_selectable(
                    parent_object_entity_name,
                    server_object_selectable_name,
                )
                .map(SelectionType::Object)
            }
        }
    }

    // TODO this function should not exist
    pub fn insert_server_scalar_selectable(
        &mut self,
        server_scalar_selectable: ServerScalarSelectable<TNetworkProtocol>,
        // TODO do not accept this
        options: &CompilerConfigOptions,
        inner_non_null_named_type: Option<&GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
    ) -> CreateAdditionalFieldsResult<(), TNetworkProtocol> {
        let parent_object_entity_name = server_scalar_selectable.parent_object_entity_name;
        let next_scalar_name = server_scalar_selectable.name;

        let ServerObjectEntityExtraInfo {
            selectables,
            id_field,
            ..
        } = self
            .server_entity_data
            .server_object_entity_extra_info
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

        // TODO do not do this here, this is a GraphQL-ism
        if server_scalar_selectable.name.item == "id" {
            set_and_validate_id_field::<TNetworkProtocol>(
                id_field,
                server_scalar_selectable.name.item,
                parent_object_entity_name,
                options,
                inner_non_null_named_type,
            )?;
        }

        self.server_scalar_selectables.insert(
            (
                server_scalar_selectable.parent_object_entity_name,
                server_scalar_selectable.name.item,
            ),
            server_scalar_selectable,
        );

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
            .server_object_entity_extra_info
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

        self.server_object_selectables.insert(
            (
                parent_object_entity_name,
                server_object_selectable.name.item,
            ),
            server_object_selectable,
        );

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
        field_id: ObjectSelectableId,
    ) -> Option<ObjectSelectable<'_, TNetworkProtocol>> {
        match field_id {
            DefinitionLocation::Server((
                parent_object_entity_name,
                server_object_selectable_name,
            )) => self
                .server_object_selectable(parent_object_entity_name, server_object_selectable_name)
                .map(DefinitionLocation::Server),
            DefinitionLocation::Client((
                parent_object_entity_name,
                client_object_selectable_name,
            )) => self
                .client_object_selectable(parent_object_entity_name, client_object_selectable_name)
                .map(DefinitionLocation::Client),
        }
    }

    pub fn scalar_selectable(
        &self,
        field_id: ScalarSelectableId,
    ) -> Option<ScalarSelectable<'_, TNetworkProtocol>> {
        match field_id {
            DefinitionLocation::Server((
                parent_object_entity_name,
                server_scalar_selectable_name,
            )) => self
                .server_scalar_selectable(parent_object_entity_name, server_scalar_selectable_name)
                .map(DefinitionLocation::Server),
            DefinitionLocation::Client((
                parent_object_entity_name,
                client_scalar_selectable_name,
            )) => self
                .client_scalar_selectable(parent_object_entity_name, client_scalar_selectable_name)
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

    pub fn selectable(&self, id: SelectableId) -> Option<Selectable<'_, TNetworkProtocol>> {
        match id {
            DefinitionLocation::Server(server_selectable_id) => Some(DefinitionLocation::Server(
                self.server_selectable(server_selectable_id)?,
            )),
            DefinitionLocation::Client(client_selectable_id) => Some(DefinitionLocation::Client(
                self.client_selectable(client_selectable_id)?,
            )),
        }
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

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field<TNetworkProtocol: NetworkProtocol + 'static>(
    id_field: &mut Option<ServerScalarIdSelectableName>,
    current_field_selectable_name: ServerScalarSelectableName,
    parent_object_entity_name: ServerObjectEntityName,
    options: &CompilerConfigOptions,
    inner_non_null_named_type: Option<&GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
) -> CreateAdditionalFieldsResult<(), TNetworkProtocol> {
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_selectable_name.unchecked_conversion());

    match inner_non_null_named_type {
        Some(type_) => {
            if type_.0.item != *ID_ENTITY_NAME {
                options.on_invalid_id_type.on_failure(|| {
                    CreateAdditionalFieldsError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_object_entity_name,
                    }
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                CreateAdditionalFieldsError::IdFieldMustBeNonNullIdType {
                    strong_field_name: "id",
                    parent_object_entity_name,
                }
            })?;
            Ok(())
        }
    }
}
