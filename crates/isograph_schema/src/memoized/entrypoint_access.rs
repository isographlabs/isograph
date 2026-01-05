use std::collections::{HashMap, hash_map::Entry};

use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location, SelectableName};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    DefinitionLocation, EntrypointDeclaration, SelectionType, from_isograph_field_directives,
};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    CompilationProfile, EntrypointDeclarationInfo, IsographDatabase,
    client_scalar_selectable_named, parse_iso_literal_in_source,
    selectable_is_not_defined_diagnostic, selectable_is_wrong_type_diagnostic, selectable_named,
};

#[memo]
pub fn entrypoint_declarations<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<EntrypointDeclaration> {
    let mut out = vec![];
    for (_relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        for result in parse_iso_literal_in_source(db, *iso_literals_source_id) {
            if let Ok((IsoLiteralExtractionResult::EntrypointDeclaration(e), _)) = result {
                out.push(e.item.clone().note_todo("Do not clone. Use a MemoRef."));
            }
            // intentionally ignore non-entrypoints
        }
    }

    out
}

#[memo]
pub fn validated_entrypoints<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> HashMap<(EntityName, SelectableName), DiagnosticResult<EntrypointDeclarationInfo>> {
    let entrypoints = entrypoint_declarations(db);

    let mut out: HashMap<_, Result<EntrypointDeclarationInfo, _>> = HashMap::new();

    // To validate an entrypoint, we confirm that its parent type exists and the client field is defined,
    // which we can validate by ensuring that the client scalar selectable exists.
    //
    // We also validate that it is a fetchable type.
    for entrypoint_declaration_info in entrypoints {
        let value = (|| {
            client_scalar_selectable_named(
                db,
                entrypoint_declaration_info.parent_type.item.0,
                entrypoint_declaration_info.client_field_name.item.0,
            )
            .clone_err()?
            .as_ref()
            .ok_or_else(|| {
                // check if it has a different type
                let selectable = selectable_named(
                    db,
                    entrypoint_declaration_info.parent_type.item.0,
                    entrypoint_declaration_info.client_field_name.item.0,
                );

                if let Ok(Some(selectable)) = selectable {
                    let actual_type = match selectable {
                        DefinitionLocation::Server(s) => {
                            match s.lookup(db).is_inline_fragment.reference() {
                                SelectionType::Scalar(_) => "a server scalar",
                                SelectionType::Object(_) => "a server object",
                            }
                        }
                        DefinitionLocation::Client(SelectionType::Scalar(_)) => {
                            panic!("Unexpected client scalar")
                        }
                        DefinitionLocation::Client(SelectionType::Object(_)) => "a client pointer",
                    };

                    return selectable_is_wrong_type_diagnostic(
                        entrypoint_declaration_info.parent_type.item.0,
                        entrypoint_declaration_info.client_field_name.item.0,
                        "client field",
                        actual_type,
                        entrypoint_declaration_info
                            .client_field_name
                            .location
                            .into(),
                    );
                }

                // if not
                selectable_is_not_defined_diagnostic(
                    entrypoint_declaration_info.parent_type.item.0,
                    entrypoint_declaration_info.client_field_name.item.0,
                    entrypoint_declaration_info
                        .client_field_name
                        .location
                        .into(),
                )
            })?;

            Ok(EntrypointDeclarationInfo {
                iso_literal_text: entrypoint_declaration_info.iso_literal_text,
                directive_set: from_isograph_field_directives(
                    entrypoint_declaration_info.directive_set.reference(),
                )?,
            })
        })();

        let key = (
            entrypoint_declaration_info.parent_type.item.0,
            entrypoint_declaration_info.client_field_name.item.0,
        );

        match out.entry(key) {
            Entry::Occupied(mut occupied_entry) => {
                let existing_result = occupied_entry.get_mut();

                // TODO we shouldn't validate this here. We can still productively use the entrypoint, even without
                // a consistent entrypoint set. But that is a theoretical problem, as of right now, it makes no
                // difference.
                // confirm that they have the same directives
                if let (Ok(existing_entrypoint), Ok(new_entrypoint)) =
                    (existing_result.as_ref(), value.as_ref())
                    && existing_entrypoint.directive_set != new_entrypoint.directive_set
                {
                    *existing_result = Diagnostic::new(
                        "Entrypoint declared lazy in one location and \
                            declared eager in another location. Entrypoint \
                            must be either lazy or non-lazy in all instances."
                            .to_string(),
                        entrypoint_declaration_info
                            .client_field_name
                            .location
                            .to::<Location>()
                            .wrap_some(),
                    )
                    .wrap_err()
                }
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(value);
            }
        }
    }

    out
}
