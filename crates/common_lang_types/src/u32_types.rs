use u32_newtypes::u32_newtype;

use crate::{ValidLinkedFieldType, ValidScalarFieldType, ValidTypeAnnotationInnerType};

u32_newtype!(ServerFieldId);
impl ValidTypeAnnotationInnerType for ServerFieldId {}

u32_newtype!(ResolverFieldId);

u32_newtype!(ObjectId);
impl ValidTypeAnnotationInnerType for ObjectId {}

impl From<ObjectId> for TypeWithFieldsId {
    fn from(id: ObjectId) -> Self {
        TypeWithFieldsId::Object(id)
    }
}

u32_newtype!(ScalarId);
impl ValidTypeAnnotationInnerType for ScalarId {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum OutputTypeId {
    Object(ObjectId),
    Scalar(ScalarId),
    // Interface
    // Union
    // Enum
}
impl ValidTypeAnnotationInnerType for OutputTypeId {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum InputTypeId {
    Scalar(ScalarId),
    // Enum
    // InputObject
}
impl ValidTypeAnnotationInnerType for InputTypeId {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum TypeWithoutFieldsId {
    Scalar(ScalarId),
    // Enum
}
impl ValidTypeAnnotationInnerType for TypeWithoutFieldsId {}
impl ValidScalarFieldType for TypeWithoutFieldsId {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum TypeId {
    Object(ObjectId),
    Scalar(ScalarId),
    // Enum
    // Interface
    // Union
    // AND ALSO: InputObject
}
impl ValidTypeAnnotationInnerType for TypeId {}

impl TypeId {
    pub fn as_output_type_id(self) -> Option<OutputTypeId> {
        match self {
            TypeId::Object(id) => Some(OutputTypeId::Object(id)),
            TypeId::Scalar(id) => Some(OutputTypeId::Scalar(id)),
        }
    }

    pub fn as_input_type_id(self) -> Option<InputTypeId> {
        match self {
            TypeId::Scalar(id) => Some(InputTypeId::Scalar(id)),
            _ => None,
        }
    }

    // as_scalar_id
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum TypeWithFieldsId {
    Object(ObjectId),
    // Interface
    // Union
}
impl ValidLinkedFieldType for TypeWithFieldsId {}
impl ValidTypeAnnotationInnerType for TypeWithFieldsId {}
