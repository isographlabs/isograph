use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use common_lang_types::{
    ClientScalarSelectableName, JavascriptName, Location, ObjectSelectableName,
    SchemaServerObjectEntityName, SchemaServerScalarEntityName, SelectableName,
    UnvalidatedTypeName, WithLocation,
};
use graphql_lang_types::GraphQLNamedTypeAnnotation;
use intern::string_key::Intern;
use intern::Lookup;
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDirectiveSet, ClientObjectSelectableId, DefinitionLocation,
    EmptyDirectiveSet, ObjectSelection, ScalarSelection, SelectionType,
    SelectionTypeContainingSelections, ServerEntityName, ServerObjectSelectableId,
    ServerScalarSelectableId, ServerStrongIdFieldId, VariableDefinition, WithId,
};
use lazy_static::lazy_static;

use crate::{
    create_additional_fields::{CreateAdditionalFieldsError, CreateAdditionalFieldsResult},
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, ClientSelectableId,
    EntrypointDeclarationInfo, NetworkProtocol, NormalizationKey, ObjectSelectable,
    ObjectSelectableId, ServerEntity, ServerObjectEntity, ServerObjectEntityAvailableSelectables,
    ServerObjectSelectable, ServerScalarEntity, ServerScalarSelectable, ServerSelectable,
    ServerSelectableId, UseRefetchFieldRefetchStrategy,
};

lazy_static! {
    pub static ref ID_GRAPHQL_TYPE: SchemaServerScalarEntityName = "ID".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug, Clone)]
pub struct RootOperationName(pub String);

/// The in-memory representation of a schema.
#[derive(Debug)]
pub struct Schema<TNetworkProtocol: NetworkProtocol> {
    pub server_scalar_selectables: Vec<ServerScalarSelectable<TNetworkProtocol>>,
    pub server_object_selectables: Vec<ServerObjectSelectable<TNetworkProtocol>>,
    pub client_scalar_selectables: HashMap<
        (SchemaServerObjectEntityName, ClientScalarSelectableName),
        ClientScalarSelectable<TNetworkProtocol>,
    >,
    pub client_object_selectables: Vec<ClientObjectSelectable<TNetworkProtocol>>,
    pub entrypoints: HashMap<
        (SchemaServerObjectEntityName, ClientScalarSelectableName),
        EntrypointDeclarationInfo,
    >,
    pub server_entity_data: ServerEntityData<TNetworkProtocol>,

    /// These are root types like Query, Mutation, Subscription
    // TODO remove??? It's a GraphQL-ism
    pub fetchable_types: BTreeMap<SchemaServerObjectEntityName, RootOperationName>,
}

impl<TNetworkProtocol: NetworkProtocol> Default for Schema<TNetworkProtocol> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    pub fn new() -> Self {
        // TODO add __typename
        let mut scalars = HashMap::new();
        let mut defined_types = HashMap::new();

