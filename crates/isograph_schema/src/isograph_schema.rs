use std::{collections::HashMap, fmt::Debug};

use common_lang_types::{
    ConstExportName, DescriptionValue, FieldArgumentName, HasName, InputTypeName,
    InterfaceTypeName, IsographObjectTypeName, JavascriptName, ResolverDefinitionPath,
    ScalarTypeName, SelectableFieldName, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    ConstantValue, GraphQLDirective, GraphQLFieldDefinition, GraphQLInputObjectTypeDefinition,
    GraphQLInputValueDefinition, GraphQLInterfaceTypeDefinition, GraphQLObjectTypeDefinition,
    NamedTypeAnnotation, TypeAnnotation,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinedTypeId, LinkedFieldSelection, NonConstantValue, ObjectId, ResolverFieldId, ScalarId,
    Selection, ServerFieldId, ServerIdFieldId, Unwrap, VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{FieldMapItem, ResolverVariant};

lazy_static! {
    pub static ref ID_GRAPHQL_TYPE: ScalarTypeName = "ID".intern().into();
    // TODO these don't belong here, and neither does the relative path stuff
    // TODO these shouldn't be SelectableFieldName's
    pub static ref READER: SelectableFieldName = "reader".intern().into();
    pub static ref ENTRYPOINT: SelectableFieldName = "entrypoint".intern().into();
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

    /// The associated data type of scalars in resolvers' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ValidatedDefinedField
    ///   i.e. DefinedField<ServerFieldId, ResolverFieldId>
    type ResolverSelectionScalarFieldAssociatedData: Debug;

    /// The associated data type of linked fields in resolvers' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ObjectId
    type ResolverSelectionLinkedFieldAssociatedData: Debug;

    /// The associated data type of resolvers' variable definitions
    /// - Unvalidated: UnvalidatedTypeName
    /// - Validated: DefinedTypeId
    type ResolverVariableDefinitionAssociatedData: Debug;

    /// On objects, what does the HashMap of encountered types contain
    /// - Unvalidated: UnvalidatedObjectFieldInfo
    ///   i.e. DefinedField<TypeAnnotation<UnvalidatedTypeName>, ResolverFieldId>
    /// - Validated: ValidatedDefinedField
    ///   i.e. DefinedField<ServerFieldId, ResolverFieldId>
    type EncounteredField: Debug;

    /// What we store in entrypoints
    /// - Unvalidated: (TextSource, WithSpan<ResolverFetch>)
    /// - Validated: (ObjectId, ResolverFieldId)
    type Entrypoint: Debug;
}

/// The in-memory representation of a schema.
///
/// The type param with which the Schema type is instantiated vary based on
/// how far along in the validation pipeline the schema is.
///
/// Invariant: a schema is append-only, because pointers into the Schema are in the
/// form of newtype wrappers around u32 indexes (e.g. FieldId, etc.) As a result,
/// the schema does not support removing items.
#[derive(Debug)]
pub struct Schema<TValidation: SchemaValidationState> {
    pub fields: Vec<SchemaServerField<TypeAnnotation<TValidation::FieldTypeAssociatedData>>>,
    pub resolvers: Vec<
        SchemaResolver<
            TValidation::ResolverSelectionScalarFieldAssociatedData,
            TValidation::ResolverSelectionLinkedFieldAssociatedData,
            TValidation::ResolverVariableDefinitionAssociatedData,
        >,
    >,
    // TODO consider whether this belongs here. It could just be a free variable.
    pub entrypoints: Vec<TValidation::Entrypoint>,
    pub schema_data: SchemaData<TValidation::EncounteredField>,

    // Well known types
    pub id_type_id: ScalarId,
    pub string_type_id: ScalarId,
    pub float_type_id: ScalarId,
    pub boolean_type_id: ScalarId,
    pub int_type_id: ScalarId,

    // typename
    // TODO name this root query type?
    pub query_type_id: Option<ObjectId>,
    // Subscription
    // Mutation
}

/// Distinguishes between server fields and locally-defined resolver fields.
/// TFieldAssociatedData can be a ScalarFieldName in an unvalidated schema, or a
/// ScalarId, in a validated schema.
///
/// TResolverType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// DefinedTypeId in a validated schema.
#[derive(Debug, Clone, Copy)]
pub enum DefinedField<TFieldAssociatedData, TResolverType> {
    ServerField(TFieldAssociatedData),
    ResolverField(TResolverType),
}

impl<TFieldAssociatedData, TResolverType> DefinedField<TFieldAssociatedData, TResolverType> {
    pub fn as_server_field(&self) -> Option<&TFieldAssociatedData> {
        match self {
            DefinedField::ServerField(server_field) => Some(server_field),
            DefinedField::ResolverField(_) => None,
        }
    }

    pub fn as_resolver_field(&self) -> Option<&TResolverType> {
        match self {
            DefinedField::ServerField(_) => None,
            DefinedField::ResolverField(resolver_field) => Some(resolver_field),
        }
    }
}

#[derive(Debug)]
pub struct SchemaData<TEncounteredField> {
    pub objects: Vec<SchemaObject<TEncounteredField>>,
    pub scalars: Vec<SchemaScalar>,
    pub defined_types: HashMap<UnvalidatedTypeName, DefinedTypeId>,
}

impl<TValidation: SchemaValidationState> Schema<TValidation> {
    /// Get a reference to a given field by its id.
    pub fn field(
        &self,
        field_id: ServerFieldId,
    ) -> &SchemaServerField<TypeAnnotation<TValidation::FieldTypeAssociatedData>> {
        &self.fields[field_id.as_usize()]
    }

    /// Get a reference to a given resolver by its id.
    pub fn resolver(
        &self,
        resolver_field_id: ResolverFieldId,
    ) -> &SchemaResolver<
        TValidation::ResolverSelectionScalarFieldAssociatedData,
        TValidation::ResolverSelectionLinkedFieldAssociatedData,
        TValidation::ResolverVariableDefinitionAssociatedData,
    > {
        &self.resolvers[resolver_field_id.as_usize()]
    }

    /// Get a reference to the root query_object, if it's defined.
    pub fn query_object(&self) -> Option<&SchemaObject<TValidation::EncounteredField>> {
        self.query_type_id
            .as_ref()
            .map(|id| self.schema_data.object(*id))
    }
}

impl<
        TFieldAssociatedData: Clone,
        TValidation: SchemaValidationState<FieldTypeAssociatedData = TFieldAssociatedData>,
    > Schema<TValidation>
{
    /// Get a reference to a given id field by its id.
    pub fn id_field<TIdFieldAssociatedData: TryFrom<TFieldAssociatedData> + Copy>(
        &self,
        id_field_id: ServerIdFieldId,
    ) -> SchemaIdField<NamedTypeAnnotation<TIdFieldAssociatedData>> {
        let field_id = id_field_id.into();

        let field = self
            .field(field_id)
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

impl<TEncounteredField> SchemaData<TEncounteredField> {
    /// Get a reference to a given scalar type by its id.
    pub fn scalar(&self, scalar_id: ScalarId) -> &SchemaScalar {
        &self.scalars[scalar_id.as_usize()]
    }

    pub fn lookup_unvalidated_type(&self, type_id: DefinedTypeId) -> SchemaType<TEncounteredField> {
        match type_id {
            DefinedTypeId::Object(id) => {
                SchemaType::Object(self.objects.get(id.as_usize()).unwrap())
            }
            DefinedTypeId::Scalar(id) => {
                SchemaType::Scalar(self.scalars.get(id.as_usize()).unwrap())
            }
        }
    }

    /// Get a reference to a given object type by its id.
    pub fn object(&self, object_id: ObjectId) -> &SchemaObject<TEncounteredField> {
        &self.objects[object_id.as_usize()]
    }

    /// Get a mutable reference to a given object type by its id.
    pub fn object_mut(&mut self, object_id: ObjectId) -> &mut SchemaObject<TEncounteredField> {
        &mut self.objects[object_id.as_usize()]
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
pub enum SchemaOutputType<'a, TValidation: SchemaValidationState> {
    Object(&'a SchemaObject<TValidation>),
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
    pub interfaces: Vec<WithLocation<InterfaceTypeName>>,
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
    pub id: ObjectId,
    // We probably don't want this
    pub directives: Vec<GraphQLDirective<ConstantValue>>,
    /// TODO remove id_field from fields, and change the type of Option<ServerFieldId>
    /// to something else.
    pub id_field: Option<ServerIdFieldId>,
    pub server_fields: Vec<ServerFieldId>,
    pub resolvers: Vec<ResolverFieldId>,
    pub encountered_fields: HashMap<SelectableFieldName, TEncounteredField>,
}

/// In GraphQL, ValidRefinement's are essentially the concrete types that an interface or
/// union can be narrowed to. valid_refinements should be empty for concrete types.
#[derive(Debug)]
pub struct ValidRefinement {
    pub target: ObjectId,
    // pub is_guaranteed_to_work: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaServerField<TData> {
    pub description: Option<DescriptionValue>,
    /// The name of the server field and the location where it was defined
    /// (i.e. for resolvers, an iso literal, and for server fields, the schema
    /// file, and for other fields, Location::Generated).
    pub name: WithLocation<SelectableFieldName>,
    pub id: ServerFieldId,
    pub associated_data: TData,
    pub parent_type_id: ObjectId,
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
    pub id: ServerIdFieldId,
    pub associated_data: TData,
    pub parent_type_id: ObjectId,
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
pub struct ResolverTypeAndField {
    pub type_name: IsographObjectTypeName,
    pub field_name: SelectableFieldName,
}

impl ResolverTypeAndField {
    pub fn underscore_separated(&self) -> String {
        format!("{}__{}", self.type_name, self.field_name)
    }

    pub fn relative_path(&self, current_file_type_name: IsographObjectTypeName) -> String {
        let ResolverTypeAndField {
            type_name,
            field_name,
        } = *self;
        if type_name != current_file_type_name {
            format!("../../{type_name}/{field_name}/{}", *READER)
        } else {
            format!("../{field_name}/{}", *READER)
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResolverActionKind {
    /// Associated js function
    NamedImport((ConstExportName, ResolverDefinitionPath)),
    /// Refetch fields
    RefetchField,
    /// Mutation field
    MutationField(MutationFieldResolverActionKindInfo),
}

#[derive(Debug, Clone)]
pub struct MutationFieldResolverActionKindInfo {
    pub field_map: Vec<FieldMapItem>,
}

#[derive(Debug)]
pub struct SchemaResolver<
    TResolverSelectionScalarFieldAssociatedData,
    TResolverSelectionLinkedFieldAssociatedData,
    TResolverVariableDefinitionAssociatedData,
> {
    pub description: Option<DescriptionValue>,
    // TODO make this a ResolverName that can be converted into a SelectableFieldName
    pub name: SelectableFieldName,
    pub id: ResolverFieldId,
    // TODO it makes no sense for a resolver to not select fields!
    // Why not just make it a global function at that point? Who knows.
    // Unless you'll eventually select fields?
    // Perhaps refetch fields for viewer (or other fields that have a known path
    // that don't require id) will have no selection set.
    pub selection_set_and_unwraps: Option<(
        Vec<
            WithSpan<
                Selection<
                    TResolverSelectionScalarFieldAssociatedData,
                    TResolverSelectionLinkedFieldAssociatedData,
                >,
            >,
        >,
        Vec<WithSpan<Unwrap>>,
    )>,

    // TODO we should probably model this differently
    pub variant: ResolverVariant,

    pub action_kind: ResolverActionKind,

    pub variable_definitions:
        Vec<WithSpan<VariableDefinition<TResolverVariableDefinitionAssociatedData>>>,

    // TODO this is probably unused
    // Why is this not calculated when needed?
    pub type_and_field: ResolverTypeAndField,

    // TODO should this be TypeWithFieldsId???
    pub parent_object_id: ObjectId,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathToRefetchField {
    pub linked_fields: Vec<NameAndArguments>,
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
    pub name: WithLocation<ScalarTypeName>,
    pub id: ScalarId,
    pub javascript_name: JavascriptName,
}
