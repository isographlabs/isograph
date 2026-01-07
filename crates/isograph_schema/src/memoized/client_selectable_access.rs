use std::collections::{BTreeMap, HashMap, btree_map::Entry};

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, CompilationProfile, IsographDatabase,
    MemoRefClientSelectable, add_client_scalar_selectable_to_entity, get_link_fields_map,
    process_client_pointer_declaration_inner,
};
use common_lang_types::{
    Diagnostic, DiagnosticResult, EntityName, Location, SelectableName, WithEmbeddedLocation,
    WithLocation, WithLocationPostfix, WithNonFatalDiagnostics,
};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    ClientFieldDeclaration, ClientPointerDeclaration, SelectionType, SelectionTypePostfix,
};
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::parse_iso_literal_in_source;

type MemoRefDeclaration =
    SelectionType<MemoRef<ClientFieldDeclaration>, MemoRef<ClientPointerDeclaration>>;

/// client selectables defined by iso literals.
/// Note: this is just the declarations, not the fields!
#[memo]
pub fn client_selectable_declaration_map_from_iso_literals<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
) -> WithNonFatalDiagnostics<
    BTreeMap<(EntityName, SelectableName), WithEmbeddedLocation<MemoRefDeclaration>>,
> {
    let mut out: BTreeMap<(_, SelectableName), _> = BTreeMap::new();
    let mut non_fatal_diagnostics = vec![];

    for (_relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        for extraction in parse_iso_literal_in_source(db, *iso_literals_source_id).to_owned() {
            match extraction {
                Ok((extraction_result, _)) => match extraction_result {
                    IsoLiteralExtractionResult::ClientPointerDeclaration(
                        client_pointer_declaration,
                    ) => {
                        insert_selectable_or_multiple_definition_diagnostic_2(
                            &mut out,
                            (
                                client_pointer_declaration.item.parent_type.item.0,
                                client_pointer_declaration.item.client_pointer_name.item.0,
                            ),
                            client_pointer_declaration
                                .item
                                .interned_value(db)
                                .object_selected()
                                .with_location(client_pointer_declaration.location),
                            &mut non_fatal_diagnostics,
                        );
                    }
                    IsoLiteralExtractionResult::ClientFieldDeclaration(
                        client_field_declaration,
                    ) => {
                        insert_selectable_or_multiple_definition_diagnostic_2(
                            &mut out,
                            (
                                client_field_declaration.item.parent_type.item.0,
                                client_field_declaration.item.client_field_name.item.0,
                            ),
                            client_field_declaration
                                .item
                                .interned_value(db)
                                .scalar_selected()
                                .with_location(client_field_declaration.location),
                            &mut non_fatal_diagnostics,
                        );
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => {
                        // Intentionally ignored. TODO reconsider
                    }
                },

                Err(_) => {
                    // For now, we can only ignore these errors! We don't know a parent entity name
                    // and a selectable name. But. we should restructure this so that we get both,
                    // even if the rest fails to parse.
                }
            }
        }
    }

    WithNonFatalDiagnostics::new(out, non_fatal_diagnostics)
}

#[memo]
pub fn client_selectable_declaration<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_selectable_name: SelectableName,
) -> Option<MemoRefDeclaration> {
    client_selectable_declaration_map_from_iso_literals(db)
        .item
        .get(&(parent_object_entity_name, client_selectable_name))
        .map(|x| x.item)
}

#[memo]
pub fn client_field_declaration<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_scalar_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRef<ClientFieldDeclaration>>> {
    let selectable =
        client_selectable_declaration(db, parent_object_entity_name, client_scalar_selectable_name);

    let item = match selectable {
        Some(item) => item,
        None => return Ok(None),
    };
    match item {
        SelectionType::Scalar(client_field_declaration) => {
            (*client_field_declaration).wrap_some().wrap_ok()
        }
        SelectionType::Object(o) => selectable_is_wrong_type_diagnostic(
            parent_object_entity_name,
            client_scalar_selectable_name,
            "a scalar",
            "an object",
            o.lookup(db).client_pointer_name.location.into(),
        )
        .wrap_err(),
    }
}

#[memo]
pub fn client_pointer_declaration<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_object_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRef<ClientPointerDeclaration>>> {
    let selectable =
        client_selectable_declaration(db, parent_object_entity_name, client_object_selectable_name);

    let item = match selectable {
        Some(item) => item,
        None => return Ok(None),
    };
    match item {
        SelectionType::Object(client_pointer_declaration) => (*client_pointer_declaration)
            .note_todo("Do not clone. Use a MemoRef.")
            .wrap_some()
            .wrap_ok(),
        SelectionType::Scalar(s) => selectable_is_wrong_type_diagnostic(
            parent_object_entity_name,
            client_object_selectable_name,
            "a scalar",
            "an object",
            s.lookup(db).client_field_name.location.into(),
        )
        .wrap_err(),
    }
}