        let id_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "ID",
            *STRING_JAVASCRIPT_TYPE,
        );
        let string_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "String",
            *STRING_JAVASCRIPT_TYPE,
        );
        let boolean_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "Boolean",
            "boolean".intern().into(),
        );
        let float_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "Float",
            "number".intern().into(),
        );
        let int_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "Int",
            "number".intern().into(),
        );
        let null_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            // The Null type should never be printed, at least for GraphQL.
            // TODO we should make this an Option and emit an error (or less
            // ideally, panic) if this is printed.
            "NullDoesNotExistIfThisIsPrintedThisIsABug",
            "number".intern().into(),
        );

        Self {
            server_scalar_selectables: vec![],
            server_object_selectables: vec![],
            client_scalar_selectables: HashMap::new(),
            client_object_selectables: vec![],

            entrypoints: Default::default(),
            server_entity_data: ServerEntityData {
                server_objects: HashMap::new(),
                server_scalars: scalars,
                defined_entities: defined_types,
                server_object_entity_extra_info: HashMap::new(),

                id_type_id,
                string_type_id,
                int_type_id,
                float_type_id,
                boolean_type_id,
                null_type_id,
            },
            fetchable_types: BTreeMap::new(),
        }
    }

    /// This is a smell, and we should refactor away from it, or all schema's
    /// should have a root type.
    pub fn query_id(&self) -> SchemaServerObjectEntityName {
        *self
            .fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
            .expect("Expected query to be found")
            .0
    }

    pub fn find_mutation(&self) -> Option<(&SchemaServerObjectEntityName, &RootOperationName)> {
        self.fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "mutation")
    }

    pub fn find_query(&self) -> Option<(&SchemaServerObjectEntityName, &RootOperationName)> {
        self.fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
    }

    pub fn traverse_object_selections(
        &self,
        root_object_name: SchemaServerObjectEntityName,
        selections: impl Iterator<Item = ObjectSelectableName>,
    ) -> Result<WithId<&ServerObjectEntity<TNetworkProtocol>>, CreateAdditionalFieldsError> {
        let mut current_entity = self
            .server_entity_data
            .server_object_entity(root_object_name);
        let mut current_selectables = &self
            .server_entity_data
            .server_object_entity_extra_info
            .get(&root_object_name)
            .expect(
                "Expected root_object_entity_name to exist \
                in server_object_entity_avaiable_selectables",
            )
            .selectables;

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
                                _parent_object_entity_name,
                                server_object_selectable_id,
                            )) => {
                                let selectable =
                                    self.server_object_selectable(*server_object_selectable_id);
                                selectable.target_object_entity.inner()
                            }
                            DefinitionLocation::Client((
                                _parent_object_entity_name,
                                client_object_selectable_id,
                            )) => {
                                let pointer = self.client_pointer(*client_object_selectable_id);
                                pointer.target_object_entity_name.inner()
                            }
                        };

                        current_entity = self
                            .server_entity_data
                            .server_object_entity(*target_object_entity_name);
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
                        primary_type_name: current_entity.name,
                        field_name: selection_name.unchecked_conversion(),
                    })
                }
            };
        }

        Ok(WithId::new(current_entity.name, current_entity))
    }

    pub fn get_object_selections_path(
        &self,
        root_object_name: SchemaServerObjectEntityName,
        selections: impl Iterator<Item = ObjectSelectableName>,
    ) -> Result<Vec<&ServerObjectSelectable<TNetworkProtocol>>, CreateAdditionalFieldsError> {
        let mut current_entity = self
            .server_entity_data
            .server_object_entity(root_object_name);

        let mut current_selectables = &self
            .server_entity_data
            .server_object_entity_extra_info
            .get(&root_object_name)
            .expect(
                "Expected root_object_entity_name to exist \
                in server_object_entity_avaiable_selectables",
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
                                _parent_object_entity_name,
                                server_object_selectable_id,
                            )) => {
                                let selectable =
                                    self.server_object_selectable(*server_object_selectable_id);
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

                        current_entity = self
                            .server_entity_data
                            .server_object_entity(*target_object_entity_name);
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
                        primary_type_name: current_entity.name,
                        field_name: selection_name.unchecked_conversion(),
                    })
                }
            };
        }

        Ok(path)
    }
}

#[derive(Debug, Default)]
pub struct ServerObjectEntityExtraInfo {
    pub selectables: ServerObjectEntityAvailableSelectables,
    pub id_field: Option<ServerStrongIdFieldId>,
}

#[derive(Debug)]
pub struct ServerEntityData<TNetworkProtocol: NetworkProtocol> {
    // TODO consider combining these.
    pub server_objects: HashMap<SchemaServerObjectEntityName, ServerObjectEntity<TNetworkProtocol>>,
    pub server_scalars: HashMap<SchemaServerScalarEntityName, ServerScalarEntity<TNetworkProtocol>>,

    // TODO consider whether this is needed. Especially when server_objects and server_scalars
    // are combined, this seems pretty useless.
    pub defined_entities: HashMap<UnvalidatedTypeName, ServerEntityName>,

    // We keep track of available selectables and id fields outside of server_objects so that
    // we don't need a server_object_entity_mut method, which is incompatible with pico.
    pub server_object_entity_extra_info:
        HashMap<SchemaServerObjectEntityName, ServerObjectEntityExtraInfo>,

