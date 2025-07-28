use std::{collections::BTreeMap, marker::PhantomData};

use common_lang_types::{
    DescriptionValue, JavascriptName, SelectableName, ServerObjectEntityName,
    ServerScalarEntityName, WithLocation, WithSpan,
};
use isograph_lang_types::{impl_with_id, DefinitionLocation, SelectionType};

use crate::{ClientSelectableId, NetworkProtocol, ServerSelectableId};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerScalarEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub name: WithLocation<ServerScalarEntityName>,
    pub javascript_name: JavascriptName,
    pub output_format: PhantomData<TNetworkProtocol>,
}

type SelectableId = DefinitionLocation<ServerSelectableId, ClientSelectableId>;

pub type ServerObjectEntityAvailableSelectables = BTreeMap<SelectableName, SelectableId>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerObjectEntity<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: ServerObjectEntityName,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<ServerObjectEntityName>,

    pub output_associated_data: TNetworkProtocol::SchemaObjectAssociatedData,
}

impl_with_id!(ServerObjectEntity<TNetworkProtocol: NetworkProtocol>, ServerObjectEntityName);

pub type ServerEntity<'a, TNetworkProtocol> = SelectionType<
    &'a ServerScalarEntity<TNetworkProtocol>,
    &'a ServerObjectEntity<TNetworkProtocol>,
>;

pub type ServerEntityName = SelectionType<ServerScalarEntityName, ServerObjectEntityName>;
