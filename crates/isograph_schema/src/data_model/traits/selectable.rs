use common_lang_types::{SelectableName, ServerObjectEntityName};
use impl_base_types_macro::impl_for_definition_location;
use isograph_lang_types::{DefinitionLocation, Description};

use crate::{ClientSelectable, ServerSelectable};

pub type Selectable<'a, TNetworkProtocol> = DefinitionLocation<
    ServerSelectable<'a, TNetworkProtocol>,
    ClientSelectable<'a, TNetworkProtocol>,
>;

#[impl_for_definition_location]
pub trait SelectableTrait {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> SelectableName;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;
}
