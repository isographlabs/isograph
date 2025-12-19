use common_lang_types::{
    DiagnosticResult, EntityName, ParentObjectEntityNameAndSelectableName, SelectableName,
    WithEmbeddedLocation,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{SelectionSet, VariableDefinition};

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, IsographDatabase,
    NetworkProtocol, ServerEntityName, client_scalar_selectable_named,
    refetch_strategy_for_client_scalar_selectable_named, selectable_reader_selection_set,
};

#[impl_for_selection_type]
pub trait ClientScalarOrObjectSelectable {
    fn name(&self) -> SelectableName;
    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName;
    fn parent_object_entity_name(&self) -> EntityName;

    fn variable_definitions(&self) -> &[VariableDefinition<ServerEntityName>];
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientScalarSelectable<TNetworkProtocol>
{
    fn name(&self) -> SelectableName {
        self.name
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        ParentObjectEntityNameAndSelectableName {
            parent_entity_name: self.parent_entity_name,
            selectable_name: self.name,
        }
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_entity_name
    }

    fn variable_definitions(&self) -> &[VariableDefinition<ServerEntityName>] {
        &self.variable_definitions
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for ClientScalarSelectable<TNetworkProtocol>
{
    fn name(&self) -> SelectableName {
        self.name
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        ParentObjectEntityNameAndSelectableName {
            parent_entity_name: self.parent_entity_name,
            selectable_name: self.name,
        }
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_entity_name
    }

    fn variable_definitions(&self) -> &[VariableDefinition<ServerEntityName>] {
        &self.variable_definitions
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn name(&self) -> SelectableName {
        self.name
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        ParentObjectEntityNameAndSelectableName {
            parent_entity_name: self.parent_entity_name,
            selectable_name: self.name,
        }
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_entity_name
    }

    fn variable_definitions(&self) -> &[VariableDefinition<ServerEntityName>] {
        &self.variable_definitions
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for ClientObjectSelectable<TNetworkProtocol>
{
    fn name(&self) -> SelectableName {
        self.name
    }

    fn type_and_field(&self) -> ParentObjectEntityNameAndSelectableName {
        ParentObjectEntityNameAndSelectableName {
            parent_entity_name: self.parent_entity_name,
            selectable_name: self.name,
        }
    }

    fn parent_object_entity_name(&self) -> EntityName {
        self.parent_entity_name
    }

    fn variable_definitions(&self) -> &[VariableDefinition<ServerEntityName>] {
        &self.variable_definitions
    }
}

pub fn client_scalar_selectable_selection_set_for_parent_query<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: EntityName,
    client_scalar_selectable_name: SelectableName,
) -> DiagnosticResult<WithEmbeddedLocation<SelectionSet>> {
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
    )
    .lookup(db);

    Ok(match selectable.variant {
        ClientFieldVariant::ImperativelyLoadedField(_) => {
            let refetch_strategy = refetch_strategy_for_client_scalar_selectable_named(
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
            selectable_reader_selection_set(
                db,
                parent_object_entity_name,
                client_scalar_selectable_name,
            )
            .as_ref()
            .expect("Expected selection set to be valid.")
            .lookup(db)
            .clone()
        }
    })
}
