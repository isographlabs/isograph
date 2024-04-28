use std::{collections::HashMap, fmt::Debug};

use common_lang_types::{
    ArtifactFileType, DescriptionValue, FieldArgumentName, GraphQLInterfaceTypeName,
    GraphQLScalarTypeName, HasName, InputTypeName, IsographObjectTypeName, JavascriptName,
    SelectableFieldName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    ConstantValue, GraphQLDirective, GraphQLFieldDefinition, GraphQLInputObjectTypeDefinition,
    GraphQLInputValueDefinition, GraphQLInterfaceTypeDefinition, GraphQLObjectTypeDefinition,
    GraphQLTypeAnnotation, NamedTypeAnnotation,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ClientFieldId, LinkedFieldSelection, NonConstantValue, SelectableServerFieldId, Selection,
    ServerFieldId, ServerObjectId, ServerScalarId, ServerStrongIdFieldId, Unwrap,
    VariableDefinition,
};
use lazy_static::lazy_static;

use crate::ClientFieldVariant;

lazy_static! {
    pub static ref ID_GRAPHQL_TYPE: GraphQLScalarTypeName = "ID".intern().into();
}

/// A trait that encapsulates all the types over which a schema, fields, etc.
/// are generic. As we go from parsed -> various states of validated -> fully
/// validated, we will get objects that are generic over a different type
/// that implements SchemaValidationState.
pub trait SchemaValidationState: Debug {
    /// A SchemaServerField contains a associated_data: TypeAnnotation<FieldTypeAssociatedData>
    /// - Unvalidated: UnvalidatedTypeName
    /// - Validated: DefinedTypeId
    type FieldTypeAssociatedData: Debug;

    /// The associated data type of scalars in client fields' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ValidatedFieldDefinitionLocation
    ///   i.e. DefinedField<ServerFieldId, ClientFieldId>
    type ClientFieldSelectionScalarFieldAssociatedData: Debug;

    /// The associated data type of linked fields in client fields' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ObjectId
    type ClientFieldSelectionLinkedFieldAssociatedData: Debug;

    /// The associated data type of client fields' variable definitions
    /// - Unvalidated: UnvalidatedTypeName
    /// - Validated: FieldDefinition
    type ClientFieldVariableDefinitionAssociatedData: Debug;

    /// On objects, what does the HashMap of encountered types contain
    /// - Unvalidated: UnvalidatedObjectFieldInfo
    ///   i.e. DefinedField<TypeAnnotation<UnvalidatedTypeName>, ClientFieldId>
    /// - Validated: ValidatedFieldDefinitionLocation
    ///   i.e. DefinedField<ServerFieldId, ClientFieldId>
    type EncounteredField: Debug;

    /// What we store in entrypoints
    /// - Unvalidated: (TextSource, WithSpan<ObjectTypeAndField>)
    /// - Validated: (ObjectId, ClientFieldId)
    type Entrypoint: Debug;
}

#[derive(Debug)]
pub struct RootOperationName(pub String);

/// The in-memory representation of a schema.
///
/// The type param with which the Schema type is instantiated vary based on
/// how far along in the validation pipeline the schema is.
///
/// Invariant: a schema is append-only, because pointers into the Schema are in the
/// form of newtype wrappers around u32 indexes (e.g. FieldId, etc.) As a result,
/// the schema does not support removing items.
#[derive(Debug)]
pub struct Schema<TSchemaValidationState: SchemaValidationState> {
    pub server_fields: Vec<
        SchemaServerField<GraphQLTypeAnnotation<TSchemaValidationState::FieldTypeAssociatedData>>,
    >,
    pub client_fields: Vec<
        ClientField<
            TSchemaValidationState::ClientFieldSelectionScalarFieldAssociatedData,
            TSchemaValidationState::ClientFieldSelectionLinkedFieldAssociatedData,
            TSchemaValidationState::ClientFieldVariableDefinitionAssociatedData,
        >,
    >,
    // TODO consider whether this belongs here. It could just be a free variable.
    pub entrypoints: Vec<TSchemaValidationState::Entrypoint>,
    pub server_field_data: ServerFieldData<TSchemaValidationState::EncounteredField>,

    // Well known types
    pub id_type_id: ServerScalarId,
    pub string_type_id: ServerScalarId,
    pub float_type_id: ServerScalarId,
    pub boolean_type_id: ServerScalarId,
    pub int_type_id: ServerScalarId,

    pub root_types: HashMap<ServerObjectId, RootOperationName>,
}

