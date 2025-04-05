use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, GraphQLScalarTypeName, IsographObjectTypeName, JavascriptName,
    SelectableName, WithLocation, WithSpan,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    impl_with_id, DefinitionLocation, SelectionType, ServerObjectEntityId, ServerScalarEntityId,
};

use crate::{ClientSelectableId, NetworkProtocol, ServerSelectableId};

/// A scalar type in the schema.
#[derive(Debug)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLScalarTypeName>,
    pub javascript_name: JavascriptName,
    pub output_format: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ServerScalarEntity<TNetworkProtocol: NetworkProtocol>, ServerScalarEntityId);

type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

/// An object type in the schema.
#[derive(Debug)]
pub struct ServerObjectEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: IsographObjectTypeName,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,

    pub output_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

impl_with_id!(ServerObjectEntity<TNetworkProtocol: NetworkProtocol>, ServerObjectEntityId);

pub type ServerEntity<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarEntity<TNetworkProtocol>,
    &'a ServerObjectEntity<TNetworkProtocol>,
>;

#[impl_for_selection_type]
pub trait ServerScalarOrObjectEntity {
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName>;
    fn description(&self) -> Option<DescriptionValue>;
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for &ServerScalarEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName> {
        SelectionType::Scalar(self.name.item)
    }

    fn description(&self) -> Option<DescriptionValue> {
        self.description.map(|x| x.item)
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for &ServerObjectEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<GraphQLScalarTypeName, IsographObjectTypeName> {
        SelectionType::Object(self.name)
    }

    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }
}
