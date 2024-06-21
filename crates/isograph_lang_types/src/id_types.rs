use u32_newtypes::{u32_conversion, u32_newtype};

// Any field defined on the server
u32_newtype!(ServerFieldId);
// A field that acts as an id
u32_newtype!(ServerStrongIdFieldId);

u32_conversion!(from: ServerStrongIdFieldId, to: ServerFieldId);

u32_newtype!(ClientFieldId);

u32_newtype!(ServerObjectId);

u32_newtype!(ServerScalarId);

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum SelectableServerFieldId {
    Object(ServerObjectId),
    Scalar(ServerScalarId),
}

impl TryFrom<SelectableServerFieldId> for ServerScalarId {
    type Error = ();

    fn try_from(value: SelectableServerFieldId) -> Result<Self, Self::Error> {
        match value {
            SelectableServerFieldId::Object(_) => Err(()),
            SelectableServerFieldId::Scalar(scalar_id) => Ok(scalar_id),
        }
    }
}

impl TryFrom<SelectableServerFieldId> for ServerObjectId {
    type Error = ();

    fn try_from(value: SelectableServerFieldId) -> Result<Self, Self::Error> {
        match value {
            SelectableServerFieldId::Object(object_id) => Ok(object_id),
            SelectableServerFieldId::Scalar(_) => Err(()),
        }
    }
}

u32_newtype!(RefetchQueryIndex);
