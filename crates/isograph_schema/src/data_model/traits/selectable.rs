use common_lang_types::{SelectableName, ServerObjectEntityName, WithLocation};
use impl_base_types_macro::{impl_for_definition_location, impl_for_selection_type};
use isograph_lang_types::{DefinitionLocation, Description, VariableDefinition};

use crate::{
    MemoRefServerSelectable, OwnedClientSelectable, OwnedServerSelectable, ServerEntityName,
};

pub type Selectable<TNetworkProtocol> = DefinitionLocation<
    OwnedServerSelectable<TNetworkProtocol>,
    OwnedClientSelectable<TNetworkProtocol>,
>;
pub type MemoRefSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRefServerSelectable<TNetworkProtocol>,
    OwnedClientSelectable<TNetworkProtocol>,
>;

pub type OwnedSelectable<TNetworkProtocol> = DefinitionLocation<
    OwnedServerSelectable<TNetworkProtocol>,
    OwnedClientSelectable<TNetworkProtocol>,
>;

#[impl_for_definition_location]
#[impl_for_selection_type]
pub trait SelectableTrait {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> WithLocation<SelectableName>;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;
    // TODO convert this to &[VariableDefinition] or &[WithSpan] or &[WithLocation]
    // i.e. settle on one!
    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>>;
}

impl<T: SelectableTrait> SelectableTrait for &T {
    fn description(&self) -> Option<Description> {
        (*self).description()
    }

    fn name(&self) -> WithLocation<SelectableName> {
        (*self).name()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        (*self).parent_object_entity_name()
    }

    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>> {
        (*self).arguments()
    }
}
