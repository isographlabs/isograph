use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use isograph_lang_types::{
    SelectionType, ServerFieldId, ServerObjectId, ServerScalarId, TypeAnnotation,
    VariableDefinition,
};

use crate::{OutputFormat, SchemaServerLinkedFieldFieldVariant};

// TODO convert this to two structs
#[derive(Debug, Clone)]
pub struct SchemaServerField<
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
    pub description: Option<DescriptionValue>,
    /// The name of the server field and the location where it was defined
    /// (an iso literal or Location::Generated).
    pub name: WithLocation<ServerSelectableName>,
    pub id: ServerFieldId,

    // TODO linked_field_variant belongs on the SelectionType::Object variant of selection_type
    pub linked_field_variant: SchemaServerLinkedFieldFieldVariant,
    pub target_server_entity: TypeAnnotation<SelectionType<ServerScalarId, ServerObjectId>>,

    pub parent_type_id: ServerObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub arguments:
        Vec<WithLocation<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,
    pub phantom_data: PhantomData<TOutputFormat>,
}