    // TODO remove. These are GraphQL-isms. And we can just hard code them, they're
    // just interned strings!
    // Well known types
    pub id_type_id: SchemaServerScalarEntityName,
    pub string_type_id: SchemaServerScalarEntityName,
    pub float_type_id: SchemaServerScalarEntityName,
    pub boolean_type_id: SchemaServerScalarEntityName,
    pub int_type_id: SchemaServerScalarEntityName,
    // TODO restructure UnionTypeAnnotation to not have a nullable field, but to instead
    // include null in its variants.
    pub null_type_id: SchemaServerScalarEntityName,
}

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    pub fn server_scalar_selectable(
        &self,
        server_scalar_selectable_id: ServerScalarSelectableId,
    ) -> &ServerScalarSelectable<TNetworkProtocol> {
        &self.server_scalar_selectables[server_scalar_selectable_id.as_usize()]
    }

    pub fn server_scalar_selectables_and_ids(
        &self,
    ) -> impl Iterator<Item = WithId<&ServerScalarSelectable<TNetworkProtocol>>> {
        self.server_scalar_selectables
            .iter()
            .enumerate()
            .map(|(id, scalar)| WithId::new(id.into(), scalar))
    }

    pub fn server_object_selectable(
        &self,
        server_object_selectable_id: ServerObjectSelectableId,
    ) -> &ServerObjectSelectable<TNetworkProtocol> {
        &self.server_object_selectables[server_object_selectable_id.as_usize()]
    }

    pub fn server_object_selectables_and_ids(
        &self,
    ) -> impl Iterator<Item = WithId<&ServerObjectSelectable<TNetworkProtocol>>> {
        self.server_object_selectables
            .iter()
            .enumerate()
            .map(|(id, object)| WithId::new(id.into(), object))
    }

    pub fn server_selectable(
        &self,
        server_selectable_id: ServerSelectableId,
    ) -> ServerSelectable<TNetworkProtocol> {
        match server_selectable_id {
            SelectionType::Scalar((_parent_object_entity_name, server_scalar_selectable_id)) => {
                SelectionType::Scalar(self.server_scalar_selectable(server_scalar_selectable_id))
            }
            SelectionType::Object((_parent_object_entity_name, server_object_selectable_id)) => {
                SelectionType::Object(self.server_object_selectable(server_object_selectable_id))
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
    ) -> CreateAdditionalFieldsResult<()> {
        let next_server_scalar_selectable_id = self.server_scalar_selectables.len().into();
        let parent_object_entity_name = server_scalar_selectable.parent_object_entity_name;
        let next_scalar_name = server_scalar_selectable.name;

        let parent_type_name = self
            .server_entity_data
            .server_object_entity(parent_object_entity_name)
            .name;

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
                    parent_type_name,
                    next_server_scalar_selectable_id,
                ))),
            )
            .is_some()
        {
            let parent_object = self
                .server_entity_data
                .server_object_entity(parent_object_entity_name);
            return Err(CreateAdditionalFieldsError::DuplicateField {
                field_name: server_scalar_selectable.name.item.into(),
                parent_type: parent_object.name,
            });
        }

        // TODO do not do this here, this is a GraphQL-ism
        if server_scalar_selectable.name.item == "id" {
            set_and_validate_id_field(
                id_field,
                next_server_scalar_selectable_id,
                parent_type_name,
                options,
                inner_non_null_named_type,
            )?;
        }

        self.server_scalar_selectables
            .push(server_scalar_selectable);

        Ok(())
    }

    // TODO this function should not exist
    pub fn insert_server_object_selectable(
        &mut self,
        server_object_selectable: ServerObjectSelectable<TNetworkProtocol>,
    ) -> CreateAdditionalFieldsResult<()> {
        let next_server_object_selectable_id = self.server_object_selectables.len().into();
        let parent_object_entity_name = server_object_selectable.parent_object_name;
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
                    next_server_object_selectable_id,
                ))),
            )
            .is_some()
        {
            let parent_object = self
                .server_entity_data
                .server_object_entity(parent_object_entity_name);
            return Err(CreateAdditionalFieldsError::DuplicateField {
                field_name: next_object_name.item.into(),
                parent_type: parent_object.name,
            });
        }

        self.server_object_selectables
            .push(server_object_selectable);

        Ok(())
    }

    /// Get a reference to a given client field by its field name and parent name.
    ///
    pub fn client_field(
        &self,
        parent_type_name: SchemaServerObjectEntityName,
        client_field_name: ClientScalarSelectableName,
    ) -> &ClientScalarSelectable<TNetworkProtocol> {
        self.client_scalar_selectables
            .get(&(parent_type_name, client_field_name))
            .expect("Expected client field to exist")
    }

    // TODO this function should not exist
    pub fn client_field_mut(
        &mut self,
        parent_type_name: SchemaServerObjectEntityName,
        client_field_name: ClientScalarSelectableName,
    ) -> &mut ClientScalarSelectable<TNetworkProtocol> {
        self.client_scalar_selectables
            .get_mut(&(parent_type_name, client_field_name))
            .expect("Expected client field to exist")
    }

    pub fn object_selectable(
        &self,
        field_id: ObjectSelectableId,
    ) -> ObjectSelectable<TNetworkProtocol> {
        match field_id {
            DefinitionLocation::Server((_parent_object_entity_name, server_field_id)) => {
                DefinitionLocation::Server(self.server_object_selectable(server_field_id))
            }
            DefinitionLocation::Client((_parent_object_entity_name, client_pointer_id)) => {
                DefinitionLocation::Client(self.client_pointer(client_pointer_id))
            }
        }
    }

    pub fn client_pointer(
        &self,
        client_pointer_id: ClientObjectSelectableId,
    ) -> &ClientObjectSelectable<TNetworkProtocol> {
        &self.client_object_selectables[client_pointer_id.as_usize()]
    }

    // TODO this function should not exist
    pub fn client_pointer_mut(
        &mut self,
        client_pointer_id: ClientObjectSelectableId,
    ) -> &mut ClientObjectSelectable<TNetworkProtocol> {
        &mut self.client_object_selectables[client_pointer_id.as_usize()]
    }

    pub fn client_object_selectables_and_ids(
        &self,
    ) -> impl Iterator<Item = WithId<&ClientObjectSelectable<TNetworkProtocol>>> {
        self.client_object_selectables
            .iter()
            .enumerate()
            .map(|(id, client_object_selectable)| WithId::new(id.into(), client_object_selectable))
    }

    pub fn client_type(
        &self,
        client_type_id: ClientSelectableId,
    ) -> SelectionType<
        &ClientScalarSelectable<TNetworkProtocol>,
        &ClientObjectSelectable<TNetworkProtocol>,
    > {
        match client_type_id {
            SelectionType::Scalar((parent_type_name, client_field_id)) => {
                SelectionType::Scalar(self.client_field(parent_type_name, client_field_id))
            }
            SelectionType::Object((_parent_type_name, client_pointer_id)) => {
                SelectionType::Object(self.client_pointer(client_pointer_id))
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn user_written_client_types(
        &self,
    ) -> impl Iterator<
        Item = (
            SelectionType<
                (SchemaServerObjectEntityName, ClientScalarSelectableName),
                (SchemaServerObjectEntityName, ClientObjectSelectableId),
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
                    SelectionType::Scalar((field.parent_object_entity_name, field.name)),
                    SelectionType::Scalar(field),
                    info.client_field_directive_set,
                )),
                ClientFieldVariant::ImperativelyLoadedField(_) => None,
            })
            .chain(
                self.client_object_selectables
                    .iter()
                    .enumerate()
                    .map(|(id, pointer)| {
                        (
                            SelectionType::Object((pointer.parent_object_name, id.into())),
                            SelectionType::Object(pointer),
                            ClientFieldDirectiveSet::None(EmptyDirectiveSet {}),
                        )
                    }),
            )
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerEntityData<TNetworkProtocol> {
    pub fn server_scalar_entity(
        &self,
        scalar_entity_name: SchemaServerScalarEntityName,
    ) -> &ServerScalarEntity<TNetworkProtocol> {
        self.server_scalars
            .get(&scalar_entity_name)
            .expect("Expected scalar to exist")
    }

    pub fn server_entity(&self, type_id: ServerEntityName) -> ServerEntity<TNetworkProtocol> {
        match type_id {
            ServerEntityName::Object(object_entity_name) => {
                ServerEntity::Object(self.server_object_entity(object_entity_name))
            }
            ServerEntityName::Scalar(scalar_entity_name) => {
                ServerEntity::Scalar(self.server_scalar_entity(scalar_entity_name))
            }
        }
    }

    pub fn server_object_entity(
        &self,
        object_entity_name: SchemaServerObjectEntityName,
    ) -> &ServerObjectEntity<TNetworkProtocol> {
        self.server_objects
            .get(&object_entity_name)
            .expect("Expected object to exist")
    }

    // TODO this function should not exist
    pub fn server_object_entities_and_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = WithId<&mut ServerObjectEntity<TNetworkProtocol>>> + '_ {
        self.server_objects
            .iter_mut()
            .map(|(name, object)| WithId::new(*name, object))
    }

    // TODO this function should not exist
    pub fn insert_server_scalar_entity(
        &mut self,
        server_scalar_entity: ServerScalarEntity<TNetworkProtocol>,
        name_location: Location,
    ) -> Result<(), WithLocation<CreateAdditionalFieldsError>> {
        if self
            .defined_entities
            .insert(
                server_scalar_entity.name.item.into(),
                SelectionType::Scalar(server_scalar_entity.name.item),
            )
            .is_some()
        {
            return Err(WithLocation::new(
                CreateAdditionalFieldsError::DuplicateTypeDefinition {
                    type_definition_type: "scalar",
                    type_name: server_scalar_entity.name.item.into(),
                },
                name_location,
            ));
        }
        self.server_scalars
            .insert(server_scalar_entity.name.item, server_scalar_entity);
        Ok(())
    }

    // TODO this function should not exist
    pub fn insert_server_object_entity(
        &mut self,
        server_object_entity: ServerObjectEntity<TNetworkProtocol>,
        name_location: Location,
    ) -> Result<SchemaServerObjectEntityName, WithLocation<CreateAdditionalFieldsError>> {
        let name = server_object_entity.name;
        if self
            .defined_entities
            .insert(
                server_object_entity.name.into(),
                SelectionType::Object(server_object_entity.name),
            )
            .is_some()
        {
            return Err(WithLocation::new(
                CreateAdditionalFieldsError::DuplicateTypeDefinition {
                    type_definition_type: "object",
                    type_name: server_object_entity.name.into(),
                },
                name_location,
            ));
        }

        self.server_objects
            .insert(server_object_entity.name, server_object_entity);
        Ok(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathToRefetchField {
    pub linked_fields: Vec<NormalizationKey>,
    pub field_name: ClientScalarSelectableName,
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

fn add_schema_defined_scalar_type<TNetworkProtocol: NetworkProtocol>(
    scalars: &mut HashMap<SchemaServerScalarEntityName, ServerScalarEntity<TNetworkProtocol>>,
    defined_types: &mut HashMap<UnvalidatedTypeName, ServerEntityName>,
    field_name: &'static str,
    javascript_name: JavascriptName,
) -> SchemaServerScalarEntityName {
    // TODO this is problematic, we have no span (or really, no location) associated with this
    // schema-defined scalar, so we will not be able to properly show error messages if users
    // e.g. have Foo implements String
    let typename = WithLocation::new(field_name.intern().into(), Location::generated());
    scalars.insert(
        typename.item,
        ServerScalarEntity {
            description: None,
            name: typename,
            javascript_name,
            output_format: std::marker::PhantomData,
        },
    );
    defined_types.insert(
        typename.item.into(),
        ServerEntityName::Scalar(typename.item),
    );
    typename.item
}

#[derive(Debug, Clone)]
// This struct is indicative of poor data modeling.
pub enum SchemaServerObjectSelectableVariant {
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
    (SchemaServerObjectEntityName, ServerScalarSelectableId),
    (SchemaServerObjectEntityName, ClientScalarSelectableName),
>;

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerStrongIdFieldId>,
    current_field_id: ServerScalarSelectableId,
    parent_type_name: SchemaServerObjectEntityName,
    options: &CompilerConfigOptions,
    inner_non_null_named_type: Option<&GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
) -> CreateAdditionalFieldsResult<()> {
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_id.unchecked_conversion());

    match inner_non_null_named_type {
        Some(type_) => {
            if type_.0.item != *ID_GRAPHQL_TYPE {
                options.on_invalid_id_type.on_failure(|| {
                    CreateAdditionalFieldsError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_type: parent_type_name,
                    }
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                CreateAdditionalFieldsError::IdFieldMustBeNonNullIdType {
                    strong_field_name: "id",
                    parent_type: parent_type_name,
                }
            })?;
            Ok(())
        }
    }
}
