use u32_newtypes::{u32_conversion, u32_newtype};

use crate::SelectionType;

// Any field defined on the server
u32_newtype!(ServerScalarSelectableId);
u32_newtype!(ServerObjectSelectableId);
// A field that acts as an id
u32_newtype!(ServerStrongIdFieldId);

u32_conversion!(from: ServerStrongIdFieldId, to: ServerScalarSelectableId);

u32_newtype!(ClientFieldId);

u32_newtype!(ClientPointerId);

u32_newtype!(ServerObjectId);

u32_newtype!(ServerScalarId);

pub type ServerEntityId = SelectionType<ServerScalarId, ServerObjectId>;

impl TryFrom<ServerEntityId> for ServerScalarId {
    type Error = ();

    fn try_from(value: ServerEntityId) -> Result<Self, Self::Error> {
        match value {
            ServerEntityId::Object(_) => Err(()),
            ServerEntityId::Scalar(scalar_id) => Ok(scalar_id),
        }
    }
}

impl TryFrom<ServerEntityId> for ServerObjectId {
    type Error = ();

    fn try_from(value: ServerEntityId) -> Result<Self, Self::Error> {
        match value {
            ServerEntityId::Object(object_id) => Ok(object_id),
            ServerEntityId::Scalar(_) => Err(()),
        }
    }
}

u32_newtype!(RefetchQueryIndex);
