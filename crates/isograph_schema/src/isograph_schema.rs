use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

use common_lang_types::{
    ClientScalarSelectableName, GraphQLScalarTypeName, SelectableName, UnvalidatedTypeName,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldId, ClientPointerId, DefinitionLocation, SelectionType,
    ServerEntityId, ServerObjectId, ServerScalarId, ServerScalarSelectableId,
};
use lazy_static::lazy_static;

use crate::{
    schema_validation_state::SchemaValidationState, ClientField, ClientFieldOrPointerId,
    ClientPointer, NormalizationKey, OutputFormat, SchemaObject, SchemaScalar, SchemaType,
    ServerScalarSelectable,
};

lazy_static! {
    pub static ref ID_GRAPHQL_TYPE: GraphQLScalarTypeName = "ID".intern().into();
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
/// The TOutputFormat type param will stay constant as the schema is validated.
///
/// Invariant: a schema is append-only, because pointers into the Schema are in the
/// form of newtype wrappers around u32 indexes (e.g. FieldId, etc.) As a result,
/// the schema does not support removing items.
#[derive(Debug)]
pub struct Schema<TSchemaValidationState: SchemaValidationState, TOutputFormat: OutputFormat> {
    pub server_scalar_selectables: Vec<ServerScalarSelectable<TOutputFormat>>,
    pub client_types: SelectionTypes<TOutputFormat>,
    // TODO consider whether this belongs here. It could just be a free variable.
    pub entrypoints: TSchemaValidationState::Entrypoint,
    pub server_field_data: ServerFieldData<TOutputFormat>,

    /// These are root types like Query, Mutation, Subscription
    pub fetchable_types: BTreeMap<ServerObjectId, RootOperationName>,
}

type SelectionTypes<TOutputFormat> =
    Vec<SelectionType<ClientField<TOutputFormat>, ClientPointer<TOutputFormat>>>;

impl<TSchemaValidationState: SchemaValidationState, TOutputFormat: OutputFormat>
    Schema<TSchemaValidationState, TOutputFormat>
{
    /// This is a smell, and we should refactor away from it, or all schema's
    /// should have a root type.
    pub fn query_id(&self) -> ServerObjectId {
        *self
            .fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
            .expect("Expected query to be found")
            .0
    }

    pub fn find_mutation(&self) -> Option<(&ServerObjectId, &RootOperationName)> {
        self.fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "mutation")
    }

    pub fn find_query(&self) -> Option<(&ServerObjectId, &RootOperationName)> {
        self.fetchable_types
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
    }
}

pub type LinkedType<'a, TOutputFormat> =
    DefinitionLocation<&'a ServerScalarSelectable<TOutputFormat>, &'a ClientPointer<TOutputFormat>>;

#[derive(Debug)]
pub struct ServerFieldData<TOutputFormat: OutputFormat> {
    pub server_objects: Vec<SchemaObject<TOutputFormat>>,
    pub server_scalars: Vec<SchemaScalar<TOutputFormat>>,
    pub defined_types: HashMap<UnvalidatedTypeName, ServerEntityId>,

    // Well known types
    pub id_type_id: ServerScalarId,
    pub string_type_id: ServerScalarId,
    pub float_type_id: ServerScalarId,
    pub boolean_type_id: ServerScalarId,
    pub int_type_id: ServerScalarId,
    // TODO restructure UnionTypeAnnotation to not have a nullable field, but to instead
    // include null in its variants.
    pub null_type_id: ServerScalarId,
}

