use common_lang_types::{DiagnosticResult, EntityName, Location, SelectableName, WithLocation};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, Description, EntityNameWrapper,
    ObjectSelectionPath, ScalarSelectionPath, SelectionParentType, SelectionSetParentType,
    SelectionSetPath, SelectionType, SelectionTypePostfix,
};
use pico::MemoRef;
use prelude::Postfix;

use crate::{
    IsographDatabase, MemoRefObjectSelectable, MemoRefSelectable, NetworkProtocol,
    ScalarSelectable, ServerObjectEntity, ServerScalarEntity, entity_not_defined_diagnostic,
    selectable_is_not_defined_diagnostic, selectable_is_wrong_type_diagnostic, selectable_named,
    server_object_entity_named,
};

#[impl_for_selection_type]
pub trait ServerScalarOrObjectEntity {
    fn name(&self) -> SelectionType<EntityName, EntityName>;
    fn description(&self) -> Option<Description>;
}

impl<T: ServerScalarOrObjectEntity> ServerScalarOrObjectEntity for WithLocation<T> {
    fn name(&self) -> SelectionType<EntityName, EntityName> {
        self.item.name()
    }

    fn description(&self) -> Option<Description> {
        self.item.description()
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for ServerScalarEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<EntityName, EntityName> {
        self.name.scalar_selected()
    }

    fn description(&self) -> Option<Description> {
        self.description
    }
}

impl<TNetworkProtocol: NetworkProtocol> ServerScalarOrObjectEntity
    for ServerObjectEntity<TNetworkProtocol>
{
    fn name(&self) -> SelectionType<EntityName, EntityName> {
        self.name.object_selected()
    }

    fn description(&self) -> Option<Description> {
        self.description
    }
}

// TODO return only selectable. The caller can look up the entity.
pub fn get_parent_and_selectable_for_scalar_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    scalar_path: &ScalarSelectionPath<'a>,
) -> DiagnosticResult<(
    MemoRef<ServerObjectEntity<TNetworkProtocol>>,
    ScalarSelectable<TNetworkProtocol>,
)> {
    let ScalarSelectionPath { parent, inner } = scalar_path;
    let scalar_selectable_name = inner.name.item;

    let (parent, selectable) = get_parent_and_selectable_for_selection_parent(
        db,
        match &parent {
            SelectionParentType::SelectionSet(position_resolution_path) => position_resolution_path,
        },
        scalar_selectable_name,
    )?;

    let selectable = match selectable {
        DefinitionLocation::Server(server) => match server {
            SelectionType::Scalar(scalar) => scalar.server_defined().wrap_ok(),
            SelectionType::Object(_) => ().wrap_err(),
        },
        DefinitionLocation::Client(client) => match client {
            SelectionType::Scalar(scalar) => scalar.client_defined().wrap_ok(),
            SelectionType::Object(_) => ().wrap_err(),
        },
    }
    .map_err(|_| {
        selectable_is_wrong_type_diagnostic(
            parent.lookup(db).name,
            scalar_selectable_name,
            "a scalar",
            "an object",
            scalar_path.inner.name.location.to::<Location>(),
        )
    })?;

    Ok((parent, selectable))
}

// TODO return only selectable. The caller can look up the entity.
pub fn get_parent_and_selectable_for_object_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    object_path: &ObjectSelectionPath<'a>,
) -> DiagnosticResult<(
    MemoRef<ServerObjectEntity<TNetworkProtocol>>,
    MemoRefObjectSelectable<TNetworkProtocol>,
)> {
    let ObjectSelectionPath { parent, inner } = object_path;
    let object_selectable_name = inner.name.item;

    let (parent, selectable) = get_parent_and_selectable_for_selection_parent(
        db,
        match &parent {
            SelectionParentType::SelectionSet(position_resolution_path) => position_resolution_path,
        },
        object_selectable_name,
    )?;

    let selectable = selectable.as_object().ok_or_else(|| {
        let location = object_path.inner.name.location.to::<Location>();
        selectable_is_wrong_type_diagnostic(
            parent.lookup(db).name,
            object_selectable_name,
            "an object",
            "a scalar",
            location,
        )
    })?;

    Ok((parent, selectable))
}

