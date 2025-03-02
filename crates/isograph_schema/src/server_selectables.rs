use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{DescriptionValue, ServerSelectableName, WithLocation};
use isograph_lang_types::{ServerFieldId, ServerObjectId, VariableDefinition};

use crate::OutputFormat;

// TODO convert this to two structs
#[derive(Debug, Clone)]
pub struct SchemaServerField<
    TData,
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
    pub description: Option<DescriptionValue>,
    /// The name of the server field and the location where it was defined
    /// (an iso literal or Location::Generated).
    pub name: WithLocation<ServerSelectableName>,
    pub id: ServerFieldId,
    pub associated_data: TData,
    pub parent_type_id: ServerObjectId,
    // pub directives: Vec<Directive<ConstantValue>>,
    pub arguments:
        Vec<WithLocation<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,
    // TODO remove this. This is indicative of poor modeling.
    pub is_discriminator: bool,
    pub phantom_data: PhantomData<TOutputFormat>,
}

impl<
        TData,
        TClientFieldVariableDefinitionAssociatedData: Clone + Ord + Debug,
        TOutputFormat: OutputFormat,
    > SchemaServerField<TData, TClientFieldVariableDefinitionAssociatedData, TOutputFormat>
{
    pub fn and_then<TData2, E>(
        &self,
        convert: impl FnOnce(&TData) -> Result<TData2, E>,
    ) -> Result<
        SchemaServerField<TData2, TClientFieldVariableDefinitionAssociatedData, TOutputFormat>,
        E,
    > {
        Ok(SchemaServerField {
            description: self.description,
            name: self.name,
            id: self.id,
            associated_data: convert(&self.associated_data)?,
            parent_type_id: self.parent_type_id,
            arguments: self.arguments.clone(),
            is_discriminator: self.is_discriminator,
            phantom_data: PhantomData,
        })
    }

    pub fn map<TData2, E>(
        &self,
        convert: impl FnOnce(&TData) -> TData2,
    ) -> SchemaServerField<TData2, TClientFieldVariableDefinitionAssociatedData, TOutputFormat>
    {
        SchemaServerField {
            description: self.description,
            name: self.name,
            id: self.id,
            associated_data: convert(&self.associated_data),
            parent_type_id: self.parent_type_id,
            arguments: self.arguments.clone(),
            is_discriminator: self.is_discriminator,
            phantom_data: PhantomData,
        }
    }
}
