use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, GraphQLScalarTypeName, IsographObjectTypeName, JavascriptName,
    SelectableName, WithLocation, WithSpan,
};
use isograph_lang_types::{impl_with_id, DefinitionLocation, SelectionType, ServerObjectEntityId};

use crate::{ClientSelectableId, NetworkProtocol, ServerSelectableId};

#[derive(Debug)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<GraphQLScalarTypeName>,
    pub javascript_name: JavascriptName,
    pub output_format: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ServerScalarEntity<TNetworkProtocol: NetworkProtocol>, GraphQLScalarTypeName);

type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

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
