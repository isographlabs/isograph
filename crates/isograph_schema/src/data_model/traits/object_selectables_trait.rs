use common_lang_types::{SelectableName, ServerObjectEntityName};
use impl_base_types_macro::impl_for_definition_location;
use isograph_lang_types::{DefinitionLocation, Description, TypeAnnotation};
use pico::MemoRef;

use crate::{ClientObjectSelectable, NetworkProtocol, ServerObjectSelectable};

pub type ObjectSelectableId = DefinitionLocation<
    (ServerObjectEntityName, SelectableName),
    (ServerObjectEntityName, SelectableName),
>;

// This is poorly named... its not owned!
pub type OwnedObjectSelectable<TNetworkProtocol> = DefinitionLocation<
    // HACK: Note the owned server selectable
    // This is fixable when memoized functions can return references with 'db lifetime
    MemoRef<ServerObjectSelectable<TNetworkProtocol>>,
    ClientObjectSelectable<TNetworkProtocol>,
>;

pub type ObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    // HACK: Note the owned server selectable
    // This is fixable when memoized functions can return references with 'db lifetime
    ServerObjectSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

pub type MemoRefObjectSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRef<ServerObjectSelectable<TNetworkProtocol>>,
    MemoRef<ClientObjectSelectable<TNetworkProtocol>>,
>;

#[impl_for_definition_location]
pub trait ClientOrServerObjectSelectable {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> SelectableName;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;
    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName>;
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName> {
        self.target_object_entity_name.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName> {
        self.target_object_entity_name.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for &ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName> {
        self.target_object_entity.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.item.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<ServerObjectEntityName> {
        self.target_object_entity.clone()
    }
}
