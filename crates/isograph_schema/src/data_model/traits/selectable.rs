use common_lang_types::EntityName;
use impl_base_types_macro::{impl_for_definition_location, impl_for_selection_type};
use isograph_lang_types::DefinitionLocation;

use crate::{MemoRefClientSelectable, MemoRefServerSelectable};

pub type MemoRefSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRefServerSelectable<TNetworkProtocol>,
    MemoRefClientSelectable<TNetworkProtocol>,
>;

#[impl_for_definition_location]
#[impl_for_selection_type]
pub trait SelectableTrait {
    fn parent_object_entity_name(&self) -> EntityName;
}

impl<T: SelectableTrait> SelectableTrait for &T {
    fn parent_object_entity_name(&self) -> EntityName {
        (*self).parent_object_entity_name()
    }
}
