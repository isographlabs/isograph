use std::collections::HashMap;

use common_lang_types::{
    DescriptionValue, FieldArgumentName, HasName, InputTypeName, InterfaceTypeName,
    IsographObjectTypeName, JavascriptName, LinkedFieldName, ResolverDefinitionPath,
    ScalarTypeName, SelectableFieldName, Span, UnvalidatedTypeName, WithSpan,
};
use graphql_lang_types::{
    ConstantValue, Directive, InputValueDefinition, InterfaceTypeDefinition, NamedTypeAnnotation,
    ObjectTypeDefinition, OutputFieldDefinition, TypeAnnotation,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinedTypeId, InputTypeId, LinkedFieldSelection, NonConstantValue, ObjectId, OutputTypeId,
    ResolverFieldId, ScalarId, Selection, ServerFieldId, ServerIdFieldId, Unwrap,
    VariableDefinition,
};
use lazy_static::lazy_static;

use crate::ResolverVariant;

lazy_static! {
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
    pub static ref ID_GRAPHQL_TYPE: ScalarTypeName = "ID".intern().into();
}

/// The in-memory representation of a schema.
///
/// The generics with which the Schema type is instantiated vary based on
/// how far along in the validation pipeline the schema is. In particular,
/// there is an [UnvalidatedSchema](UnvalidatedSchema) type and a
/// ValidatedSchema type.
///
/// Invariant: a schema is append-only, because pointers into the Schema are in the
/// form of newtype wrappers around u32 indexes (e.g. FieldId, etc.) As a result,
/// the schema does not support removing items.
#[derive(Debug)]
pub struct Schema<
    // Fields contain a field_type: TypeAnnotation<TFieldAssociatedType>
    // Validated: OutputTypeId, Unvalidated: UnvalidatedTypeName
    TFieldAssociatedType,
    // The associated data type of scalars in resolvers' selection sets and unwraps
    // Validated: ValidatedScalarDefinedField, Unvalidated: ()
    TScalarField,
    // The associated data type of linked fields in resolvers' selection sets and unwraps
    // Validated: ObjectId, Unvalidated: ()
    TLinkedField,
    // The associated data type of resolvers' variable definitions
    // Validated: InputTypeId, Unvalidated: UnvalidatedTypeName
    TVariableType,
    // On objects, what does the HashMap of encountered types contain
    // Validated: ValidatedDefinedField, Unvalidated: UnvalidatedObjectFieldInfo
    TEncounteredField,
