use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use isograph_lang_types::{ServerFieldId, ServerObjectId, VariableDefinition};

use crate::{OutputFormat, ServerFieldTypeAssociatedData};

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
    pub associated_data: ServerFieldTypeAssociatedData,
    pub parent_type_id: ServerObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub arguments:
        Vec<WithLocation<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,
    pub phantom_data: PhantomData<TOutputFormat>,
}
