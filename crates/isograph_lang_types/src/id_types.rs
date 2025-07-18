// Almost all of these types should not exist. We are using names, not indexes,
// as ids.

use common_lang_types::{SchemaServerObjectEntityName, SchemaServerScalarEntityName};
use u32_newtypes::u32_newtype;

use crate::SelectionType;

u32_newtype!(ClientObjectSelectableId);

pub type ServerEntityName =
    SelectionType<SchemaServerScalarEntityName, SchemaServerObjectEntityName>;

u32_newtype!(RefetchQueryIndex);
