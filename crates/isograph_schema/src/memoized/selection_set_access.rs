use std::collections::HashMap;

use common_lang_types::{
    ClientSelectableName, DiagnosticResult, DiagnosticVecResult,
    ParentObjectEntityNameAndSelectableName, ServerObjectEntityName, WithSpan, WithSpanPostfix,
};
use isograph_lang_types::{SelectionSet, SelectionType, SelectionTypePostfix};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    IsographDatabase, NetworkProtocol, ObjectSelectableId, ScalarSelectableId,
    client_selectable_declaration_map_from_iso_literals, expose_field_map, get_link_fields,
    get_validated_selection_set, multiple_selectable_definitions_found_diagnostic,
};

type ValidatedSelectionSet = WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>;

#[expect(clippy::type_complexity)]
#[memo]
pub fn memoized_unvalidated_reader_selection_set_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    DiagnosticResult<SelectionType<WithSpan<SelectionSet<(), ()>>, WithSpan<SelectionSet<(), ()>>>>,
> {
    // TODO use client_selectable_map
    let declaration_map = client_selectable_declaration_map_from_iso_literals(db);

    let mut map: HashMap<_, _> = declaration_map
        .iter()
        .map(|(key, declarations)| {
            let (first, rest) = declarations.split_first().expect(
                "Expected at least one item to be present in map. \
                This is indicative of a bug in Isograph.",
            );
            if rest.is_empty() {
                match first {
                    SelectionType::Scalar(s) => {
                        (*key, s.selection_set.clone().scalar_selected().wrap_ok())
                    }
                    SelectionType::Object(o) => {
                        (*key, o.selection_set.clone().object_selected().wrap_ok())
                    }
                }
            } else {
                (
                    *key,
                    multiple_selectable_definitions_found_diagnostic(
                        key.0,
                        key.1.into(),
                        match first {
                            SelectionType::Scalar(s) => s.client_field_name.location.into(),
                            SelectionType::Object(o) => o.client_pointer_name.location.into(),
                        },
                    )
                    .wrap_err(),
                )
            }
        })
        .collect();

    // we must add empty selection sets for __link fields, and TODO perhaps others.
    // TODO this should be cleaned up and thought about holistically.
    match get_link_fields(db) {
        Ok(fields) => {
            for field in fields {
                map.insert(
                    (field.parent_object_entity_name, field.name.item.into()),
                    SelectionSet { selections: vec![] }
                        .with_generated_span()
                        .scalar_selected()
                        .wrap_ok(),
                );
            }
        }
        Err(_) => {
            // TODO do not silently ignore errors in link fields. Instead, return a Result
            // from this fn
        }
    }

    // And we must also do it for expose fields. Ay ay ay
    match expose_field_map(db) {
        Ok(expose_field_map) => {
            for (key, (_, selection_set)) in expose_field_map {
                map.insert(
                    (key.0, key.1.into()),
                    selection_set
                        .reader_selection_set
                        .clone()
                        .scalar_selected()
                        .wrap_ok(),
                );
            }
        }
        Err(_) => {
            // TODO don't silently ignore this error.
        }
    }

    map
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn memoized_validated_reader_selection_set_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    DiagnosticVecResult<WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>>,
> {
    let unvalidated_map = memoized_unvalidated_reader_selection_set_map(db).to_owned();

    unvalidated_map
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                value
                    .map_err(|e| vec![e])
                    .and_then(|unvalidated_selection_set| {
                        let top_level_field_or_pointer = match unvalidated_selection_set {
                            SelectionType::Scalar(_) => {
                                ParentObjectEntityNameAndSelectableName::new(key.0, key.1.into())
                                    .scalar_selected()
                            }
                            SelectionType::Object(_) => {
                                ParentObjectEntityNameAndSelectableName::new(key.0, key.1.into())
                                    .object_selected()
                            }
                        };

                        get_validated_selection_set(
                            db,
                            unvalidated_selection_set.inner(),
                            key.0,
                            top_level_field_or_pointer,
                        )
                    }),
            )
        })
        .collect()
}

pub fn selectable_validated_reader_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> DiagnosticVecResult<ValidatedSelectionSet> {
    let map = memoized_validated_reader_selection_set_map(db);

    map.get(&(parent_server_object_entity_name, client_selectable_name))
        .expect(
            "Expected selectable to have been defined. \
            This is indicative of a bug in Isograph.",
        )
        .clone()
}
