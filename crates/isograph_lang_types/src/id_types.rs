use u32_newtypes::{u32_conversion, u32_newtype};

// Any field defined on the server
u32_newtype!(ServerFieldId);
// A field that acts as an id
u32_newtype!(ServerIdFieldId);

u32_conversion!(from: ServerIdFieldId, to: ServerFieldId);

u32_newtype!(ResolverFieldId);

u32_newtype!(ObjectId);

u32_newtype!(ScalarId);

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum OutputTypeId {
    Object(ObjectId),
    Scalar(ScalarId),
}

impl TryFrom<OutputTypeId> for ScalarId {
    type Error = ();

    fn try_from(value: OutputTypeId) -> Result<Self, Self::Error> {
        match value {
            OutputTypeId::Object(_) => Err(()),
            OutputTypeId::Scalar(scalar_id) => Ok(scalar_id),
        }
    }
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
}
