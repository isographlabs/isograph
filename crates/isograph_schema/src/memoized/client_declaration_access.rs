use std::collections::{BTreeMap, btree_map::Entry};

use crate::{
    CompilationProfile, IsographDatabase, MemoRefDeclaration,
    multiple_selectable_definitions_found_diagnostic, selectable_is_wrong_type_diagnostic,
};
use common_lang_types::{
    Diagnostic, DiagnosticResult, EntityName, Location, SelectableName, WithEmbeddedLocation,
    WithLocationPostfix, WithNonFatalDiagnostics,
};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    ClientFieldDeclaration, ClientPointerDeclaration, SelectionType, SelectionTypePostfix,
};
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

use crate::parse_iso_literal_in_source;

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

fn insert_selectable_or_multiple_definition_diagnostic_2<Value>(
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
