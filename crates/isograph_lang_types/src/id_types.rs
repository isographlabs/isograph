use u32_newtypes::{u32_conversion, u32_newtype};

// Any field defined on the server
u32_newtype!(ServerFieldId);
// A field that acts as an id
u32_newtype!(ServerIdFieldId);

u32_conversion!(from: ServerIdFieldId, to: ServerFieldId);

u32_newtype!(ClientFieldId);

u32_newtype!(ObjectId);

u32_newtype!(ScalarId);

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum DefinedTypeId {
    Object(ObjectId),
    Scalar(ScalarId),
}

impl TryFrom<DefinedTypeId> for ScalarId {
    type Error = ();

    fn try_from(value: DefinedTypeId) -> Result<Self, Self::Error> {
        match value {
            DefinedTypeId::Object(_) => Err(()),
            DefinedTypeId::Scalar(scalar_id) => Ok(scalar_id),
        }
    }
}
