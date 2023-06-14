use std::collections::HashMap;

use common_lang_types::{
    DefinedField, DescriptionValue, FieldDefinitionName, FieldId, HasName, InputTypeId,
    InputTypeName, JavascriptName, ObjectId, ObjectTypeName, OutputTypeId, OutputTypeName,
    ResolverDefinitionPath, ScalarFieldName, ScalarId, ScalarTypeName, TypeAndField, TypeId,
    TypeWithFieldsId, TypeWithFieldsName, TypeWithoutFieldsId, TypeWithoutFieldsName,
    UnvalidatedTypeName, ValidLinkedFieldType, ValidScalarFieldType, ValidTypeAnnotationInnerType,
    WithSpan,
};
use graphql_lang_types::TypeAnnotation;
use intern::string_key::Intern;
use isograph_lang_types::{Selection, Unwrap, VariableDefinition};
use lazy_static::lazy_static;

use crate::ResolverVariant;

lazy_static! {
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

/// The first, unvalidated in-memory representation of a schema.
///
/// The things that are unvalidated include:
/// - That each field's type exists
/// - That each resolver's fragment is valid, i.e. that fields
///   exist, no duplicates, etc.
///
/// This is almost certainly a subset of validations we should do.
///
/// Invariant: a schema is append-only, because pointers into the Schema are in the
/// form of newtype wrappers around u32 indexes (e.g. FieldId, etc.) As a result,
/// the schema does not support removing items.
///
/// TServerType: the type of a parsed or validated server field in the fields array.
/// In an unvalidated schema, this is UnvalidatedTypeName. In a validated schema,
/// this is OutputTypeId.
#[derive(Debug)]
pub struct Schema<
    TServerType: ValidTypeAnnotationInnerType,
    TScalarField: ValidScalarFieldType,
    TLinkedField: ValidLinkedFieldType,
    TVariableType: ValidTypeAnnotationInnerType,
    TEncounteredField,
> {
    // TODO fields should probably be two vectors: server_fields and resolvers, and have
    // separate ID types.
    pub fields: Vec<
        SchemaField<
            DefinedField<
                TypeAnnotation<TServerType>,
                SchemaResolverDefinitionInfo<TScalarField, TLinkedField, TVariableType>,
            >,
        >,
    >,
    pub schema_data: SchemaData<TEncounteredField>,

    // Well known types
    pub id_type_id: ScalarId,
    pub string_type_id: ScalarId,
    pub float_type_id: ScalarId,
    pub boolean_type_id: ScalarId,
    pub int_type_id: ScalarId,

    // typename
    pub query_type_id: Option<ObjectId>,
    // Subscription
    // Mutation
}

pub(crate) type UnvalidatedSchema =
    Schema<UnvalidatedTypeName, (), (), UnvalidatedTypeName, UnvalidatedObjectFieldInfo>;
pub type UnvalidatedObjectFieldInfo =
    DefinedField<TypeAnnotation<UnvalidatedTypeName>, ScalarFieldName>;
pub(crate) type UnvalidatedSchemaData = SchemaData<UnvalidatedObjectFieldInfo>;

pub(crate) type UnvalidatedSchemaField = SchemaField<
    DefinedField<
        TypeAnnotation<UnvalidatedTypeName>,
        SchemaResolverDefinitionInfo<(), (), UnvalidatedTypeName>,
    >,
>;

#[derive(Debug)]
pub struct SchemaData<TEncounteredField> {
    pub objects: Vec<SchemaObject<TEncounteredField>>,
    pub scalars: Vec<SchemaScalar>,
    // enums, unions, interfaces, input objects
    pub defined_types: HashMap<UnvalidatedTypeName, TypeId>,
}

impl<
        TServerType: ValidTypeAnnotationInnerType,
        TScalarField: ValidScalarFieldType,
        TLinkedField: ValidLinkedFieldType,
        TVariableType: ValidTypeAnnotationInnerType,
        TEncounteredField,
    > Schema<TServerType, TScalarField, TLinkedField, TVariableType, TEncounteredField>
{
    pub fn field(
        &self,
        field_id: FieldId,
    ) -> &SchemaField<
        DefinedField<
            TypeAnnotation<TServerType>,
            SchemaResolverDefinitionInfo<TScalarField, TLinkedField, TVariableType>,
        >,
    > {
        &self.fields[field_id.as_usize()]
    }

    pub fn query_object(&self) -> Option<&SchemaObject<TEncounteredField>> {
        self.query_type_id
            .as_ref()
            .map(|id| self.schema_data.object(*id))
    }
}

impl UnvalidatedSchema {
    pub fn new() -> Self {
        // TODO add __typename
        let fields = vec![];
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
            schema_data: SchemaData {
                objects,
                scalars,
                defined_types,
            },
            id_type_id,
            string_type_id,
            query_type_id: None,
            int_type_id,
            float_type_id,
            boolean_type_id,
        }
    }
}

