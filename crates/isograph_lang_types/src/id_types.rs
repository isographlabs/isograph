// Almost all of these types should not exist. We are using names, not indexes,
// as ids.

use common_lang_types::{SchemaServerObjectEntityName, SchemaServerScalarEntityName};
use u32_newtypes::u32_newtype;

use crate::SelectionType;

// Any field defined on the server
u32_newtype!(ServerObjectSelectableId);
// A field that acts as an id
u32_newtype!(ServerStrongIdFieldId);

u32_newtype!(ClientObjectSelectableId);

pub type ServerEntityName =
    SelectionType<SchemaServerScalarEntityName, SchemaServerObjectEntityName>;

u32_newtype!(RefetchQueryIndex);
