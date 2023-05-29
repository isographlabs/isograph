use std::collections::HashMap;

use boulton_lang_types::SelectionSetAndUnwraps;
use common_lang_types::{
    DefinedField, DescriptionValue, FieldDefinitionName, FieldId, HasName, LinkedFieldName,
    ObjectId, ObjectTypeName, OutputTypeId, OutputTypeName, ResolverDefinitionPath,
    ScalarFieldName, ScalarId, ScalarTypeName, TypeId, TypeWithFieldsId, TypeWithFieldsName,
    UnvalidatedTypeName, ValidLinkedFieldType, ValidScalarFieldType, ValidTypeAnnotationInnerType,
};
use intern::string_key::Intern;

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
> {
    pub fields: Vec<
        SchemaField<
            DefinedField<TServerType, SchemaResolverDefinitionInfo<TScalarField, TLinkedField>>,
        >,
    >,
    pub schema_data: SchemaData,

    // Well known types
    pub id_type: ScalarId,
    pub string_type: ScalarId,
    // float
    // typename
    pub query_type: Option<ObjectId>,
    // Subscription
    // Mutation
}
pub(crate) type UnvalidatedSchemaField = SchemaField<
    DefinedField<
        UnvalidatedTypeName,
        SchemaResolverDefinitionInfo<ScalarFieldName, LinkedFieldName>,
    >,
>;

#[derive(Debug)]
pub struct SchemaData {
    pub objects: Vec<SchemaObject>,
    pub scalars: Vec<SchemaScalar>,
    // enums, unions, interfaces, input objects
    pub defined_types: HashMap<UnvalidatedTypeName, TypeId>,
}

pub(crate) type UnvalidatedSchema = Schema<UnvalidatedTypeName, ScalarFieldName, LinkedFieldName>;

impl UnvalidatedSchema {
    pub fn new() -> Self {
        // TODO add __typename
        let fields = vec![];
        let objects = vec![];
        let mut scalars = vec![];
        let mut defined_types = HashMap::default();

        let id_type_id = add_schema_defined_scalar_type(&mut scalars, &mut defined_types, "ID");
        let string_type_id =
            add_schema_defined_scalar_type(&mut scalars, &mut defined_types, "String");

        Self {
            fields,
            schema_data: SchemaData {
                objects,
                scalars,
                defined_types,
            },
            id_type: id_type_id,
            string_type: string_type_id,
            query_type: None,
        }
    }
}

impl SchemaData {
    pub fn lookup_type_with_fields(&self, type_id: TypeWithFieldsId) -> SchemaTypeWithFields {
        match type_id {
            TypeWithFieldsId::Object(object_id) => {
                // TODO replace with an unchecked lookup?
                SchemaTypeWithFields::Object(&self.objects[object_id.as_usize()])
            }
        }
    }

    pub fn lookup_unvalidated_type(&self, type_id: TypeId) -> SchemaType {
        match type_id {
            TypeId::Object(id) => SchemaType::Object(self.objects.get(id.as_usize()).unwrap()),
            TypeId::Scalar(id) => SchemaType::Scalar(self.scalars.get(id.as_usize()).unwrap()),
        }
    }

    pub fn lookup_output_type(&self, output_type_id: OutputTypeId) -> SchemaOutputType {
        match output_type_id {
            OutputTypeId::Object(id) => {
                SchemaOutputType::Object(self.objects.get(id.as_usize()).unwrap())
            }
            OutputTypeId::Scalar(id) => {
                SchemaOutputType::Scalar(self.scalars.get(id.as_usize()).unwrap())
            }
        }
    }
}

fn add_schema_defined_scalar_type(
    scalars: &mut Vec<SchemaScalar>,
    defined_types: &mut HashMap<UnvalidatedTypeName, TypeId>,
    field_name: &'static str,
) -> ScalarId {
    let scalar_id = scalars.len().into();

    let typename = field_name.intern().into();
    scalars.push(SchemaScalar {
        description: None,
        name: typename,
        id: scalar_id,
    });
    defined_types.insert(typename.into(), TypeId::Scalar(scalar_id.into()));
    scalar_id
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaTypeWithFields<'a> {
    Object(&'a SchemaObject),
}
impl<'a> SchemaTypeWithFields<'a> {
    pub fn encountered_field_names(
        &self,
    ) -> &HashMap<FieldDefinitionName, DefinedField<UnvalidatedTypeName, ScalarFieldName>> {
        match self {
            SchemaTypeWithFields::Object(object) => &object.encountered_field_names,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaType<'a> {
    Object(&'a SchemaObject),
    Scalar(&'a SchemaScalar),
    // Includes input object
}

impl<'a> HasName for SchemaTypeWithFields<'a> {
    type Name = TypeWithFieldsName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaTypeWithFields::Object(object) => object.name.into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaOutputType<'a> {
    Object(&'a SchemaObject),
    Scalar(&'a SchemaScalar),
    // excludes input object
}

impl<'a> HasName for SchemaOutputType<'a> {
    type Name = OutputTypeName;

    fn name(&self) -> Self::Name {
        match self {
            SchemaOutputType::Object(object) => object.name.into(),
            SchemaOutputType::Scalar(scalar) => scalar.name.into(),
        }
    }
}

#[derive(Debug)]
// TODO UnvalidatedTypeName => OutputTypeId via generic, and ScalarFieldName => OutputTypeId?
pub struct SchemaObject {
    pub description: Option<DescriptionValue>,
    pub name: ObjectTypeName,
    pub id: ObjectId,
    // pub interfaces: Vec<InterfaceTypeName>,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub fields: Vec<FieldId>,
    pub encountered_field_names:
        HashMap<FieldDefinitionName, DefinedField<UnvalidatedTypeName, ScalarFieldName>>,
}

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

/// This describes the definition of a resolver, e.g. the path to its definition.
/// It will be contained in a hashmap, the key in that hash map is the resolver name.
/// But that's weird, maybe it needs the field.
#[derive(Debug)]
// TODO map selection_set
pub struct SchemaResolverDefinitionInfo<
    TScalarField: ValidScalarFieldType,
    TLinkedField: ValidLinkedFieldType,
> {
    pub resolver_definition_path: ResolverDefinitionPath,
    pub selection_set_and_unwraps: Option<SelectionSetAndUnwraps<TScalarField, TLinkedField>>,
}

impl<TScalarField: ValidScalarFieldType, TLinkedField: ValidLinkedFieldType>
    SchemaResolverDefinitionInfo<TScalarField, TLinkedField>
{
    pub fn map<TNewScalarField: ValidScalarFieldType, TNewLinkedField: ValidLinkedFieldType>(
        self,
        map: impl FnOnce(
            SelectionSetAndUnwraps<TScalarField, TLinkedField>,
        ) -> SelectionSetAndUnwraps<TNewScalarField, TNewLinkedField>,
    ) -> SchemaResolverDefinitionInfo<TNewScalarField, TNewLinkedField> {
        SchemaResolverDefinitionInfo {
            resolver_definition_path: self.resolver_definition_path,
            selection_set_and_unwraps: self
                .selection_set_and_unwraps
                .map(|selection_set_and_unwraps| map(selection_set_and_unwraps)),
        }
    }
}

#[derive(Debug)]
pub struct SchemaScalar {
    pub description: Option<DescriptionValue>,
    pub name: ScalarTypeName,
    pub id: ScalarId,
    // pub directives: Vec<Directive<ConstantValue>>,
}