impl<TEncounteredField> SchemaData<TEncounteredField> {
    pub fn lookup_type_with_fields(
        &self,
        type_id: TypeWithFieldsId,
    ) -> SchemaTypeWithFields<TEncounteredField> {
        match type_id {
            TypeWithFieldsId::Object(object_id) => {
                // TODO replace with an unchecked lookup?
                SchemaTypeWithFields::Object(&self.objects[object_id.as_usize()])
            }
        }
    }

    pub fn lookup_type_without_fields(
        &self,
        type_without_fields_id: TypeWithoutFieldsId,
    ) -> SchemaTypeWithoutFields {
        match type_without_fields_id {
            TypeWithoutFieldsId::Scalar(scalar_id) => {
                SchemaTypeWithoutFields::Scalar(self.scalar(scalar_id))
            }
        }
    }

    pub fn scalar(&self, scalar_id: ScalarId) -> &SchemaScalar {
        self.scalars
            .get(scalar_id.as_usize())
            .expect("Invalid ScalarId")
    }

    pub fn lookup_unvalidated_type(&self, type_id: TypeId) -> SchemaType<TEncounteredField> {
        match type_id {
            TypeId::Object(id) => SchemaType::Object(self.objects.get(id.as_usize()).unwrap()),
            TypeId::Scalar(id) => SchemaType::Scalar(self.scalars.get(id.as_usize()).unwrap()),
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

    pub fn object(&self, object_id: ObjectId) -> &SchemaObject<TEncounteredField> {
        self.objects
            .get(object_id.as_usize())
            .expect("ObjectId should exist, this indicates a bug in Isograph")
    }
}

fn add_schema_defined_scalar_type(
    scalars: &mut Vec<SchemaScalar>,
    defined_types: &mut HashMap<UnvalidatedTypeName, TypeId>,
    field_name: &'static str,
    javascript_name: JavascriptName,
) -> ScalarId {
    let scalar_id = scalars.len().into();

    let typename = field_name.intern().into();
    scalars.push(SchemaScalar {
        description: None,
        name: typename,
        id: scalar_id,
        javascript_name,
    });
    defined_types.insert(typename.into(), TypeId::Scalar(scalar_id.into()));
    scalar_id
}

#[derive(Debug)]
pub enum SchemaTypeWithFields<'a, TEncounteredField> {
    Object(&'a SchemaObject<TEncounteredField>),
}

impl<'a, TEncounteredField> Copy for SchemaTypeWithFields<'a, TEncounteredField> {}
impl<'a, TEncounteredField> Clone for SchemaTypeWithFields<'a, TEncounteredField> {
    fn clone(&self) -> Self {
        match self {
            Self::Object(arg0) => Self::Object(arg0),
        }
    }
}

pub type UnvalidatedSchemaTypeWithFields<'a> = SchemaTypeWithFields<'a, UnvalidatedObjectFieldInfo>;

impl<'a, TEncounteredField> From<&'a SchemaObject<TEncounteredField>>
    for SchemaTypeWithFields<'a, TEncounteredField>
{
    fn from(object: &'a SchemaObject<TEncounteredField>) -> Self {
        SchemaTypeWithFields::Object(object)
    }
}

impl<'a, TEncounteredField> SchemaTypeWithFields<'a, TEncounteredField> {
    pub fn encountered_field_names(&self) -> &HashMap<FieldDefinitionName, TEncounteredField> {
        match self {
            SchemaTypeWithFields::Object(object) => &object.encountered_field_names,
        }
    }

    pub fn fields(&self) -> &[FieldId] {
        match self {
            SchemaTypeWithFields::Object(object) => &object.fields,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaType<'a, TEncounteredField> {
    Object(&'a SchemaObject<TEncounteredField>),
    Scalar(&'a SchemaScalar),
    // Includes input object
}

impl<'a, TEncounteredField> HasName for SchemaTypeWithFields<'a, TEncounteredField> {
    type Name = TypeWithFieldsName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaTypeWithFields::Object(object) => object.name.into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaOutputType<'a, TEncounteredField> {
    Object(&'a SchemaObject<TEncounteredField>),
    Scalar(&'a SchemaScalar),
    // excludes input object
}

impl<'a, TEncounteredField> HasName for SchemaOutputType<'a, TEncounteredField> {
    type Name = OutputTypeName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaOutputType::Object(object) => object.name.into(),
            SchemaOutputType::Scalar(scalar) => scalar.name.into(),
        }
    }
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
            SchemaInputType::Scalar(x) => (x.name).into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaTypeWithoutFields<'a> {
    Scalar(&'a SchemaScalar),
    // enum
}

impl<'a> HasName for SchemaTypeWithoutFields<'a> {
    type Name = TypeWithoutFieldsName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaTypeWithoutFields::Scalar(scalar) => scalar.name.into(),
        }
    }
}

impl<'schema> SchemaTypeWithoutFields<'schema> {
    pub fn javascript_name(&self) -> JavascriptName {
        match self {
            SchemaTypeWithoutFields::Scalar(scalar) => scalar.javascript_name,
        }
    }
}

#[derive(Debug)]
pub struct SchemaObject<TEncounteredField> {
    pub description: Option<DescriptionValue>,
    pub name: ObjectTypeName,
    pub id: ObjectId,
    // pub interfaces: Vec<InterfaceTypeName>,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<FieldId>,
    // TODO: the ScalarFieldName in DefinedField is pretty useless. Consider
    // storing more useful information there, like the field index or something.
    pub encountered_field_names: HashMap<FieldDefinitionName, TEncounteredField>,
}
// Unvalidated => TScalarField: TypeAnnotation<UnvalidatedTypeName>,
// Validated => FieldId

#[derive(Debug)]
pub struct SchemaField<T> {
    pub description: Option<DescriptionValue>,
    pub name: FieldDefinitionName,
    pub id: FieldId,
    pub field_type: T,
    pub parent_type_id: TypeWithFieldsId,
    // pub arguments: Vec<InputValue<ConstantValue>>,
    // pub directives: Vec<Directive<ConstantValue>>,
}

impl<T> SchemaField<T> {
    pub fn split(self) -> (SchemaField<()>, T) {
        let Self {
            description,
            name,
            id,
            field_type,
            parent_type_id,
        } = self;
        (
            SchemaField {
                description,
                name,
                id,
                field_type: (),
                parent_type_id,
            },
            field_type,
        )
    }
}

#[derive(Debug)]
// TODO map selection_set
pub struct SchemaResolverDefinitionInfo<
    TScalarField: ValidScalarFieldType,
    TLinkedField: ValidLinkedFieldType,
    // TODO this should be restricted to ValidTypeAnnotationInnerInputType
    TVariableDefinitionType: ValidTypeAnnotationInnerType,
> {
    pub resolver_definition_path: ResolverDefinitionPath,
    pub selection_set_and_unwraps: Option<(
        Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
        Vec<WithSpan<Unwrap>>,
    )>,
    pub field_id: FieldId,
    pub variant: Option<WithSpan<ResolverVariant>>,
    pub is_fetchable: bool,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<TVariableDefinitionType>>>,
    pub type_and_field: TypeAndField,
    pub has_associated_js_function: bool,
}

impl<
        TScalarField: ValidScalarFieldType,
        TLinkedField: ValidLinkedFieldType,
        TVariableDefinitionType: ValidTypeAnnotationInnerType,
    > SchemaResolverDefinitionInfo<TScalarField, TLinkedField, TVariableDefinitionType>
{
    pub fn map<TNewScalarField: ValidScalarFieldType, TNewLinkedField: ValidLinkedFieldType>(
        self,
        map: impl FnOnce(
            (
                Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
                Vec<WithSpan<Unwrap>>,
            ),
        ) -> (
            Vec<WithSpan<Selection<TNewScalarField, TNewLinkedField>>>,
            Vec<WithSpan<Unwrap>>,
        ),
    ) -> SchemaResolverDefinitionInfo<TNewScalarField, TNewLinkedField, TVariableDefinitionType>
    {
        SchemaResolverDefinitionInfo {
            resolver_definition_path: self.resolver_definition_path,
            selection_set_and_unwraps: self
                .selection_set_and_unwraps
                .map(|selection_set_and_unwraps| map(selection_set_and_unwraps)),
            field_id: self.field_id,
            variant: self.variant,
            is_fetchable: self.is_fetchable,
            variable_definitions: self.variable_definitions,
            type_and_field: self.type_and_field,
            has_associated_js_function: self.has_associated_js_function,
        }
    }
}

#[derive(Debug)]
pub struct SchemaScalar {
    pub description: Option<DescriptionValue>,
    pub name: ScalarTypeName,
    pub id: ScalarId,
    pub javascript_name: JavascriptName,
    // pub directives: Vec<Directive<ConstantValue>>,
}
