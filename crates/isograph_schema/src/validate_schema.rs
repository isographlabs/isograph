use common_lang_types::IsographObjectTypeName;
use isograph_lang_types::{
    ClientFieldId, ClientPointerId, DefinitionLocation, LinkedFieldSelection,
    LoadableDirectiveParameters, ObjectSelectionDirectiveSet, ScalarFieldSelection,
    ScalarSelectionDirectiveSet, SelectionType, SelectionTypeContainingSelections, ServerEntityId,
    ServerObjectId, ServerScalarSelectableId, VariableDefinition,
};

use crate::{
    ClientField, ClientFieldVariant, ClientPointer, ImperativelyLoadedFieldVariant, OutputFormat,
    UseRefetchFieldRefetchStrategy,
};

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

pub enum Loadability<'a> {
    LoadablySelectedField(&'a LoadableDirectiveParameters),
    ImperativelyLoadedField(&'a ImperativelyLoadedFieldVariant),
}

/// Why do we do this? Because how we handle a field is determined by both the
/// the field defition (e.g. exposed fields can only be fetched imperatively)
/// and the selection (i.e. we can also take non-imperative fields and make them
/// imperative.)
///
/// The eventual plan is to clean this model up. Instead, imperative fields will
/// need to be explicitly selected loadably. If they are not, they will be fetched
/// as an immediate follow-up request. Once we do this, there will always be one
/// source of truth for whether a field is fetched imperatively: the presence of the
/// @loadable directive.
pub fn categorize_field_loadability<'a, TOutputFormat: OutputFormat>(
    client_field: &'a ClientField<TOutputFormat>,
    selection_variant: &'a ScalarSelectionDirectiveSet,
) -> Option<Loadability<'a>> {
    match &client_field.variant {
        ClientFieldVariant::Link => None,
        ClientFieldVariant::UserWritten(_) => match selection_variant {
            ScalarSelectionDirectiveSet::None(_) => None,
            ScalarSelectionDirectiveSet::Updatable(_) => None,
            ScalarSelectionDirectiveSet::Loadable(l) => {
                Some(Loadability::LoadablySelectedField(&l.loadable))
            }
        },
        ClientFieldVariant::ImperativelyLoadedField(i) => {
            Some(Loadability::ImperativelyLoadedField(i))
        }
    }
}
