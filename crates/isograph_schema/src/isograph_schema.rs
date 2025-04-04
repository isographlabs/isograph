use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use common_lang_types::{
    ClientScalarSelectableName, GraphQLScalarTypeName, IsoLiteralText, IsographObjectTypeName,
    JavascriptName, Location, SelectableName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLNamedTypeAnnotation;
use intern::string_key::Intern;
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientObjectSelectableId, ClientScalarSelectableId, DefinitionLocation,
    ObjectSelection, ObjectSelectionDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionType, SelectionTypeContainingSelections, ServerEntityId, ServerObjectEntityId,
    ServerObjectSelectableId, ServerScalarEntityId, ServerScalarSelectableId,
    ServerStrongIdFieldId, VariableDefinition, WithId,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarOrObjectSelectableId,
    ClientScalarSelectable, NetworkProtocol, NormalizationKey, ServerEntity, ServerObjectEntity,
    ServerObjectSelectable, ServerScalarEntity, ServerScalarSelectable, ServerSelectable,
    ServerSelectableId, UseRefetchFieldRefetchStrategy, UserWrittenComponentVariant,
};

lazy_static! {
    pub static ref ID_GRAPHQL_TYPE: GraphQLScalarTypeName = "ID".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug, Clone)]
pub struct RootOperationName(pub String);

/// The in-memory representation of a schema.
///
/// The TSchemaValidationState type param varies based on how far along in the
/// validation pipeline the schema instance is, i.e. validating the schema means
/// consuming an instance and creating a new instance with another
/// TSchemaValidationState.
///
/// The TNetworkProtocol type param will stay constant as the schema is validated.
///
/// Invariant: a schema is append-only, because pointers into the Schema are in the
/// form of newtype wrappers around u32 indexes (e.g. FieldId, etc.) As a result,
/// the schema does not support removing items.
#[derive(Debug)]
pub struct Schema<TNetworkProtocol: NetworkProtocol> {
    pub server_scalar_selectables: Vec<ServerScalarSelectable<TNetworkProtocol>>,
    pub server_object_selectables: Vec<ServerObjectSelectable<TNetworkProtocol>>,
    pub client_scalar_selectables: Vec<ClientScalarSelectable<TNetworkProtocol>>,
    pub client_object_selectables: Vec<ClientObjectSelectable<TNetworkProtocol>>,
    pub entrypoints: HashMap<ClientScalarSelectableId, IsoLiteralText>,
    pub server_entity_data: ServerEntityData<TNetworkProtocol>,

    /// These are root types like Query, Mutation, Subscription
    pub fetchable_types: BTreeMap<ServerObjectEntityId, RootOperationName>,
}

impl<TNetworkProtocol: NetworkProtocol> Default for Schema<TNetworkProtocol> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    pub fn new() -> Self {
        // TODO add __typename
        let mut scalars = vec![];
        let mut defined_types = HashMap::default();

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
            client_scalar_selectables: vec![],
            client_object_selectables: vec![],

            entrypoints: Default::default(),
            server_entity_data: ServerEntityData {
                server_objects: vec![],
                server_scalars: scalars,
                defined_entities: defined_types,

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
    pub fn query_id(&self) -> ServerObjectEntityId {
        *self
            .fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
            .expect("Expected query to be found")
            .0
    }

    pub fn find_mutation(&self) -> Option<(&ServerObjectEntityId, &RootOperationName)> {
        self.fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "mutation")
    }

    pub fn find_query(&self) -> Option<(&ServerObjectEntityId, &RootOperationName)> {
        self.fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
    }
}

pub type ObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerObjectSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

#[derive(Debug)]
pub struct ServerEntityData<TNetworkProtocol: NetworkProtocol> {
    pub server_objects: Vec<ServerObjectEntity<TNetworkProtocol>>,
    pub server_scalars: Vec<ServerScalarEntity<TNetworkProtocol>>,
    pub defined_entities: HashMap<UnvalidatedTypeName, ServerEntityId>,

