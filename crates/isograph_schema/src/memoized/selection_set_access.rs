use std::collections::HashMap;

use common_lang_types::{
    ClientSelectableName, ParentObjectEntityNameAndSelectableName, ServerObjectEntityName,
    WithLocation, WithSpan,
};
use isograph_lang_types::{SelectionType, UnvalidatedSelection};
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::{
    AddSelectionSetsError, EntityAccessError, IsographDatabase, NetworkProtocol,
    ValidatedSelection, client_selectable_declaration_map_from_iso_literals, expose_field_map,
    get_link_fields, get_validated_selection_set,
};

type UnvalidatedSelectionSet = Vec<WithSpan<UnvalidatedSelection>>;
type ValidatedSelectionSet = Vec<WithSpan<ValidatedSelection>>;

#[expect(clippy::type_complexity)]
#[memo]
pub fn memoized_unvalidated_reader_selection_set_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    Result<
        SelectionType<UnvalidatedSelectionSet, UnvalidatedSelectionSet>,
        MemoizedSelectionSetError<TNetworkProtocol>,
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
                        (*key, SelectionType::Scalar(s.selection_set.clone()).ok())
                    }
                    SelectionType::Object(o) => {
                        (*key, SelectionType::Object(o.selection_set.clone()).ok())
                    }
                }
            } else {
                (
                    *key,
                    MemoizedSelectionSetError::DuplicateDefinition {
                        parent_object_entity_name: key.0,
                        client_selectable_name: key.1,
                    }
                    .err(),
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
                    SelectionType::Scalar(vec![]).ok(),
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
                    SelectionType::Scalar(selection_set.reader_selection_set.clone()).ok(),
                );
            }
        }
        Err(_) => {
            // TODO don't silently ignore this error.
        }
    }

    map
}

#[memo]
pub fn memoized_validated_reader_selection_set_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    Result<ValidatedSelectionSet, MemoizedSelectionSetError<TNetworkProtocol>>,
> {
    let unvalidated_map = memoized_unvalidated_reader_selection_set_map(db).to_owned();

    unvalidated_map
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                value.and_then(|unvalidated_selection_set| {
                    let top_level_field_or_pointer = match unvalidated_selection_set {
                        SelectionType::Scalar(_) => SelectionType::Scalar(
                            ParentObjectEntityNameAndSelectableName::new(key.0, key.1.into()),
                        ),
                        SelectionType::Object(_) => SelectionType::Object(
                            ParentObjectEntityNameAndSelectableName::new(key.0, key.1.into()),
                        ),
                    };

                    get_validated_selection_set(
                        db,
                        unvalidated_selection_set.inner(),
                        key.0,
                        top_level_field_or_pointer,
                    )
                    .map_err(|e| e.into())
                }),
            )
        })
        .collect()
}

pub fn selectable_validated_reader_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Result<ValidatedSelectionSet, MemoizedSelectionSetError<TNetworkProtocol>> {
    let map = memoized_validated_reader_selection_set_map(db);

    match map.get(&(parent_server_object_entity_name, client_selectable_name)) {
        Some(result) => match result {
            Ok(selections) => selections.clone().ok(),
            Err(e) => e.clone().err(),
        },
        None => MemoizedSelectionSetError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_selectable_name,
        }
        .err(),
    }
}

#[derive(Clone, Error, Eq, PartialEq, Debug)]
pub enum MemoizedSelectionSetError<TNetworkProtocol: NetworkProtocol> {
    #[error("`{parent_object_entity_name}.{client_selectable_name}` has been defined twice.")]
    DuplicateDefinition {
        parent_object_entity_name: ServerObjectEntityName,
        client_selectable_name: ClientSelectableName,
    },

    #[error("{0}", 
        errors.iter().map(|error| format!("{}", error.for_display())).collect::<Vec<_>>().join("\n")
    )]
    ValidateAddSelectionSetsResultWithMultipleErrors {
        #[from]
        errors: Vec<WithLocation<AddSelectionSetsError<TNetworkProtocol>>>,
    },

    #[error("{0}")]
    EntityAccessError(#[from] EntityAccessError<TNetworkProtocol>),

    // TODO this should be an option in the return value, not an error variant, but
    // realistically, that's super annoying.
    #[error("`{parent_server_object_entity_name}.{selectable_name}` is not defined.")]
    NotFound {
        parent_server_object_entity_name: ServerObjectEntityName,
        selectable_name: ClientSelectableName,
    },
}
