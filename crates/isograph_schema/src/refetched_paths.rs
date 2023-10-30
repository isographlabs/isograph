use std::collections::HashSet;

use common_lang_types::WithSpan;
use isograph_lang_types::{ObjectId, Selection, ServerFieldSelection};

use crate::{
    DefinedField, NameAndArguments, PathToRefetchField, ResolverVariant,
    ValidatedScalarDefinedField, ValidatedSchema,
};

pub fn refetched_paths_with_path(
    selection_set: &[WithSpan<Selection<ValidatedScalarDefinedField, ObjectId>>],
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
) -> HashSet<PathToRefetchField> {
    let mut paths = HashSet::default();

    for selection in selection_set {
        match &selection.item {
            Selection::ServerField(field) => match field {
                ServerFieldSelection::ScalarField(scalar) => match scalar.associated_data {
                    DefinedField::ServerField(_) => {
                        // Do nothing, we encountered a server field
                    }
                    DefinedField::ResolverField(resolver_field_id) => {
                        let resolver_field = schema.resolver(resolver_field_id);
                        match resolver_field.variant {
                            WithSpan {
                                item:
                                    ResolverVariant::RefetchField | ResolverVariant::MutationField(_),
                                ..
                            } => {
                                paths.insert(PathToRefetchField {
                                    linked_fields: path.clone(),
                                });
                            }
                            _ => {
                                // For non-refetch fields, we need to recurse into the selection set
                                // (if there is one)
                                match &resolver_field.selection_set_and_unwraps {
                                    Some((selection_set, _unwraps)) => {
                                        let new_paths =
                                            refetched_paths_with_path(selection_set, schema, path);

                                        paths.extend(new_paths.into_iter());
                                    }
                                    None => panic!("Resolver field has no selection set"),
                                };
                            }
                        }
                    }
                },
                ServerFieldSelection::LinkedField(linked_field_selection) => {
                    path.push(NameAndArguments {
                        name: linked_field_selection.name.item,
                        // arguments: linked_field_selection.arguments.clone(),
                        arguments: vec![],
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
