use std::collections::HashMap;

use common_lang_types::{
    ClientSelectableName, Diagnostic, ParentObjectEntityNameAndSelectableName,
    ServerObjectEntityName, WithSpan, WithSpanPostfix,
};
use isograph_lang_types::{SelectionSet, SelectionType, SelectionTypePostfix};
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::{
    IsographDatabase, NetworkProtocol, ObjectSelectableId, ScalarSelectableId,
    client_selectable_declaration_map_from_iso_literals, expose_field_map, get_link_fields,
    get_validated_selection_set,
};

type ValidatedSelectionSet = WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>;

#[expect(clippy::type_complexity)]
#[memo]
pub fn memoized_unvalidated_reader_selection_set_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    Result<
        SelectionType<WithSpan<SelectionSet<(), ()>>, WithSpan<SelectionSet<(), ()>>>,
        MemoizedSelectionSetError,
    >,
> {
    // TODO use client_selectable_map
    let declaration_map = client_selectable_declaration_map_from_iso_literals(db);

    let mut map = declaration_map
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
                    MemoizedSelectionSetError::DuplicateDefinition {
                        parent_object_entity_name: key.0,
                        client_selectable_name: key.1,
                    }
                    .wrap_err(),
                )
            }
        })
        .collect::<HashMap<_, _>>();

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
    Result<
        WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>,
        MemoizedSelectionSetError,
    >,
> {
    let unvalidated_map = memoized_unvalidated_reader_selection_set_map(db).to_owned();

    unvalidated_map
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                value.and_then(|unvalidated_selection_set| {
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
                    .map_err(|e| MemoizedSelectionSetError::ValidateAddSelectionSetsResultWithMultipleErrors { errors: e })
                }),
            )
        })
        .collect()
}

pub fn selectable_validated_reader_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Result<ValidatedSelectionSet, MemoizedSelectionSetError> {
    let map = memoized_validated_reader_selection_set_map(db);

    match map.get(&(parent_server_object_entity_name, client_selectable_name)) {
        Some(result) => match result {
            Ok(selections) => selections.clone().wrap_ok(),
            Err(e) => e.clone().wrap_err(),
        },
        None => MemoizedSelectionSetError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_selectable_name,
        }
        .wrap_err(),
    }
}

#[derive(Clone, Error, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub enum MemoizedSelectionSetError {
    #[error("`{parent_object_entity_name}.{client_selectable_name}` has been defined twice.")]
    DuplicateDefinition {
        parent_object_entity_name: ServerObjectEntityName,
        client_selectable_name: ClientSelectableName,
    },

    #[error("{0}", 
        errors.iter().map(|error| format!("{}", error)).collect::<Vec<_>>().join("\n")
    )]
    ValidateAddSelectionSetsResultWithMultipleErrors { errors: Vec<Diagnostic> },

    #[error("{0}")]
    EntityAccessError(Diagnostic),

    // TODO this should be an option in the return value, not an error variant, but
    // realistically, that's super annoying.
    #[error("`{parent_server_object_entity_name}.{selectable_name}` is not defined.")]
    NotFound {
        parent_server_object_entity_name: ServerObjectEntityName,
        selectable_name: ClientSelectableName,
    },
}
