use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName, DiagnosticResult,
    DiagnosticVecResult, ParentObjectEntityNameAndSelectableName, SelectableName,
    ServerObjectEntityName, WithLocation, WithSpan,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{Description, SelectionSet, VariableDefinition};

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, IsographDatabase,
    NetworkProtocol, ObjectSelectableId, ScalarSelectableId, SelectableTrait, ServerEntityName,
    client_scalar_selectable_named, selectable_validated_reader_selection_set,
    validated_refetch_strategy_for_client_scalar_selectable_named,
};

#[impl_for_selection_type]
pub trait ClientScalarOrObjectSelectable {
    fn description(&self) -> Option<Description>;
    fn name(&self) -> ClientSelectableName;
    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName;
    fn parent_object_entity_name(&self) -> ServerObjectEntityName;

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
        self.name.item.into()
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        self.type_and_field
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityName>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "field"
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.item.into()
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        self.type_and_field
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
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
        self.name.item.into()
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        self.type_and_field
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityName>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "pointer"
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<Description> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.item.into()
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        self.type_and_field
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
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

    fn name(&self) -> WithLocation<SelectableName> {
        self.name.map(|x| x.into())
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

    fn name(&self) -> WithLocation<SelectableName> {
        self.name.map(|x| x.into())
    }

    fn parent_object_entity_name(&self) -> ServerObjectEntityName {
        self.parent_object_entity_name
    }

    fn arguments(&self) -> Vec<&VariableDefinition<ServerEntityName>> {
        self.variable_definitions.iter().map(|x| &x.item).collect()
    }
}

pub fn client_scalar_selectable_selection_set_for_parent_query<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_scalar_selectable_name: ClientScalarSelectableName,
) -> DiagnosticResult<WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>> {
    let selectable = client_scalar_selectable_named(
        db,
        parent_object_entity_name,
        client_scalar_selectable_name,
    )
    .as_ref()
    .map_err(|e| e.clone())?
    .as_ref()
    .expect(
        "Expected selectable to exist. \
        This is indicative of a bug in Isograph.",
    );

    Ok(match selectable.variant {
        ClientFieldVariant::ImperativelyLoadedField(_) => {
            let refetch_strategy = validated_refetch_strategy_for_client_scalar_selectable_named(
                db,
                parent_object_entity_name,
                client_scalar_selectable_name,
            )
            .as_ref()
            .expect(
                "Expected imperatively loaded field to have refetch selection set. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected imperatively loaded field to have refetch selection set. \
                This is indicative of a bug in Isograph.",
            );

            refetch_strategy
                .refetch_selection_set()
                .expect(
                    "Expected imperatively loaded field to have refetch selection set. \
                This is indicative of a bug in Isograph.",
                )
                // TODO don't clone
                .clone()
        }
        _ => {
            // TODO don't clone
            selectable_validated_reader_selection_set(
                db,
                parent_object_entity_name,
                client_scalar_selectable_name.into(),
            )
            .as_ref()
            .expect("Expected selection set to be valid.")
            .clone()
        }
    })
}

pub fn client_object_selectable_selection_set_for_parent_query<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_object_selectable_name: ClientObjectSelectableName,
) -> DiagnosticVecResult<WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>> {
    selectable_validated_reader_selection_set(
        db,
        parent_object_entity_name,
        client_object_selectable_name.into(),
    )
    .clone()
}