> {
    pub fields: Vec<SchemaServerField<TypeAnnotation<TFieldAssociatedType>>>,
    pub resolvers: Vec<SchemaResolver<TScalarField, TLinkedField, TVariableType>>,
    pub schema_data: SchemaData<TEncounteredField>,

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

pub(crate) type UnvalidatedSchema =
    Schema<UnvalidatedTypeName, (), (), UnvalidatedTypeName, UnvalidatedObjectFieldInfo>;

/// Distinguishes between server fields and locally-defined resolver fields.
/// TFieldAssociatedType can be a ScalarFieldName in an unvalidated schema, or a
/// ScalarId, in a validated schema.
///
/// TResolverType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// OutputTypeId in a validated schema.
#[derive(Debug, Clone, Copy)]
pub enum DefinedField<TFieldAssociatedType, TResolverType> {
    ServerField(TFieldAssociatedType),
    ResolverField(TResolverType),
}

impl<TFieldAssociatedType, TResolverType> DefinedField<TFieldAssociatedType, TResolverType> {
    pub fn as_server_field(&self) -> Option<&TFieldAssociatedType> {
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

/// On unvalidated schema objects, the encountered types are either a type annotation
/// for server fields with an unvalidated inner type, or a ScalarFieldName (the name of the
/// resolver.)
pub type UnvalidatedObjectFieldInfo =
    DefinedField<TypeAnnotation<UnvalidatedTypeName>, ResolverFieldId>;

pub(crate) type UnvalidatedSchemaData = SchemaData<UnvalidatedObjectFieldInfo>;

pub(crate) type UnvalidatedSchemaField = SchemaServerField<TypeAnnotation<UnvalidatedTypeName>>;

pub(crate) type UnvalidatedSchemaResolver = SchemaResolver<(), (), UnvalidatedTypeName>;

pub(crate) type UnvalidatedSchemaServerField = SchemaServerField<TypeAnnotation<OutputTypeId>>;

#[derive(Debug)]
pub struct SchemaData<TEncounteredField> {
    pub objects: Vec<SchemaObject<TEncounteredField>>,
    pub scalars: Vec<SchemaScalar>,
    pub defined_types: HashMap<UnvalidatedTypeName, DefinedTypeId>,
}

impl<TFieldAssociatedType, TScalarField, TLinkedField, TVariableType, TEncounteredField>
    Schema<TFieldAssociatedType, TScalarField, TLinkedField, TVariableType, TEncounteredField>
{
    /// Get a reference to a given field by its id.
    pub fn field(
        &self,
        field_id: ServerFieldId,
    ) -> &SchemaServerField<TypeAnnotation<TFieldAssociatedType>> {
        &self.fields[field_id.as_usize()]
    }

    /// Get a reference to a given resolver by its id.
    pub fn resolver(
        &self,
        resolver_field_id: ResolverFieldId,
    ) -> &SchemaResolver<TScalarField, TLinkedField, TVariableType> {
        &self.resolvers[resolver_field_id.as_usize()]
    }

    /// Get a reference to the root query_object, if it's defined.
    pub fn query_object(&self) -> Option<&SchemaObject<TEncounteredField>> {
        self.query_type_id
            .as_ref()
            .map(|id| self.schema_data.object(*id))
    }
}

impl<TFieldAssociatedType: Clone, TScalarField, TLinkedField, TVariableType, TEncounteredField>
    Schema<TFieldAssociatedType, TScalarField, TLinkedField, TVariableType, TEncounteredField>
{
    /// Get a reference to a given id field by its id.
    pub fn id_field<TIdFieldAssociatedType: TryFrom<TFieldAssociatedType> + Copy>(
        &self,
        id_field_id: ServerIdFieldId,
    ) -> SchemaIdField<NamedTypeAnnotation<TIdFieldAssociatedType>> {
        let field_id = id_field_id.into();

        let field = self
            .field(field_id)
            .and_then(|e| match e.inner_non_null_named_type() {
                Some(inner) => Ok(NamedTypeAnnotation(inner.0.clone().map(|x| {
                    let y: Result<TIdFieldAssociatedType, _> = x.try_into();
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

impl UnvalidatedSchema {
    pub fn new() -> Self {
        // TODO add __typename
        let fields = vec![];
        let resolvers = vec![];
        let objects = vec![];
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

        Self {
            fields,
            resolvers,
            schema_data: SchemaData {
                objects,
                scalars,
                defined_types,
            },

            id_type_id,
            string_type_id,
            int_type_id,
            float_type_id,
            boolean_type_id,

            query_type_id: None,
        }
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

    pub fn lookup_output_type(
        &self,
        output_type_id: OutputTypeId,
    ) -> SchemaOutputType<TEncounteredField> {
        match output_type_id {
            OutputTypeId::Object(id) => {
                SchemaOutputType::Object(self.objects.get(id.as_usize()).unwrap())
            }
            OutputTypeId::Scalar(id) => {
                SchemaOutputType::Scalar(self.scalars.get(id.as_usize()).unwrap())
            }
        }
    }

    pub fn lookup_input_type(&self, input_type_id: InputTypeId) -> SchemaInputType {
        match input_type_id {
            InputTypeId::Scalar(id) => {
                SchemaInputType::Scalar(self.scalars.get(id.as_usize()).unwrap())
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

fn add_schema_defined_scalar_type(
    scalars: &mut Vec<SchemaScalar>,
    defined_types: &mut HashMap<UnvalidatedTypeName, DefinedTypeId>,
    field_name: &'static str,
    javascript_name: JavascriptName,
) -> ScalarId {
    let scalar_id = scalars.len().into();

    // TODO this is problematic, we have no span (or really, no location) associated with this
    // schema-defined scalar, so we will not be able to properly show error messages if users
    // e.g. have Foo implements String
    let typename = WithSpan::new(field_name.intern().into(), Span::todo_generated());
    scalars.push(SchemaScalar {
        description: None,
        name: typename,
        id: scalar_id,
        javascript_name,
    });
    defined_types.insert(
        typename.item.into(),
        DefinedTypeId::Scalar(scalar_id.into()),
    );
    scalar_id
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaType<'a, TEncounteredField> {
    Object(&'a SchemaObject<TEncounteredField>),
    Scalar(&'a SchemaScalar),
    // Includes input object
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaOutputType<'a, TEncounteredField> {
    Object(&'a SchemaObject<TEncounteredField>),
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
    pub name: WithSpan<IsographObjectTypeName>,
    // maybe this should be Vec<WithSpan<IsographObjectTypeName>>>
    pub interfaces: Vec<WithSpan<InterfaceTypeName>>,
    pub directives: Vec<Directive<ConstantValue>>,
    // TODO the spans of these fields are wrong
    pub fields: Vec<WithSpan<OutputFieldDefinition>>,
}

impl From<ObjectTypeDefinition> for IsographObjectTypeDefinition {
    fn from(value: ObjectTypeDefinition) -> Self {
        Self {
            description: value.description,
            name: value.name.map(|x| x.into()),
            interfaces: value.interfaces,
            directives: value.directives,
            fields: value.fields,
        }
    }
}

impl From<InterfaceTypeDefinition> for IsographObjectTypeDefinition {
    fn from(value: InterfaceTypeDefinition) -> Self {
        Self {
            description: value.description,
            name: value.name.map(|x| x.into()),
            interfaces: value.interfaces,
            directives: value.directives,
            fields: value.fields,
        }
    }
}

/// An object type in the schema.
#[derive(Debug)]
pub struct SchemaObject<TEncounteredField> {
    pub description: Option<DescriptionValue>,
    pub name: IsographObjectTypeName,
    pub id: ObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
    /// TODO remove id_field from fields, and change the type of Option<ServerFieldId>
    /// to something else.
    pub id_field: Option<ServerIdFieldId>,
    pub server_fields: Vec<ServerFieldId>,
    pub resolvers: Vec<ResolverFieldId>,
    pub encountered_fields: HashMap<SelectableFieldName, TEncounteredField>,
    pub valid_refinements: Vec<ValidRefinement>,
}
// TODO iterator of fields that includes id_field?

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
    // TODO this should be WithLocation<SelectableFieldName>
    pub name: SelectableFieldName,
    pub id: ServerFieldId,
    pub associated_data: TData,
    pub parent_type_id: ObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub arguments: Vec<WithSpan<InputValueDefinition>>,
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
    pub name: SelectableFieldName,
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
        // Also, before this is called, we have already converted the field_type to be valid
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
            format!("../{type_name}/{field_name}.isograph")
        } else {
            format!("./{field_name}.isograph")
        }
    }
}

#[derive(Debug)]
pub enum ResolverArtifactKind {
    FetchableOnQuery,
    NonFetchable,
}

#[derive(Debug, Clone, Copy)]
pub enum ResolverActionKind {
    /// No associated js function
    Identity,
    /// Associated js function
    /// TODO the first element should have a type of JavascriptName
    NamedImport((SelectableFieldName, ResolverDefinitionPath)),
    /// Refetch fields
    RefetchField,
    /// Mutation field
    MutationField,
}

#[derive(Debug)]
pub struct SchemaResolver<TScalarField, TLinkedField, TVariableDefinitionType> {
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
        Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
        Vec<WithSpan<Unwrap>>,
    )>,
    pub variant: Option<WithSpan<ResolverVariant>>,

    // TODO should this be create_normalization_ast: bool?
    pub artifact_kind: ResolverArtifactKind,
    pub action_kind: ResolverActionKind,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<TVariableDefinitionType>>>,

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

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameAndArguments {
    pub name: LinkedFieldName,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

pub fn into_name_and_arguments<T, U>(field: &LinkedFieldSelection<T, U>) -> NameAndArguments {
    NameAndArguments {
        name: field.name.item,
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
            associated_data: field_type,
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
            field_type,
        )
    }
}

/// A scalar type in the schema.
#[derive(Debug)]
pub struct SchemaScalar {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithSpan<ScalarTypeName>,
    pub id: ScalarId,
    pub javascript_name: JavascriptName,
}
