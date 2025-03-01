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
    // TODO model this so that reader_selection_sets are required for
    // non-imperative client fields. (Are imperatively loaded fields
    // true client fields? Probably not!)
    pub reader_selection_set: Option<
        Vec<
            WithSpan<
                ServerFieldSelection<
                    TSelectionTypeSelectionScalarFieldAssociatedData,
                    TSelectionTypeSelectionLinkedFieldAssociatedData,
                >,
            >,
        >,
    >,

    // None -> not refetchable
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
pub trait FieldOrPointer {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> FieldOrPointerName;
    fn id(&self) -> SelectionTypeId;
    fn type_and_field(&self) -> ObjectTypeAndFieldName;
    fn parent_object_id(&self) -> ServerObjectId;
    // the following are unsupported, for now, because the return values include a generic
    // fn reader_selection_set
    // fn refetch_strategy
    // fn variable_definitions
}

impl<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
        TOutputFormat: OutputFormat,
    > FieldOrPointer
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
}

impl<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
        TClientFieldVariableDefinitionAssociatedData: Ord + Debug,
        TOutputFormat: OutputFormat,
    > FieldOrPointer
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
}