// For a selection set, you are not hovering on an individual selection, so it doesn't make sense to
// get a selectable! Just the enclosing object entity.
pub fn get_parent_for_selection_set_path<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_set_path: &SelectionSetPath<'a>,
) -> DiagnosticResult<MemoRef<ServerObjectEntity<TNetworkProtocol>>> {
    let parent_object_entity_name = match &selection_set_path.parent {
        SelectionSetParentType::ObjectSelection(object_selection_path) => {
            let (_parent, selectable) =
                get_parent_and_selectable_for_object_path(db, object_selection_path)?;
            // in pet(id: 123) { /* we are hovering here */ }
            // _parent is Query, selectable is pet. So, we need to get the target of the selectable.

            match selectable {
                DefinitionLocation::Server(s) => {
                    s.lookup(db).target_object_entity.inner().dereference()
                }
                DefinitionLocation::Client(c) => {
                    c.lookup(db).target_object_entity_name.inner().dereference()
                }
            }
        }
        SelectionSetParentType::ClientFieldDeclaration(client_field_declaration_path) => {
            client_field_declaration_path.inner.parent_type.item.0
        }
        SelectionSetParentType::ClientPointerDeclaration(client_pointer_declaration_path) => {
            client_pointer_declaration_path.inner.parent_type.item.0
        }
    };

    server_object_entity_named(db, parent_object_entity_name)
        .clone()?
        .ok_or_else(|| {
            entity_not_defined_diagnostic(
                parent_object_entity_name,
                // TODO we can thread a location here?
                Location::Generated,
            )
        })
}

pub fn get_parent_and_selectable_for_selection_parent<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_set_path: &SelectionSetPath<'a>,
    selectable_name: SelectableName,
) -> DiagnosticResult<(
    MemoRef<ServerObjectEntity<TNetworkProtocol>>,
    MemoRefSelectable<TNetworkProtocol>,
)> {
    match &selection_set_path.parent {
        SelectionSetParentType::ObjectSelection(object_selection_path) => {
            let (_, object_selectable) =
                get_parent_and_selectable_for_object_path(db, object_selection_path)?;

            let object_parent_entity_name = match object_selectable {
                DefinitionLocation::Server(s) => {
                    s.lookup(db).target_object_entity.inner().dereference()
                }
                DefinitionLocation::Client(c) => {
                    c.lookup(db).target_object_entity_name.inner().dereference()
                }
            };

            parent_object_entity_and_selectable(
                db,
                object_parent_entity_name.into(),
                selectable_name,
            )
        }
        SelectionSetParentType::ClientFieldDeclaration(client_field_declaration_path) => {
            let parent_type_name = client_field_declaration_path.inner.parent_type.item;

            parent_object_entity_and_selectable(db, parent_type_name, selectable_name)
        }
        SelectionSetParentType::ClientPointerDeclaration(client_pointer_declaration_path) => {
            let parent_type_name = client_pointer_declaration_path.inner.parent_type.item;

            parent_object_entity_and_selectable(db, parent_type_name, selectable_name)
        }
    }
}

pub fn parent_object_entity_and_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: EntityNameWrapper,
    selectable_name: SelectableName,
) -> DiagnosticResult<(
    MemoRef<ServerObjectEntity<TNetworkProtocol>>,
    MemoRefSelectable<TNetworkProtocol>,
)> {
    let parent_entity = server_object_entity_named(db, parent_server_object_entity_name.0)
        .clone()?
        .ok_or(entity_not_defined_diagnostic(
            parent_server_object_entity_name.0,
            // TODO we can get a location
            Location::Generated,
        ))?;

    match selectable_named(db, parent_server_object_entity_name.0, selectable_name).to_owned()? {
        Some(selectable) => Ok((parent_entity, selectable)),
        None => selectable_is_not_defined_diagnostic(
            parent_server_object_entity_name.0,
            selectable_name,
            // TODO we can get a location
            Location::Generated,
        )
        .wrap_err(),
    }
}
