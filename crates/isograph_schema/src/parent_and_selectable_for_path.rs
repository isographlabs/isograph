use common_lang_types::{DiagnosticResult, Location, SelectableName};
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, EntityNameWrapper, ObjectSelectionPath,
    ScalarSelectionPath, SelectionParentType, SelectionSetParentType, SelectionSetPath,
    SelectionType,
};
use pico::MemoRef;
use prelude::{ErrClone, Postfix};

use crate::{
    ClientScalarSelectable, CompilationProfile, FlattenedDataModelEntity,
    FlattenedDataModelSelectable, IsographDatabase, MemoRefObjectSelectable, MemoRefSelectable,
    entity_not_defined_diagnostic, flattened_entity_named, selectable_is_not_defined_diagnostic,
    selectable_is_wrong_type_diagnostic, selectable_named,
};

type ScalarSelectable<TCompilationProfile> = DefinitionLocation<
    MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>,
    MemoRef<ClientScalarSelectable<TCompilationProfile>>,
>;

// TODO return only selectable. The caller can look up the entity.
pub fn get_parent_and_selectable_for_scalar_path<'a, TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    scalar_path: &ScalarSelectionPath<'a>,
) -> DiagnosticResult<(
    MemoRef<FlattenedDataModelEntity<TCompilationProfile>>,
    ScalarSelectable<TCompilationProfile>,
)> {
    let ScalarSelectionPath { parent, inner } = scalar_path;
    let scalar_selectable_name = inner.name.item;

    let (parent, selectable) = get_parent_and_selectable_for_selection_parent(
        db,
        match parent.reference() {
            SelectionParentType::SelectionSet(position_resolution_path) => position_resolution_path,
        },
        scalar_selectable_name,
    )?;

    let selectable = match selectable {
        DefinitionLocation::Server(server) => {
            let selectable = server.lookup(db);
            let target_entity_name = selectable.target_entity.item.clone_err()?.inner().0;
            let entity = flattened_entity_named(db, target_entity_name)
                .ok_or_else(|| {
                    entity_not_defined_diagnostic(
                        target_entity_name,
                        Location::Generated.note_todo("Use a real location"),
                    )
                })?
                .lookup(db);

            match entity.selection_info {
                SelectionType::Scalar(_) => server
                    .note_todo(
                        "Do not call server.server_defined as \
                        that means we look up twice",
                    )
                    .server_defined()
                    .wrap_ok(),
                SelectionType::Object(_) => ().wrap_err(),
            }
        }
        DefinitionLocation::Client(client) => match client {
            SelectionType::Scalar(scalar) => scalar.client_defined().wrap_ok(),
            SelectionType::Object(_) => ().wrap_err(),
        },
    }
    .map_err(|_| {
        selectable_is_wrong_type_diagnostic(
            parent.lookup(db).name.item,
            scalar_selectable_name,
            "a scalar",
            "an object",
            scalar_path.inner.name.location.to::<Location>(),
        )
    })?;

    Ok((parent, selectable))
}

// TODO return only selectable. The caller can look up the entity.
pub fn get_parent_and_selectable_for_object_path<'a, TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    object_path: &ObjectSelectionPath<'a>,
) -> DiagnosticResult<(
    MemoRef<FlattenedDataModelEntity<TCompilationProfile>>,
    MemoRefObjectSelectable<TCompilationProfile>,
)> {
    let ObjectSelectionPath { parent, inner } = object_path;
    let object_selectable_name = inner.name.item;

    let (parent, selectable) = get_parent_and_selectable_for_selection_parent(
        db,
        match parent.reference() {
            SelectionParentType::SelectionSet(position_resolution_path) => position_resolution_path,
        },
        object_selectable_name,
    )?;

    let selectable = match selectable {
        DefinitionLocation::Server(s) => {
            let selectable = s.lookup(db);
            let target_entity_name = selectable.target_entity.item.clone_err()?.inner().0;
            let entity = flattened_entity_named(db, target_entity_name)
                .ok_or_else(|| {
                    let location = object_path.inner.name.location.to::<Location>();
                    entity_not_defined_diagnostic(target_entity_name, location)
                })?
                .lookup(db);

            if entity.selection_info.as_scalar().is_some() {
                let location = object_path.inner.name.location.to::<Location>();
                return selectable_is_wrong_type_diagnostic(
                    parent.lookup(db).name.item,
                    object_selectable_name,
                    "an object",
                    "a scalar",
                    location,
                )
                .wrap_err();
            }
            s.server_defined()
        }
        DefinitionLocation::Client(c) => match c {
            SelectionType::Scalar(_) => {
                let location = object_path.inner.name.location.to::<Location>();
                return selectable_is_wrong_type_diagnostic(
                    parent.lookup(db).name.item,
                    object_selectable_name,
                    "an object",
                    "a scalar",
                    location,
                )
                .wrap_err();
            }
            SelectionType::Object(o) => o.client_defined(),
        },
    };

    Ok((parent, selectable))
}

