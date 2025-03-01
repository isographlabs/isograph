use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ClientPointerFieldName, DescriptionValue, ObjectTypeAndFieldName, SelectableFieldName, WithSpan,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, SelectionType, ServerFieldSelection, ServerObjectId,
    TypeAnnotation, VariableDefinition,
};

use crate::{ClientFieldVariant, OutputFormat, RefetchStrategy, SelectionTypeId};

pub type FieldOrPointerName = SelectionType<SelectableFieldName, ClientPointerFieldName>;

#[derive(Debug)]
pub struct ClientField<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
    pub description: Option<DescriptionValue>,
    // TODO make this a ClientFieldName that can be converted into a SelectableFieldName
    pub name: SelectableFieldName,
    pub id: ClientFieldId,
    pub reader_selection_set: Vec<
        WithSpan<
            ServerFieldSelection<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    >,

    // None -> not refetchable
    // TODO - this is only used if variant === imperatively loaded field
    // consider moving it into that struct.
    pub refetch_strategy: Option<
        RefetchStrategy<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub variable_definitions:
        Vec<WithSpan<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_id: ServerObjectId,
    pub output_format: PhantomData<TOutputFormat>,
}

#[derive(Debug)]
pub struct ClientPointer<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
    TOutputFormat: OutputFormat,
> {
    pub description: Option<DescriptionValue>,
    pub name: ClientPointerFieldName,
    pub id: ClientPointerId,
    pub to: TypeAnnotation<ServerObjectId>,

    pub reader_selection_set: Vec<
        WithSpan<
            ServerFieldSelection<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    >,

    pub refetch_strategy: RefetchStrategy<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
    >,

    pub variable_definitions:
        Vec<WithSpan<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_id: ServerObjectId,

    pub output_format: PhantomData<TOutputFormat>,
}

#[impl_for_selection_type]
pub trait FieldOrPointer<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
>
{
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> FieldOrPointerName;
    fn id(&self) -> SelectionTypeId;
    fn type_and_field(&self) -> ObjectTypeAndFieldName;
    fn parent_object_id(&self) -> ServerObjectId;
    // the following are unsupported, for now, because the return values include a generic
    fn reader_selection_set(
        &self,
    ) -> &[WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >];
    fn refetch_strategy(
        &self,
    ) -> Option<
        &RefetchStrategy<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >;
    fn selection_set_for_parent_query(
        &self,
    ) -> &[WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >];

    fn variable_definitions(
        &self,
    ) -> &[WithSpan<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>];
}

impl<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
        TOutputFormat: OutputFormat,
    >
    FieldOrPointer<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData,
    >
    for &ClientField<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData,
        TOutputFormat,
    >
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> FieldOrPointerName {
        SelectionType::Scalar(self.name)
    }

    fn id(&self) -> SelectionTypeId {
        SelectionType::Scalar(self.id)
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_id(&self) -> ServerObjectId {
        self.parent_object_id
    }

    fn reader_selection_set(
        &self,
    ) -> &[WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >] {
        &self.reader_selection_set
    }

    fn refetch_strategy(
        &self,
    ) -> Option<
        &RefetchStrategy<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    > {
        self.refetch_strategy.as_ref()
    }

    fn selection_set_for_parent_query(
        &self,
    ) -> &[WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >] {
        match self.variant {
            ClientFieldVariant::ImperativelyLoadedField(_) => self
                .refetch_strategy
                .as_ref()
                .map(|strategy| strategy.refetch_selection_set())
                .expect(
                    "Expected imperatively loaded field to have refetch selection set. \
                    This is indicative of a bug in Isograph.",
                ),
            _ => &self.reader_selection_set,
        }
    }

    fn variable_definitions(
        &self,
    ) -> &[WithSpan<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>] {
        &self.variable_definitions
    }
}

impl<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
        TOutputFormat: OutputFormat,
    >
    FieldOrPointer<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData,
    >
    for &ClientPointer<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData,
        TOutputFormat,
    >
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> FieldOrPointerName {
        SelectionType::Object(self.name)
    }

    fn id(&self) -> SelectionTypeId {
        SelectionType::Object(self.id)
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_id(&self) -> ServerObjectId {
        self.parent_object_id
    }

    fn reader_selection_set(
        &self,
    ) -> &[WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >] {
        &self.reader_selection_set
    }

    fn refetch_strategy(
        &self,
    ) -> Option<
        &RefetchStrategy<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    > {
        Some(&self.refetch_strategy)
    }

    fn selection_set_for_parent_query(
        &self,
    ) -> &[WithSpan<
        ServerFieldSelection<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >] {
        &self.reader_selection_set
    }

    fn variable_definitions(
        &self,
    ) -> &[WithSpan<VariableDefinition<TClientFieldVariableDefinitionAssociatedData>>] {
        &self.variable_definitions
    }
}
