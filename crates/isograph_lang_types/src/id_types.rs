use u32_newtypes::{u32_conversion, u32_newtype};

use crate::SelectionType;

// Any field defined on the server
u32_newtype!(ServerScalarSelectableId);
u32_newtype!(ServerObjectSelectableId);
// A field that acts as an id
u32_newtype!(ServerStrongIdFieldId);

u32_conversion!(from: ServerStrongIdFieldId, to: ServerScalarSelectableId);

u32_newtype!(ClientScalarSelectableId);

u32_newtype!(ClientObjectSelectableId);

u32_newtype!(ServerObjectEntityId);

u32_newtype!(ServerScalarEntityId);

pub type ServerEntityId = SelectionType<ServerScalarEntityId, ServerObjectEntityId>;

u32_newtype!(RefetchQueryIndex);
