use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use isograph_lang_types::{
    SelectionType, ServerEntityId, ServerObjectId, ServerScalarId, ServerScalarSelectableId,
    TypeAnnotation, VariableDefinition,
};

use crate::{NetworkProtocol, SchemaServerLinkedFieldFieldVariant};

// TODO convert this to two structs
// NOTE: for the time being, despite the name, this represents both
// fields with point to scalar entities and object entities
#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    /// The name of the server field and the location where it was defined
    /// (an iso literal or Location::Generated).
    pub name: WithLocation<ServerSelectableName>,
    pub id: ServerScalarSelectableId,

    pub target_server_entity: SelectionType<
        TypeAnnotation<ServerScalarId>,
        (
            SchemaServerLinkedFieldFieldVariant,
            TypeAnnotation<ServerObjectId>,
        ),
    >,

    pub parent_type_id: ServerObjectId,
    pub arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    pub phantom_data: PhantomData<TNetworkProtocol>,
}
