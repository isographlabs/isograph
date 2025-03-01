use std::fmt::Debug;

use common_lang_types::{ObjectTypeAndFieldName, SelectableFieldName, WithSpan};
use isograph_lang_types::{
    SelectionType, ServerFieldSelection, ServerObjectId, VariableDefinition,
};

use crate::{ClientField, ClientPointer, OutputFormat, SelectionTypeId};

#[allow(clippy::type_complexity)]
pub fn parent_object_id<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TSelectionTypeVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
>(
    selection_type: &SelectionType<
        &ClientField<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
        &ClientPointer<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
    >,
) -> ServerObjectId {
    match selection_type {
        SelectionType::Scalar(client_field) => client_field.parent_object_id,
        SelectionType::Object(client_pointer) => client_pointer.parent_object_id,
    }
}

#[allow(clippy::type_complexity)]
pub fn type_and_field<
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
) -> &'a ObjectTypeAndFieldName {
    match selection_type {
        SelectionType::Scalar(client_field) => &client_field.type_and_field,
        SelectionType::Object(client_pointer) => &client_pointer.type_and_field,
    }
}

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
pub fn selection_type_name<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TSelectionTypeVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
>(
    selection_type: &SelectionType<
        &ClientField<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
        &ClientPointer<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
    >,
) -> SelectableFieldName {
    match selection_type {
        SelectionType::Scalar(client_field) => client_field.name,
        SelectionType::Object(client_pointer) => client_pointer.name.into(),
    }
}

#[allow(clippy::type_complexity)]
pub fn selection_type_id<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TSelectionTypeVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
>(
    selection_type: &SelectionType<
        &ClientField<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
        &ClientPointer<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
    >,
) -> SelectionTypeId {
    match selection_type {
        SelectionType::Scalar(client_field) => SelectionType::Scalar(client_field.id),
        SelectionType::Object(client_pointer) => SelectionType::Object(client_pointer.id),
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
) -> Option<
    &'a [WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >],
> {
    match selection_type {
        SelectionType::Object(client_pointer) => Some(&client_pointer.reader_selection_set),
        SelectionType::Scalar(client_field) => client_field.reader_selection_set.as_deref(),
    }
}
