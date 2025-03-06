use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use isograph_lang_types::{
    ServerObjectId, ServerScalarId, ServerScalarSelectableId, TypeAnnotation, VariableDefinition,
};

use crate::{OutputFormat, SchemaServerLinkedFieldFieldVariant};

#[derive(Debug, Clone)]
pub struct ServerScalarSelectable<
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerSelectableName>,
    pub id: ServerScalarSelectableId,

    pub target_server_entity: TypeAnnotation<ServerScalarId>,

    pub parent_type_id: ServerObjectId,
    pub arguments:
        Vec<WithLocation<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,
    pub phantom_data: PhantomData<TOutputFormat>,
}

#[derive(Debug, Clone)]
pub struct ServerObjectSelectable<
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
    pub description: Option<DescriptionValue>,
    pub name: WithLocation<ServerSelectableName>,
    pub id: ServerScalarSelectableId,

    pub target_server_entity: TypeAnnotation<ServerObjectId>,
    pub linked_field_variant: SchemaServerLinkedFieldFieldVariant,

    pub parent_type_id: ServerObjectId,
    pub arguments:
        Vec<WithLocation<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,
    pub phantom_data: PhantomData<TOutputFormat>,
}