#[memo]
pub fn client_scalar_selectable_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_scalar_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRef<ClientScalarSelectable<TCompilationProfile>>>> {
    let declaration =
        client_field_declaration(db, parent_object_entity_name, client_scalar_selectable_name)
            .clone_err()?;

    let declaration = match declaration {
        Some(declaration) => (*declaration).note_todo("Do not clone. Use a MemoRef."),
        None => {
            // This is an awkward situation! We didn't find any client scalar selectable defined
            // by an iso literal. But, we still need to check for linked fields.
            //
            // What's nice, though, is that we don't actually need the schema to successfully
            // compile if we've already found the field we need! That's neat.
            //
            // We could theoretically skip this if the name is not *LINK_FIELD_NAME /shrug
            //
            // This is also problematic, because we really actually want a "all client fields map" fn,
            // but we don't really have one, since we're adding this here. Oh well. See the awkwardness in
            // selection_set_access.
            let link_fields = get_link_fields_map(db);

            if let Some(link_field) = link_fields
                .get(&(parent_object_entity_name, client_scalar_selectable_name))
                .cloned()
            {
                return link_field.wrap_some().wrap_ok();
            }

            // Awkward! We also need to check for expose fields. Ay ay ay
            return client_selectables_defined_by_network_protocol(db)
                .clone_err()?
                .get(&(parent_object_entity_name, client_scalar_selectable_name))
                .and_then(|x| x.as_scalar())
                .wrap_ok();
        }
    };

    let (_, scalar_selectable) =
        add_client_scalar_selectable_to_entity(db, declaration).clone_err()?;

    scalar_selectable.dereference().wrap_some().wrap_ok()
}

#[memo]
pub fn client_object_selectable_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_object_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRef<ClientObjectSelectable<TCompilationProfile>>>> {
    let declaration =
        client_pointer_declaration(db, parent_object_entity_name, client_object_selectable_name)
            .clone_err()?;

    let declaration = match declaration {
        Some(declaration) => (*declaration).note_todo("Do not clone. Use a MemoRef."),
        None => return Ok(None),
    };

    let (_, object_selectable) =
        process_client_pointer_declaration_inner(db, declaration).clone_err()?;

    object_selectable.dereference().wrap_some().wrap_ok()
}

#[memo]
pub fn client_selectable_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_selectable_name: SelectableName,
) -> DiagnosticResult<Option<MemoRefClientSelectable<TCompilationProfile>>> {
    // we can do this better by reordering functions in this file
    // just in general, we can do better! This is awkward!
    // TODO don't call to_owned, since that clones an error unnecessarily

    let object_selectable = client_object_selectable_named(
        db,
        parent_object_entity_name,
        client_selectable_name.unchecked_conversion(),
    )
    .to_owned();

    let client_selectable = client_scalar_selectable_named(
        db,
        parent_object_entity_name,
        client_selectable_name.unchecked_conversion(),
    )
    .to_owned();

    match (object_selectable, client_selectable) {
        (Ok(Some(_)), Ok(Some(_))) => panic!(
            "Unexpected duplicate. \
            This is indicative of a bug in Isograph."
        ),
        (Ok(object), Ok(scalar)) => {
            // If we received two Ok's, that can only be because the field is not defined.
            //
            // Just kidding! That's true if the field is defined in an iso literal, but for __link
            // or an exposed field, object will be None and scalar might be Some.
            //
            // So it's sufficient to ensure that at least one is None.
            assert!(
                object.is_none() || scalar.is_none(),
                "Expected at least one case to be None. \
                This is indicative of a bug in Isograph."
            );
            object
                .map(SelectionType::Object)
                .or(scalar.map(SelectionType::Scalar))
                .wrap_ok()
        }
        (Ok(object_selectable), Err(_)) => object_selectable.map(SelectionType::Object).wrap_ok(),
        (Err(_), Ok(scalar_selectable)) => scalar_selectable.map(SelectionType::Scalar).wrap_ok(),
        (Err(e), Err(_)) => e.wrap_err(),
    }
}

#[memo]
// TODO this function seems quite useless!
pub fn client_selectables_defined_by_network_protocol<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<
    BTreeMap<(EntityName, SelectableName), MemoRefClientSelectable<TCompilationProfile>>,
