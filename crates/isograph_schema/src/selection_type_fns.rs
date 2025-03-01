use std::fmt::Debug;

use common_lang_types::WithSpan;
use isograph_lang_types::{SelectionType, ServerFieldSelection, VariableDefinition};

use crate::{ClientField, ClientPointer, OutputFormat};

#[allow(clippy::type_complexity)]
pub fn selection_set_for_parent_query<
    'a,
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TSelectionTypeVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
>(
    selection_type: &'a SelectionType<
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
) -> &'a [WithSpan<
    ServerFieldSelection<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
    >,
>] {
    match selection_type {
        SelectionType::Scalar(client_field) => client_field.selection_set_for_parent_query(),
        SelectionType::Object(client_pointer) => &client_pointer.reader_selection_set,
    }
}

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

#[allow(clippy::type_complexity)]
pub fn reader_selection_set<
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
) -> &'a [WithSpan<
    ServerFieldSelection<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
    >,
>] {
    match selection_type {
        SelectionType::Object(client_pointer) => &client_pointer.reader_selection_set,
        SelectionType::Scalar(client_field) => &client_field.reader_selection_set,
    }
}
