use common_lang_types::IsographObjectTypeName;
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, DefinitionLocation, LinkedFieldSelection,
    ObjectSelectionDirectiveSet, ScalarFieldSelection, ScalarSelectionDirectiveSet, SelectionType,
    SelectionTypeContainingSelections, ServerEntityId, ServerObjectId, ServerScalarSelectableId,
    VariableDefinition,
};

use crate::{ClientField, ClientPointer, UseRefetchFieldRefetchStrategy};

pub type ValidatedSelection = SelectionTypeContainingSelections<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;

pub type ValidatedLinkedFieldSelection = LinkedFieldSelection<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;
pub type ValidatedScalarFieldSelection =
    ScalarFieldSelection<ValidatedScalarSelectionAssociatedData>;

pub type ValidatedVariableDefinition = VariableDefinition<ServerEntityId>;

pub type ValidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    ValidatedScalarSelectionAssociatedData,
    ValidatedLinkedFieldAssociatedData,
>;

/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedFieldDefinitionLocation =
    DefinitionLocation<ServerScalarSelectableId, ClientFieldId>;

#[derive(Debug, Clone)]
pub struct ValidatedLinkedFieldAssociatedData {
    pub parent_object_id: ServerObjectId,
    pub field_id: DefinitionLocation<ServerScalarSelectableId, ClientPointerId>,
    pub selection_variant: ObjectSelectionDirectiveSet,
    /// Some if the (destination?) object is concrete; None otherwise.
    pub concrete_type: Option<IsographObjectTypeName>,
}

// TODO this should encode whether the scalar selection points to a
// client field or to a server scalar
#[derive(Debug, Clone)]
pub struct ValidatedScalarSelectionAssociatedData {
    pub location: ValidatedFieldDefinitionLocation,
    pub selection_variant: ScalarSelectionDirectiveSet,
}

pub type ValidatedSelectionType<'a, TOutputFormat> =
    SelectionType<&'a ClientField<TOutputFormat>, &'a ClientPointer<TOutputFormat>>;