> {
    let outcome = TCompilationProfile::deprecated_parse_type_system_documents(db).clone_err()?;
    let expose_as_field_queue = &outcome.0.item.selectables;

    expose_as_field_queue
        .iter()
        .map(|(key, value)| (key.dereference(), value.item))
        .collect::<BTreeMap<_, _>>()
        .note_todo("Don't clone")
        .wrap_ok()
}

// TODO use this as a source for the other functions, especially for
// client_scalar_selectable_named
#[expect(clippy::type_complexity)]
#[memo]
pub fn client_selectable_map<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<
    HashMap<
        (EntityName, SelectableName),
        DiagnosticResult<MemoRefClientSelectable<TCompilationProfile>>,
    >,
> {
    let iso_literal_map = client_selectable_declaration_map_from_iso_literals(db);

    iso_literal_map
        .item
        .iter()
        .map(
            |((parent_object_entity_name, client_selectable_name), value)| {
                (
                    (*parent_object_entity_name, *client_selectable_name),
                    (|| {
                        match value.item {
                            SelectionType::Scalar(scalar_declaration) => {
                                add_client_scalar_selectable_to_entity(db, scalar_declaration)
                                    .clone()
                                    .map(|(_, selectable)| selectable)?
                                    .scalar_selected()
                            }
                            SelectionType::Object(object_declaration) => {
                                process_client_pointer_declaration_inner(db, object_declaration)
                                    .clone()
                                    .map(|(_, selectable)| selectable)?
                                    .object_selected()
                            }
                        }
                        .wrap_ok()
                    })(),
                )
            },
        )
        .chain(
            get_link_fields_map(db)
                .clone()
                .into_iter()
                .map(|(key, value)| ((key.0, key.1), value.scalar_selected().wrap_ok())),
        )
        .chain(
            client_selectables_defined_by_network_protocol(db)
                .to_owned()?
                .into_iter()
                .map(|(key, value)| (key, value.wrap_ok())),
        )
        .collect::<HashMap<_, _>>()
        .wrap_ok()
}

pub fn multiple_selectable_definitions_found_diagnostic(
    parent_object_entity_name: EntityName,
    selectable_name: SelectableName,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "Multiple definitions of `{parent_object_entity_name}.{selectable_name}` were found"
        ),
        location,
    )
}

pub fn selectable_is_wrong_type_diagnostic(
    parent_object_entity_name: EntityName,
    client_selectable_name: SelectableName,
    intended_type: &'static str,
    actual_type: &'static str,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "Expected `{parent_object_entity_name}.{client_selectable_name}` to \
            be {intended_type}. But it was {actual_type}."
        ),
        location.wrap_some(),
    )
    .note_todo("Rewrite this to account for the new location (the selection location)")
}

pub fn selectable_is_not_defined_diagnostic(
    parent_object_entity_name: EntityName,
    client_selectable_name: SelectableName,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!("`{parent_object_entity_name}.{client_selectable_name}` is not defined."),
        location.wrap_some(),
    )
}

pub fn entity_not_defined_diagnostic(entity_name: EntityName, location: Location) -> Diagnostic {
    Diagnostic::new(
        format!("`{entity_name}` is not defined."),
        location.wrap_some(),
    )
}

// TODO make this generic over value, too
pub fn insert_selectable_or_multiple_definition_diagnostic<Value>(
    map: &mut BTreeMap<(EntityName, SelectableName), WithLocation<Value>>,
    key: (EntityName, SelectableName),
    item: WithLocation<Value>,
    non_fatal_diagnostics: &mut Vec<Diagnostic>,
) {
    match map.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(item);
        }
        Entry::Occupied(_) => {
            non_fatal_diagnostics.push(multiple_selectable_definitions_found_diagnostic(
                key.0,
                key.1,
                item.location.wrap_some(),
            ))
        }
    }
}

pub fn insert_selectable_or_multiple_definition_diagnostic_2<Value>(
    map: &mut BTreeMap<(EntityName, SelectableName), WithEmbeddedLocation<Value>>,
    key: (EntityName, SelectableName),
    item: WithEmbeddedLocation<Value>,
    non_fatal_diagnostics: &mut Vec<Diagnostic>,
) {
    match map.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(item);
        }
        Entry::Occupied(_) => {
            non_fatal_diagnostics.push(multiple_selectable_definitions_found_diagnostic(
                key.0,
                key.1,
                item.location.to::<Location>().wrap_some(),
            ))
        }
    }
}
