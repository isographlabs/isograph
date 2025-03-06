use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use isograph_lang_types::{
    SelectionType, ServerObjectId, ServerScalarId, ServerScalarSelectableId, TypeAnnotation,
    VariableDefinition,
};

use crate::{OutputFormat, SchemaServerLinkedFieldFieldVariant};

// TODO convert this to two structs
#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
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
    pub arguments:
        Vec<WithLocation<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,
    pub phantom_data: PhantomData<TOutputFormat>,
}
