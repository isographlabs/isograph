use common_lang_types::{ClientSelectableName, DescriptionValue, ObjectTypeAndFieldName, WithSpan};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{ServerEntityId, ServerObjectEntityId, VariableDefinition};

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, NetworkProtocol,
    ObjectSelectableId, RefetchStrategy, ScalarSelectableId, ValidatedSelection,
};

#[impl_for_selection_type]
pub trait ClientScalarOrObjectSelectable {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> ClientSelectableName;
    fn type_and_field(&self) -> ObjectTypeAndFieldName;
    fn parent_object_entity_id(&self) -> ServerObjectEntityId;
    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>];
    fn refetch_strategy(&self) -> Option<&RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>;
    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>];

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>];

    fn client_type(&self) -> &'static str;
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_entity_id(&self) -> ServerObjectEntityId {
        self.parent_object_entity_id
    }

    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn refetch_strategy(&self) -> Option<&RefetchStrategy<ScalarSelectableId, ObjectSelectableId>> {
        self.refetch_strategy.as_ref()
    }

    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>] {
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

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "field"
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_entity_id(&self) -> ServerObjectEntityId {
        self.parent_object_entity_id
    }

    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn refetch_strategy(&self) -> Option<&RefetchStrategy<ScalarSelectableId, ObjectSelectableId>> {
        Some(&self.refetch_strategy)
    }

    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "pointer"
    }
}