    // Well known types
    pub id_type_id: ServerScalarEntityId,
    pub string_type_id: ServerScalarEntityId,
    pub float_type_id: ServerScalarEntityId,
    pub boolean_type_id: ServerScalarEntityId,
    pub int_type_id: ServerScalarEntityId,
    // TODO restructure UnionTypeAnnotation to not have a nullable field, but to instead
    // include null in its variants.
    pub null_type_id: ServerScalarEntityId,
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
            SelectionType::Scalar(server_scalar_selectable_id) => {
                SelectionType::Scalar(self.server_scalar_selectable(server_scalar_selectable_id))
            }
            SelectionType::Object(server_object_selectable_id) => {
                SelectionType::Object(self.server_object_selectable(server_object_selectable_id))
            }
        }
    }

    pub fn insert_server_scalar_selectable(
        &mut self,
        server_scalar_selectable: ServerScalarSelectable<TNetworkProtocol>,
        // TODO do not accept this
        options: &CompilerConfigOptions,
        inner_non_null_named_type: Option<&GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
    ) -> InsertFieldsResult<()> {
        let next_server_scalar_selectable_id = self.server_scalar_selectables.len().into();
        let parent_object_entity_id = server_scalar_selectable.parent_type_id;
        let next_scalar_name = server_scalar_selectable.name;

        let parent_object = self
            .server_entity_data
            .object_entity_mut(parent_object_entity_id);

        if parent_object
            .available_selectables
            .insert(
                next_scalar_name.item.into(),
                DefinitionLocation::Server(SelectionType::Scalar(next_server_scalar_selectable_id)),
            )
            .is_some()
        {
            return Err(InsertFieldsError::DuplicateField {
                field_name: server_scalar_selectable.name.item.into(),
                parent_type: parent_object.name,
            });
        }

        // TODO do not do this here, this is a GraphQL-ism
        if server_scalar_selectable.name.item == "id" {
            set_and_validate_id_field(
                &mut parent_object.id_field,
                next_server_scalar_selectable_id,
                parent_object.name,
                options,
                inner_non_null_named_type,
            )?;
        }

        self.server_scalar_selectables
            .push(server_scalar_selectable);

        Ok(())
    }

    pub fn insert_server_object_selectable(
        &mut self,
        server_object_selectable: ServerObjectSelectable<TNetworkProtocol>,
    ) -> InsertFieldsResult<()> {
        let next_server_object_selectable_id = self.server_object_selectables.len().into();
        let parent_object_entity_id = server_object_selectable.parent_type_id;
        let next_object_name = server_object_selectable.name;

        let parent_object = self
            .server_entity_data
            .object_entity_mut(parent_object_entity_id);
        if parent_object
            .available_selectables
            .insert(
                next_object_name.item.into(),
                DefinitionLocation::Server(SelectionType::Object(next_server_object_selectable_id)),
            )
            .is_some()
        {
            return Err(InsertFieldsError::DuplicateField {
                field_name: next_object_name.item.into(),
                parent_type: parent_object.name,
            });
        }

        self.server_object_selectables
            .push(server_object_selectable);

        Ok(())
    }

    /// Get a reference to a given client field by its id.
    pub fn client_field(
        &self,
        client_field_id: ClientScalarSelectableId,
    ) -> &ClientScalarSelectable<TNetworkProtocol> {
        &self.client_scalar_selectables[client_field_id.as_usize()]
    }

    pub fn client_field_mut(
        &mut self,
        client_field_id: ClientScalarSelectableId,
    ) -> &mut ClientScalarSelectable<TNetworkProtocol> {
        &mut self.client_scalar_selectables[client_field_id.as_usize()]
    }

    pub fn client_scalar_selectables_and_ids(
        &self,
    ) -> impl Iterator<Item = WithId<&ClientScalarSelectable<TNetworkProtocol>>> {
        self.client_scalar_selectables
            .iter()
            .enumerate()
            .map(|(id, client_scalar_selectable)| WithId::new(id.into(), client_scalar_selectable))
    }

    pub fn object_selectable(
        &self,
        field_id: DefinitionLocation<ServerObjectSelectableId, ClientObjectSelectableId>,
    ) -> ObjectSelectable<TNetworkProtocol> {
        match field_id {
            DefinitionLocation::Server(server_field_id) => {
                DefinitionLocation::Server(self.server_object_selectable(server_field_id))
            }
            DefinitionLocation::Client(client_pointer_id) => {
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
        client_type_id: ClientScalarOrObjectSelectableId,
    ) -> SelectionType<
        &ClientScalarSelectable<TNetworkProtocol>,
        &ClientObjectSelectable<TNetworkProtocol>,
    > {
        match client_type_id {
            SelectionType::Scalar(client_field_id) => {
                SelectionType::Scalar(self.client_field(client_field_id))
            }
            SelectionType::Object(client_pointer_id) => {
                SelectionType::Object(self.client_pointer(client_pointer_id))
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn user_written_client_types(
        &self,
    ) -> impl Iterator<
        Item = (
            SelectionType<ClientScalarSelectableId, ClientObjectSelectableId>,
            SelectionType<
                &ClientScalarSelectable<TNetworkProtocol>,
                &ClientObjectSelectable<TNetworkProtocol>,
            >,
            UserWrittenComponentVariant,
        ),
    > {
        self.client_scalar_selectables
            .iter()
            .enumerate()
            .flat_map(|(id, field)| match field.variant {
                ClientFieldVariant::Link => None,
                ClientFieldVariant::UserWritten(info) => Some((
                    SelectionType::Scalar(id.into()),
                    SelectionType::Scalar(field),
                    info.user_written_component_variant,
                )),
                ClientFieldVariant::ImperativelyLoadedField(_) => None,
            })
            .chain(
                self.client_object_selectables
                    .iter()
                    .enumerate()
                    .map(|(id, pointer)| {
                        (
                            SelectionType::Object(id.into()),
                            SelectionType::Object(pointer),
                            UserWrittenComponentVariant::Eager,
                        )
                    }),
            )
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerEntityData<TNetworkProtocol> {
    /// Get a reference to a given scalar type by its id.
    pub fn scalar_entity(
        &self,
        scalar_entity_id: ServerScalarEntityId,
    ) -> &ServerScalarEntity<TNetworkProtocol> {
        &self.server_scalars[scalar_entity_id.as_usize()]
    }

    pub fn server_scalar_entities_and_ids(
        &self,
    ) -> impl Iterator<Item = WithId<&ServerScalarEntity<TNetworkProtocol>>> + '_ {
        self.server_scalars
            .iter()
            .enumerate()
            .map(|(id, scalar)| WithId::new(id.into(), scalar))
    }

    // TODO this function is horribly named
    pub fn lookup_unvalidated_type(
        &self,
        type_id: ServerEntityId,
    ) -> ServerEntity<TNetworkProtocol> {
        match type_id {
            ServerEntityId::Object(object_entity_id) => {
                ServerEntity::Object(self.object_entity(object_entity_id))
            }
            ServerEntityId::Scalar(scalar_entity_id) => {
                ServerEntity::Scalar(self.scalar_entity(scalar_entity_id))
            }
        }
    }

    /// Get a reference to a given object type by its id.
    pub fn object_entity(
        &self,
        object_entity_id: ServerObjectEntityId,
    ) -> &ServerObjectEntity<TNetworkProtocol> {
        &self.server_objects[object_entity_id.as_usize()]
    }

    /// Get a mutable reference to a given object type by its id.
    pub fn object_entity_mut(
        &mut self,
        object_entity_id: ServerObjectEntityId,
    ) -> &mut ServerObjectEntity<TNetworkProtocol> {
        &mut self.server_objects[object_entity_id.as_usize()]
    }

    pub fn server_object_entities_and_ids(
        &self,
    ) -> impl Iterator<Item = WithId<&ServerObjectEntity<TNetworkProtocol>>> + '_ {
        self.server_objects
            .iter()
            .enumerate()
            .map(|(id, object)| WithId::new(id.into(), object))
    }

    pub fn server_object_entities_and_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = WithId<&mut ServerObjectEntity<TNetworkProtocol>>> + '_ {
        self.server_objects
            .iter_mut()
            .enumerate()
            .map(|(id, object)| WithId::new(id.into(), object))
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
    scalars: &mut Vec<ServerScalarEntity<TNetworkProtocol>>,
    defined_types: &mut HashMap<UnvalidatedTypeName, ServerEntityId>,
    field_name: &'static str,
    javascript_name: JavascriptName,
) -> ServerScalarEntityId {
    let scalar_entity_id = scalars.len().into();

    // TODO this is problematic, we have no span (or really, no location) associated with this
    // schema-defined scalar, so we will not be able to properly show error messages if users
    // e.g. have Foo implements String
    let typename = WithLocation::new(field_name.intern().into(), Location::generated());
    scalars.push(ServerScalarEntity {
        description: None,
        name: typename,
        javascript_name,
        output_format: std::marker::PhantomData,
    });
    defined_types.insert(
        typename.item.into(),
        ServerEntityId::Scalar(scalar_entity_id),
    );
    scalar_entity_id
}

#[derive(Debug, Clone)]
pub enum SchemaServerObjectSelectableVariant {
    LinkedField,
    InlineFragment(ServerFieldTypeAssociatedDataInlineFragment),
}

#[derive(Debug, Clone)]
pub struct ServerFieldTypeAssociatedDataInlineFragment {
    pub server_object_selectable_id: ServerObjectSelectableId,
    pub concrete_type: IsographObjectTypeName,
    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,
}

pub type ValidatedSelection = SelectionTypeContainingSelections<
    ValidatedScalarSelectionAssociatedData,
    ValidatedObjectSelectionAssociatedData,
>;

pub type ValidatedObjectSelection =
    ObjectSelection<ValidatedScalarSelectionAssociatedData, ValidatedObjectSelectionAssociatedData>;

pub type ValidatedScalarSelection = ScalarSelection<ValidatedScalarSelectionAssociatedData>;

pub type ValidatedVariableDefinition = VariableDefinition<ServerEntityId>;

pub type ValidatedUseRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    ValidatedScalarSelectionAssociatedData,
    ValidatedObjectSelectionAssociatedData,
>;

/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedFieldDefinitionLocation =
    DefinitionLocation<ServerScalarSelectableId, ClientScalarSelectableId>;

#[derive(Debug, Clone)]
pub struct ValidatedObjectSelectionAssociatedData {
    pub parent_object_entity_id: ServerObjectEntityId,
    pub field_id: DefinitionLocation<ServerObjectSelectableId, ClientObjectSelectableId>,
    pub selection_variant: ObjectSelectionDirectiveSet,
    /// Some if the (destination?) object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,
}

// TODO this should encode whether the scalar selection points to a
// client field or to a server scalar
#[derive(Debug, Clone)]
pub struct ValidatedScalarSelectionAssociatedData {
    pub location: ValidatedFieldDefinitionLocation,
    pub selection_variant: ScalarSelectionDirectiveSet,
}

pub type ClientSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ClientScalarSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerStrongIdFieldId>,
    current_field_id: ServerScalarSelectableId,
    parent_type_name: IsographObjectTypeName,
    options: &CompilerConfigOptions,
    inner_non_null_named_type: Option<&GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
) -> InsertFieldsResult<()> {
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
                    InsertFieldsError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_type: parent_type_name,
                    }
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                InsertFieldsError::IdFieldMustBeNonNullIdType {
                    strong_field_name: "id",
                    parent_type: parent_type_name,
                }
            })?;
            Ok(())
        }
    }
}

#[derive(Error, Eq, PartialEq, Debug)]
pub enum InsertFieldsError {
    #[error(
        "The {strong_field_name} field on \"{parent_type}\" must have type \"ID!\".\n\
        This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_type: IsographObjectTypeName,
        strong_field_name: &'static str,
    },

    // TODO include info about where the field was previously defined
    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: SelectableName,
        parent_type: IsographObjectTypeName,
    },
}

type InsertFieldsResult<T> = Result<T, InsertFieldsError>;
