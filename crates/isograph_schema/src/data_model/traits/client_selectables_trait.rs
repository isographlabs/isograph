use common_lang_types::{
    ClientSelectableName, ParentObjectEntityNameAndSelectableName, SelectableName,
    ServerObjectEntityName, WithSpan,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{Description, VariableDefinition};

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, NetworkProtocol,
    ObjectSelectableId, RefetchStrategy, ScalarSelectableId, SelectableTrait, ServerEntityName,
    ValidatedSelection,
};

#[impl_for_selection_type]
pub trait ClientScalarOrObjectSelectable {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> ClientSelectableName;
    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;
    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>];
    fn refetch_strategy(&self) -> Option<&RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>;
    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>];

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityName>>];

    fn client_type(&self) -> &'static str;
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        self.type_and_field
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
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

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityName>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "field"
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        self.type_and_field
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
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

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityName>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "pointer"
    }
}

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>> {
        self.variable_definitions.iter().map(|x| &x.item).collect()
    }
}

impl<TNetworkProtocol: NetworkProtocol> SelectableTrait
    for ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> SelectableName {
        self.name.into()
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>> {
        self.variable_definitions.iter().map(|x| &x.item).collect()
    }
}
