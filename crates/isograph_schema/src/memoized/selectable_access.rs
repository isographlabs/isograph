use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location, SelectableName};
use isograph_lang_types::{DefinitionLocationPostfix, SelectionType};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    CompilationProfile, IsographDatabase, MemoRefSelectable, client_selectable_declaration,
    client_selectable_map, client_selectable_named, deprecated_server_selectables_map_for_entity,
    flattened_selectables, multiple_selectable_definitions_found_diagnostic,
    server_selectable_named,
};

#[memo]
pub fn selectable_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRefSelectable<TCompilationProfile>>> {
    // we don't obviously have a better way to do this besides checking whether this
    // a server selectable and also checking whether it is a client selectable, and
    // error'ing if we have multiple definitions.
    let server_selectable = server_selectable_named(
        db,
        parent_server_object_entity_name,
        selectable_name.unchecked_conversion(),
    )
    .as_ref();

    let client_selectable = client_selectable_named(
        db,
        parent_server_object_entity_name,
        selectable_name.unchecked_conversion(),
    )
    .as_ref();

    // case 1: both are error -> return error
    // case 2: one is error -> assume that is an unrelated error and return the other one
    // case 3: both are ok -> check that there aren't duplicate definitions, and return remaining one or None

    // TODO wrap_ok here is silly! We should figure out how to rewrite this fn.
    match (server_selectable.wrap_ok::<Diagnostic>(), client_selectable) {
        (Err(e), Err(_)) => e.clone().wrap_err(),
        (Ok(server), Err(_)) => match server {
            Some(server_selectable) => (*server_selectable).server_defined().wrap_some().wrap_ok(),
            None => Ok(None),
        },
        (Err(_), Ok(client)) => match *client {
            Some(client_selectable) => client_selectable.client_defined().wrap_some().wrap_ok(),
            None => Ok(None),
        },
        (Ok(server), Ok(client)) => match (server, client) {
            (None, None) => Ok(None),
            (None, Some(client_selectable)) => {
                (*client_selectable).client_defined().wrap_some().wrap_ok()
            }
            (Some(server_selectable), None) => {
                (*server_selectable).server_defined().wrap_some().wrap_ok()
            }
            (Some(_), Some(_)) => multiple_selectable_definitions_found_diagnostic(
                parent_server_object_entity_name,
                selectable_name,
                None.note_todo("Get a real location"),
            )
            .wrap_err(),
        },
    }
}

#[memo]
pub fn selectables_for_entity<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityName,
) -> DiagnosticResult<Vec<DiagnosticResult<MemoRefSelectable<TCompilationProfile>>>> {
    let mut selectables =
        deprecated_server_selectables_map_for_entity(db, parent_server_object_entity_name)
            .values()
            .map(|value| value.dereference().server_defined().wrap_ok())
            .collect::<Vec<_>>();

    selectables.extend(
        client_selectable_map(db)
            .clone_err()?
            .iter()
            .filter(|((entity_name, _selectable_name), _value)| {
                *entity_name == parent_server_object_entity_name
            })
            .map(|(_key, value)| {
                let value = value.clone()?;
                value.client_defined().wrap_ok()
            }),
    );

    selectables.wrap_ok()
}

#[memo]
pub fn selectable_definition_location<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Option<Location>> {
    let (outcome, _) =
        TCompilationProfile::deprecated_parse_type_system_documents(db).clone_err()?;

    if let Some(x) = outcome
        .item
        .selectables
        .get(&(entity_name, selectable_name))
        .map(|x| x.location)
    {
        return x.wrap_some().wrap_ok();
    }

    let selectable = client_selectable_declaration(db, entity_name, selectable_name);

    selectable
        .map(|x| match x {
            SelectionType::Scalar(s) => s.lookup(db).client_field_name.location.into(),
            SelectionType::Object(o) => o.lookup(db).client_pointer_name.location.into(),
        })
        .wrap_ok()
}

#[memo]
pub fn selectables<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<Vec<MemoRefSelectable<TCompilationProfile>>> {
    let mut selectables = flattened_selectables(db)
        .iter()
        .map(|(_, _, value)| value.dereference().server_defined())
        .collect::<Vec<_>>();

    selectables.extend(
        client_selectable_map(db)
            .clone_err()?
            .iter()
            .flat_map(|(_key, value)| {
                let value = value.clone().ok()?;
                value.client_defined().wrap_some()
            }),
    );

    selectables.wrap_ok()
}