/// Distinguishes between server-defined fields and locally-defined fields.
/// TFieldAssociatedData can be a ScalarFieldName in an unvalidated schema, or a
/// ScalarId, in a validated schema.
///
/// TLocalType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// DefinedTypeId in a validated schema.
///
/// Note that locally-defined fields do **not** only include fields defined in
/// an iso field literal. Refetch fields and generated mutation fields are
/// also local fields.
#[derive(Debug, Clone, Copy)]
pub enum FieldDefinitionLocation<TServer, TClient> {
    Server(TServer),
    Client(TClient),
}

impl<TFieldAssociatedData, TClientFieldType>
    FieldDefinitionLocation<TFieldAssociatedData, TClientFieldType>
{
    pub fn as_server_field(&self) -> Option<&TFieldAssociatedData> {
        match self {
            FieldDefinitionLocation::Server(server_field) => Some(server_field),
            FieldDefinitionLocation::Client(_) => None,
        }
    }

    pub fn as_client_field(&self) -> Option<&TClientFieldType> {
        match self {
            FieldDefinitionLocation::Server(_) => None,
            FieldDefinitionLocation::Client(client_field) => Some(client_field),
        }
    }
}

#[derive(Debug)]
pub struct ServerFieldData<TEncounteredField> {
    pub server_objects: Vec<SchemaObject<TEncounteredField>>,
    pub server_scalars: Vec<SchemaScalar>,
    pub defined_types: HashMap<UnvalidatedTypeName, SelectableServerFieldId>,
}

impl<TSchemaValidationState: SchemaValidationState> Schema<TSchemaValidationState> {
    /// Get a reference to a given server field by its id.
    pub fn server_field(
        &self,
        server_field_id: ServerFieldId,
    ) -> &SchemaServerField<GraphQLTypeAnnotation<TSchemaValidationState::FieldTypeAssociatedData>>
    {
        &self.server_fields[server_field_id.as_usize()]
    }

    /// Get a reference to a given client field by its id.
    pub fn client_field(
        &self,
        client_field_id: ClientFieldId,
    ) -> &ClientField<
        TSchemaValidationState::ClientFieldSelectionScalarFieldAssociatedData,
        TSchemaValidationState::ClientFieldSelectionLinkedFieldAssociatedData,
        TSchemaValidationState::ClientFieldVariableDefinitionAssociatedData,
    > {
        &self.client_fields[client_field_id.as_usize()]
    }
}

impl<
        TFieldAssociatedData: Clone,
        TSchemaValidationState: SchemaValidationState<FieldTypeAssociatedData = TFieldAssociatedData>,
    > Schema<TSchemaValidationState>
{
    /// Get a reference to a given id field by its id.
    pub fn id_field<TIdFieldAssociatedData: TryFrom<TFieldAssociatedData> + Copy>(
        &self,
        id_field_id: ServerStrongIdFieldId,
    ) -> SchemaIdField<NamedTypeAnnotation<TIdFieldAssociatedData>> {
        let field_id = id_field_id.into();

        let field = self
            .server_field(field_id)
            .and_then(|e| match e.inner_non_null_named_type() {
                Some(inner) => Ok(NamedTypeAnnotation(inner.0.clone().map(|x| {
                    let y: Result<TIdFieldAssociatedData, _> = x.try_into();
                    match y {
                        Ok(y) => y,
                        Err(_) => {
                            panic!("Conversion failed. This indicates a bug in Isograph")
                        }
                    }
                }))),
                None => Err(()),
            })
            .expect(
                "We had an id field, the type annotation should be named. \
                    This indicates a bug in Isograph.",
            );

        field.try_into().expect(
            "We had an id field, no arguments should exist. This indicates a bug in Isograph.",
        )
    }
}

impl<TEncounteredField> ServerFieldData<TEncounteredField> {
    /// Get a reference to a given scalar type by its id.
    pub fn scalar(&self, scalar_id: ServerScalarId) -> &SchemaScalar {
        &self.server_scalars[scalar_id.as_usize()]
    }

    pub fn lookup_unvalidated_type(
        &self,
        type_id: SelectableServerFieldId,
    ) -> SchemaType<TEncounteredField> {
        match type_id {
            SelectableServerFieldId::Object(id) => {
                SchemaType::Object(self.server_objects.get(id.as_usize()).unwrap())
            }
            SelectableServerFieldId::Scalar(id) => {
                SchemaType::Scalar(self.server_scalars.get(id.as_usize()).unwrap())
            }
        }
    }

    /// Get a reference to a given object type by its id.
    pub fn object(&self, object_id: ServerObjectId) -> &SchemaObject<TEncounteredField> {
        &self.server_objects[object_id.as_usize()]
    }

