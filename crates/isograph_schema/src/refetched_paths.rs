use std::collections::HashSet;

use common_lang_types::WithSpan;
use isograph_lang_types::{Selection, ServerFieldSelection};

use crate::{
    ArgumentKeyAndValue, ClientFieldVariant, FieldDefinitionLocation, NameAndArguments,
    PathToRefetchField, ValidatedSchema, ValidatedSelection,
};

pub fn refetched_paths_with_path(
    selection_set: &[WithSpan<ValidatedSelection>],
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
) -> HashSet<PathToRefetchField> {
    let mut paths = HashSet::default();

    for selection in selection_set {
        match &selection.item {
            Selection::ServerField(field) => match field {
                ServerFieldSelection::ScalarField(scalar) => {
                    match scalar.associated_data.location {
                        FieldDefinitionLocation::Server(_) => {
                            // Do nothing, we encountered a server field
                        }
                        FieldDefinitionLocation::Client(client_field_id) => {
                            let client_field = schema.client_field(client_field_id);
                            match client_field.variant {
                                ClientFieldVariant::ImperativelyLoadedField(_) => {
                                    paths.insert(PathToRefetchField {
                                        linked_fields: path.clone(),
                                        field_name: client_field.name,
                                    });
                                }
                                _ => {
                                    // For non-refetch fields, we need to recurse into the selection set
                                    // (if there is one)
                                    match &client_field.selection_set_and_unwraps {
                                        Some((selection_set, _unwraps)) => {
                                            let new_paths = refetched_paths_with_path(
                                                selection_set,
                                                schema,
                                                path,
                                            );

                                            paths.extend(new_paths.into_iter());
                                        }
                                        None => panic!("Client field has no selection set"),
                                    };
                                }
                            }
                        }
                    }
                }
                ServerFieldSelection::LinkedField(linked_field_selection) => {
                    path.push(NameAndArguments {
                        name: linked_field_selection.name.item.into(),
                        arguments: linked_field_selection
                            .arguments
                            .iter()
                            .map(|x| ArgumentKeyAndValue {
                                key: x.item.name.item,
                                value: x.item.value.item.clone(),
                            })
                            .collect::<Vec<_>>(),
                    });

                    let new_paths = refetched_paths_with_path(
                        &linked_field_selection.selection_set,
                        schema,
                        path,
                    );

                    paths.extend(new_paths.into_iter());

                    path.pop();
                }
            },
        };
    }

    paths
}
