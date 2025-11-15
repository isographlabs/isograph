use std::collections::HashMap;

use common_lang_types::{ClientSelectableName, ServerObjectEntityName, WithLocation, WithSpan};
use isograph_lang_types::{SelectionType, UnvalidatedSelection};
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    AddSelectionSetsError, EntityAccessError, IsographDatabase, NetworkProtocol,
    ValidatedSelection, client_selectable_declaration_map_from_iso_literals,
    get_validated_selection_set, server_object_entity_named,
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
    let declaration_map_memo_ref = client_selectable_declaration_map_from_iso_literals(db);
    let declaration_map = declaration_map_memo_ref.lookup();

    declaration_map
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
        .collect()
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
                    let parent_object_entity_memo_ref = server_object_entity_named(db, key.0);
                    let parent_object_entity =
                        parent_object_entity_memo_ref.try_lookup()?.as_ref().expect(
                            "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                        );
                    let top_level_field_or_pointer = match unvalidated_selection_set {
                        SelectionType::Scalar(_) => {
                            SelectionType::Scalar((key.0, key.1.into()).into())
                        }
                        SelectionType::Object(_) => {
                            SelectionType::Object((key.0, key.1.into()).into())
                        }
                    };

                    get_validated_selection_set(
                        db,
                        unvalidated_selection_set.inner(),
                        &parent_object_entity.item,
                        top_level_field_or_pointer,
                    )
                    .map_err(|e| e.into())
                }),
            )
        })
        .collect()
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
}
