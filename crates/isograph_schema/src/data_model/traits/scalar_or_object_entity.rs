use common_lang_types::{DescriptionValue, GraphQLScalarTypeName, IsographObjectTypeName};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::SelectionType;

use crate::{NetworkProtocol, ServerObjectEntity, ServerScalarEntity};

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
