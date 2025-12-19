use common_lang_types::{EntityName, SelectableName};
use impl_base_types_macro::impl_for_definition_location;
use isograph_lang_types::{DefinitionLocation, Description, TypeAnnotation};
use pico::MemoRef;

use crate::{ClientObjectSelectable, NetworkProtocol, ServerObjectSelectable};

pub type ObjectSelectableId =
    DefinitionLocation<(EntityName, SelectableName), (EntityName, SelectableName)>;

pub type BorrowedObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerObjectSelectable<TNetworkProtocol>,
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
    fn parent_object_entity_name(&self) -> EntityName;
    fn target_object_entity_name(&self) -> TypeAnnotation<EntityName>;
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<EntityName> {
        self.target_entity_name.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<EntityName> {
        self.target_entity_name.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for &ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<EntityName> {
        self.target_entity_name.clone()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientOrServerObjectSelectable
    for ServerObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_object_entity_name
    }

    fn target_object_entity_name(&self) -> TypeAnnotation<EntityName> {
        self.target_entity_name.clone()
    }
}