impl<TSchemaValidationState: SchemaValidationState, TOutputFormat: OutputFormat>
    Schema<TSchemaValidationState, TOutputFormat>
{
    /// Get a reference to a given server field by its id.
    pub fn server_field(
        &self,
        server_field_id: ServerScalarSelectableId,
    ) -> &ServerScalarSelectable<TOutputFormat> {
        &self.server_scalar_selectables[server_field_id.as_usize()]
    }

    /// Get a reference to a given client field by its id.
    pub fn client_field(&self, client_field_id: ClientFieldId) -> &ClientField<TOutputFormat> {
        match &self.client_types[client_field_id.as_usize()] {
            SelectionType::Scalar(client_field) => client_field,
            SelectionType::Object(_) => panic!(
                "encountered ClientPointer under ClientFieldId. \
                This is indicative of a bug in Isograph."
            ),
        }
    }

    pub fn client_field_mut(
        &mut self,
        client_field_id: ClientFieldId,
    ) -> &mut ClientField<TOutputFormat> {
        match &mut self.client_types[client_field_id.as_usize()] {
            SelectionType::Scalar(client_field) => client_field,
            SelectionType::Object(_) => panic!(
                "encountered ClientPointer under ClientFieldId. \
                This is indicative of a bug in Isograph."
            ),
        }
    }

    pub fn linked_type(
        &self,
        field_id: DefinitionLocation<ServerScalarSelectableId, ClientPointerId>,
    ) -> LinkedType<TOutputFormat> {
        match field_id {
            DefinitionLocation::Server(server_field_id) => {
                DefinitionLocation::Server(self.server_field(server_field_id))
            }
            DefinitionLocation::Client(client_pointer_id) => {
                DefinitionLocation::Client(self.client_pointer(client_pointer_id))
            }
        }
    }

    pub fn client_pointer(
        &self,
        client_pointer_id: ClientPointerId,
    ) -> &ClientPointer<TOutputFormat> {
        match &self.client_types[client_pointer_id.as_usize()] {
            SelectionType::Object(client_pointer) => client_pointer,
            SelectionType::Scalar(_) => panic!(
                "encountered ClientField under ClientPointerId. \
                This is indicative of a bug in Isograph."
            ),
        }
    }

    pub fn client_pointer_mut(
        &mut self,
        client_pointer_id: ClientPointerId,
    ) -> &mut ClientPointer<TOutputFormat> {
        match &mut self.client_types[client_pointer_id.as_usize()] {
            SelectionType::Object(client_pointer) => client_pointer,
            SelectionType::Scalar(_) => panic!(
                "encountered ClientField under ClientPointerId. \
                This is indicative of a bug in Isograph."
            ),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn client_type(
        &self,
        client_type_id: ClientFieldOrPointerId,
    ) -> SelectionType<&ClientField<TOutputFormat>, &ClientPointer<TOutputFormat>> {
        match client_type_id {
            SelectionType::Scalar(client_field_id) => {
                SelectionType::Scalar(self.client_field(client_field_id))
            }
            SelectionType::Object(client_pointer_id) => {
                SelectionType::Object(self.client_pointer(client_pointer_id))
            }
        }
    }
}

impl<TOutputFormat: OutputFormat> ServerFieldData<TOutputFormat> {
    /// Get a reference to a given scalar type by its id.
    pub fn scalar(&self, scalar_id: ServerScalarId) -> &SchemaScalar<TOutputFormat> {
        &self.server_scalars[scalar_id.as_usize()]
    }

    // TODO this function is horribly named
    pub fn lookup_unvalidated_type(&self, type_id: ServerEntityId) -> SchemaType<TOutputFormat> {
        match type_id {
            ServerEntityId::Object(object_id) => SchemaType::Object(self.object(object_id)),
            ServerEntityId::Scalar(scalar_id) => SchemaType::Scalar(self.scalar(scalar_id)),
        }
    }

    /// Get a reference to a given object type by its id.
    pub fn object(&self, object_id: ServerObjectId) -> &SchemaObject<TOutputFormat> {
        &self.server_objects[object_id.as_usize()]
    }

    /// Get a mutable reference to a given object type by its id.
    pub fn object_mut(&mut self, object_id: ServerObjectId) -> &mut SchemaObject<TOutputFormat> {
        &mut self.server_objects[object_id.as_usize()]
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
