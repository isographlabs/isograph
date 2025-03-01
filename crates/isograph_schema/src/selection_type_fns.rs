use std::fmt::Debug;

use common_lang_types::WithSpan;
use isograph_lang_types::{SelectionType, VariableDefinition};

use crate::{ClientField, ClientPointer, OutputFormat};

#[allow(clippy::type_complexity)]
pub fn variable_definitions<
    'a,
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TSelectionTypeVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
>(
    selection_type: &SelectionType<
        &'a ClientField<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
        &'a ClientPointer<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
    >,
) -> &'a [WithSpan<VariableDefinition<TSelectionTypeVariableDefinitionAssociatedData>>] {
    match selection_type {
        SelectionType::Object(client_pointer) => &client_pointer.variable_definitions,
        SelectionType::Scalar(client_field) => &client_field.variable_definitions,
    }
}