// For a selection set, you are not hovering on an individual selection, so it doesn't make sense to
// get a selectable! Just the enclosing object entity.
pub fn get_parent_for_selection_set_path<'a, TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    selection_set_path: &SelectionSetPath<'a>,
) -> DiagnosticResult<MemoRef<FlattenedDataModelEntity<TCompilationProfile>>> {
    let parent_object_entity_name = match selection_set_path.parent.reference() {
        SelectionSetParentType::ObjectSelection(object_selection_path) => {
            let (_parent, selectable) =
                get_parent_and_selectable_for_object_path(db, object_selection_path)?;
            // in pet(id: 123) { /* we are hovering here */ }
            // _parent is Query, selectable is pet. So, we need to get the target of the selectable.

            match selectable {
                DefinitionLocation::Server(s) => {
                    s.lookup(db).target_entity.item.clone_err()?.inner()
                }
                DefinitionLocation::Client(c) => c.lookup(db).target_entity_name.inner(),
            }
        }
        SelectionSetParentType::ClientFieldDeclaration(client_field_declaration_path) => {
            client_field_declaration_path.inner.parent_type.item
        }
        SelectionSetParentType::ClientPointerDeclaration(client_pointer_declaration_path) => {
            client_pointer_declaration_path.inner.parent_type.item
        }
    }
    .0;

    flattened_entity_named(db, parent_object_entity_name).ok_or_else(|| {
        entity_not_defined_diagnostic(
            parent_object_entity_name,
            // TODO we can thread a location here?
            Location::Generated,
        )
    })
}

pub fn get_parent_and_selectable_for_selection_parent<
    'a,
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    selection_set_path: &SelectionSetPath<'a>,
    selectable_name: SelectableName,
) -> DiagnosticResult<(
    MemoRef<FlattenedDataModelEntity<TCompilationProfile>>,
    MemoRefSelectable<TCompilationProfile>,
)> {
    match selection_set_path.parent.reference() {
        SelectionSetParentType::ObjectSelection(object_selection_path) => {
            let (_, object_selectable) =
                get_parent_and_selectable_for_object_path(db, object_selection_path)?;

            let object_parent_entity_name = match object_selectable {
                DefinitionLocation::Server(s) => {
                    s.lookup(db).target_entity.item.clone_err()?.inner()
                }
                DefinitionLocation::Client(c) => c.lookup(db).target_entity_name.inner(),
            };

            parent_object_entity_and_selectable(db, object_parent_entity_name, selectable_name)
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

pub fn parent_object_entity_and_selectable<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityNameWrapper,
    selectable_name: SelectableName,
) -> DiagnosticResult<(
    MemoRef<FlattenedDataModelEntity<TCompilationProfile>>,
    MemoRefSelectable<TCompilationProfile>,
)> {
    let parent_entity = flattened_entity_named(db, parent_server_object_entity_name.0).ok_or(
        entity_not_defined_diagnostic(
            parent_server_object_entity_name.0,
            // TODO we can get a location
            Location::Generated,
        ),
    )?;

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