    /// Get a mutable reference to a given object type by its id.
    pub fn object_mut(
        &mut self,
        object_id: ServerObjectId,
    ) -> &mut SchemaObject<TEncounteredField> {
        &mut self.server_objects[object_id.as_usize()]
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaType<'a, TEncounteredField> {
    Object(&'a SchemaObject<TEncounteredField>),
    Scalar(&'a SchemaScalar),
}

impl<'a, T> HasName for SchemaType<'a, T> {
    type Name = UnvalidatedTypeName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaType::Object(object) => object.name.into(),
            SchemaType::Scalar(scalar) => scalar.name.item.into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaOutputType<'a, TSchemaValidationState: SchemaValidationState> {
    Object(&'a SchemaObject<TSchemaValidationState>),
    Scalar(&'a SchemaScalar),
    // excludes input object
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaInputType<'a> {
    Scalar(&'a SchemaScalar),
    // input object
    // enum
}

impl<'a> HasName for SchemaInputType<'a> {
    type Name = InputTypeName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaInputType::Scalar(x) => x.name.item.into(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct IsographObjectTypeDefinition {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<IsographObjectTypeName>,
    // maybe this should be Vec<WithSpan<IsographObjectTypeName>>>
    pub interfaces: Vec<WithLocation<GraphQLInterfaceTypeName>>,
    /// Directives that we don't know about. Maybe this should be validated to be
    /// empty, or not exist.
    pub directives: Vec<GraphQLDirective<ConstantValue>>,
    // TODO the spans of these fields are wrong
    // TODO use a shared field type
    pub fields: Vec<WithLocation<GraphQLFieldDefinition>>,
}

impl From<GraphQLObjectTypeDefinition> for IsographObjectTypeDefinition {
    fn from(object_type_definition: GraphQLObjectTypeDefinition) -> Self {
        IsographObjectTypeDefinition {
            description: object_type_definition.description,
            name: object_type_definition.name.map(|x| x.into()),
            interfaces: object_type_definition.interfaces,
            directives: object_type_definition.directives,
            fields: object_type_definition.fields,
        }
    }
}

impl From<GraphQLInterfaceTypeDefinition> for IsographObjectTypeDefinition {
    fn from(value: GraphQLInterfaceTypeDefinition) -> Self {
        Self {
            description: value.description,
            name: value.name.map(|x| x.into()),
            interfaces: value.interfaces,
            directives: value.directives,
            fields: value.fields,
        }
    }
}

// TODO this is bad. We should instead convert both GraphQL types to a common
// Isograph type
impl From<GraphQLInputObjectTypeDefinition> for IsographObjectTypeDefinition {
    fn from(value: GraphQLInputObjectTypeDefinition) -> Self {
        Self {
            description: value.description,
            name: value.name.map(|x| x.into()),
            interfaces: vec![],
            directives: value.directives,
            fields: value
                .fields
                .into_iter()
                .map(|with_location| with_location.map(From::from))
                .collect(),
        }
    }
}

/// An object type in the schema.
#[derive(Debug)]
pub struct SchemaObject<TEncounteredField> {
    pub description: Option<DescriptionValue>,
    pub name: IsographObjectTypeName,
    pub id: ServerObjectId,
    // We probably don't want this
    pub directives: Vec<GraphQLDirective<ConstantValue>>,
    /// TODO remove id_field from fields, and change the type of Option<ServerFieldId>
    /// to something else.
    pub id_field: Option<ServerStrongIdFieldId>,
    pub server_field_ids: Vec<ServerFieldId>,
    pub client_field_ids: Vec<ClientFieldId>,
    pub encountered_fields: HashMap<SelectableFieldName, TEncounteredField>,
}

/// In GraphQL, ValidRefinement's are essentially the concrete types that an interface or
/// union can be narrowed to. valid_refinements should be empty for concrete types.
#[derive(Debug)]
pub struct ValidRefinement {
    pub target: ServerObjectId,
    // pub is_guaranteed_to_work: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaServerField<TData> {
    pub description: Option<DescriptionValue>,
    /// The name of the server field and the location where it was defined
    /// (i.e. for client fields, an iso literal, and for server fields, the schema
    /// file, and for other fields, Location::Generated).
    pub name: WithLocation<SelectableFieldName>,
    pub id: ServerFieldId,
    pub associated_data: TData,
    pub parent_type_id: ServerObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
}

impl<TData> SchemaServerField<TData> {
    pub fn and_then<TData2, E>(
        &self,
        convert: impl FnOnce(&TData) -> Result<TData2, E>,
    ) -> Result<SchemaServerField<TData2>, E> {
        Ok(SchemaServerField {
            description: self.description,
            name: self.name,
            id: self.id,
            associated_data: convert(&self.associated_data)?,
            parent_type_id: self.parent_type_id,
            arguments: self.arguments.clone(),
        })
    }
}

// TODO make SchemaServerField generic over TData, TId and TArguments, instead of just TData.
// Then, SchemaIdField can be the same struct.
#[derive(Debug, Clone, Copy)]
pub struct SchemaIdField<TData> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<SelectableFieldName>,
    pub id: ServerStrongIdFieldId,
    pub associated_data: TData,
    pub parent_type_id: ServerObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
}

impl<TData: Copy> TryFrom<SchemaServerField<TData>> for SchemaIdField<TData> {
    type Error = ();

    fn try_from(value: SchemaServerField<TData>) -> Result<Self, Self::Error> {
        // If the field is valid as an id field, we succeed, otherwise, fail.
        // Initially, that will mean checking that there are no arguments.
        // This will result in a lot of false positives, and that can be improved
        // by requiring a specific directive or something.
        //
        // There are no arguments now, so this will always succeed.
        //
        // Also, before this is called, we have already converted the associated_data to be valid
        // (it should go from TypeAnnotation<T> to NamedTypeAnnotation<T>) via
        // inner_non_null_named_type. We should eventually add some NewType wrapper to
        // enforce that we didn't just call .inner()
        Ok(SchemaIdField {
            description: value.description,
            name: value.name,
            id: value.id.0.into(),
            associated_data: value.associated_data,
            parent_type_id: value.parent_type_id,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct ObjectTypeAndFieldNames {
    pub type_name: IsographObjectTypeName,
    pub field_name: SelectableFieldName,
}

impl ObjectTypeAndFieldNames {
    pub fn underscore_separated(&self) -> String {
        format!("{}__{}", self.type_name, self.field_name)
    }

    pub fn relative_path(
        &self,
        current_file_type_name: IsographObjectTypeName,
        file_type: ArtifactFileType,
    ) -> String {
        let ObjectTypeAndFieldNames {
            type_name,
            field_name,
        } = *self;
        if type_name != current_file_type_name {
            format!("../../{type_name}/{field_name}/{}", file_type)
        } else {
            format!("../{field_name}/{}", file_type)
        }
    }
}

#[derive(Debug)]
pub struct ClientField<
    TClientFieldSelectionScalarFieldAssociatedData,
    TClientFieldSelectionLinkedFieldAssociatedData,
    TClientFieldVariableDefinitionAssociatedData,
> {
    pub description: Option<DescriptionValue>,
    // TODO make this a ClientFieldName that can be converted into a SelectableFieldName
    pub name: SelectableFieldName,
    pub id: ClientFieldId,
    // TODO it makes no sense for a client field to not select fields!
    // Why not just make it a global function at that point? Who knows.
    // Unless you'll eventually select fields?
    // Perhaps refetch fields for viewer (or other fields that have a known path
    // that don't require id) will have no selection set.
    pub selection_set_and_unwraps: Option<(
        Vec<
            WithSpan<
                Selection<
                    TClientFieldSelectionScalarFieldAssociatedData,
                    TClientFieldSelectionLinkedFieldAssociatedData,
                >,
            >,
        >,
        Vec<WithSpan<Unwrap>>,
    )>,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub variable_definitions:
        Vec<WithSpan<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,

    // TODO this is probably unused
    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldNames,

    // TODO should this be TypeWithFieldsId???
    pub parent_object_id: ServerObjectId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathToRefetchField {
    pub linked_fields: Vec<NameAndArguments>,
    pub field_name: SelectableFieldName,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameAndArguments {
    pub name: SelectableFieldName,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

pub fn into_name_and_arguments<T, U>(field: &LinkedFieldSelection<T, U>) -> NameAndArguments {
    NameAndArguments {
        name: field.name.item.into(),
        arguments: field
            .arguments
            .iter()
            .map(|selection_field_argument| ArgumentKeyAndValue {
                key: selection_field_argument.item.name.item,
                // TODO do we need to clone here?
                value: selection_field_argument.item.value.item.clone(),
            })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArgumentKeyAndValue {
    pub key: FieldArgumentName,
    pub value: NonConstantValue,
}

impl<T> SchemaServerField<T> {
    // TODO probably unnecessary, and can be replaced with .map and .transpose
    pub fn split(self) -> (SchemaServerField<()>, T) {
        let Self {
            description,
            name,
            id,
            associated_data,
            parent_type_id,
            arguments,
        } = self;
        (
            SchemaServerField {
                description,
                name,
                id,
                associated_data: (),
                parent_type_id,
                arguments,
            },
            associated_data,
        )
    }
}

/// A scalar type in the schema.
#[derive(Debug)]
pub struct SchemaScalar {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLScalarTypeName>,
    pub id: ServerScalarId,
    pub javascript_name: JavascriptName,
}
