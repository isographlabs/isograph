use std::collections::HashMap;

use common_lang_types::{
    ClientSelectableName, ParentObjectEntityNameAndSelectableName, ServerObjectEntityName,
    WithLocation, WithSpan,
};
use isograph_lang_types::{SelectionType, UnvalidatedSelection};
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    AddSelectionSetsError, EntityAccessError, IsographDatabase, NetworkProtocol,
    ValidatedSelection, client_selectable_declaration_map_from_iso_literals, get_link_fields,
    get_validated_selection_set,
};

type UnvalidatedSelectionSet = Vec<WithSpan<UnvalidatedSelection>>;
type ValidatedSelectionSet = Vec<WithSpan<ValidatedSelection>>;

#[expect(clippy::type_complexity)]
#[legacy_memo]
pub fn memoized_unvalidated_reader_selection_set_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    Result<
        SelectionType<UnvalidatedSelectionSet, UnvalidatedSelectionSet>,
        MemoizedSelectionSetError<TNetworkProtocol>,
    >,
> {
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
                        (*key, Ok(SelectionType::Scalar(s.selection_set.clone())))
                    }
                    SelectionType::Object(o) => {
                        (*key, Ok(SelectionType::Object(o.selection_set.clone())))
                    }
                }
            } else {
                (
                    *key,
                    Err(MemoizedSelectionSetError::DuplicateDefinition {
                        parent_object_entity_name: key.0,
                        client_selectable_name: key.1,
                    }),
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
                    Ok(SelectionType::Scalar(vec![])),
                );
            }
        }
        Err(_) => {
            // TODO do not silently ignore errors in link fields. Instead, return a Result
            // from this fn
        }
    }

    map
}

#[legacy_memo]
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
            Ok(selections) => Ok(selections.clone()),
            Err(e) => Err(e.clone()),
        },
        None => Err(MemoizedSelectionSetError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_selectable_name,
        }),
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
