use u32_newtypes::u32_newtype;

u32_newtype!(ServerFieldId);

u32_newtype!(ResolverFieldId);

u32_newtype!(ObjectId);

u32_newtype!(ScalarId);

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum OutputTypeId {
    Object(ObjectId),
    Scalar(ScalarId),
    // Interface
    // Union
    // Enum
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum InputTypeId {
    Scalar(ScalarId),
    // Enum
    // InputObject
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum DefinedTypeId {
    Object(ObjectId),
    Scalar(ScalarId),
}

impl DefinedTypeId {
    pub fn as_output_type_id(self) -> Option<OutputTypeId> {
        match self {
            DefinedTypeId::Object(id) => Some(OutputTypeId::Object(id)),
            DefinedTypeId::Scalar(id) => Some(OutputTypeId::Scalar(id)),
        }
    }

    pub fn as_input_type_id(self) -> Option<InputTypeId> {
        match self {
            DefinedTypeId::Scalar(id) => Some(InputTypeId::Scalar(id)),
            _ => None,
        }
    }

    // as_scalar_id
}
