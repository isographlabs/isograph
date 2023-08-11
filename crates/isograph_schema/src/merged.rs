use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{DefinedField, NormalizationKey, SelectableFieldName, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{
    LinkedFieldSelection, ObjectId, ScalarFieldSelection, ScalarId,
    Selection::{self, ServerField},
    SelectionFieldArgument, ServerFieldSelection,
};

use crate::{SchemaObject, ValidatedDefinedField, ValidatedSchema, ValidatedSelection};

pub type MergedSelection = Selection<ScalarId, ObjectId>;
pub type MergedSelectionSet = Vec<WithSpan<MergedSelection>>;

/// A merged selection set is an input for generating:
/// - query texts
/// - normalization artifacts
/// - raw response types
///
/// TODO: SelectionSetAndUnwraps should be generic enough to handle this
pub fn merge_selection_set(
    schema: &ValidatedSchema,
    parent_type: &SchemaObject<ValidatedDefinedField>,
    selection_set: &Vec<WithSpan<ValidatedSelection>>,
) -> MergedSelectionSet {
    let mut merged_selection_set = HashMap::new();

    merge_selections_into_set(
        schema,
        &mut merged_selection_set,
        parent_type,
        selection_set,
    );

    let mut merged_fields: Vec<_> = merged_selection_set
        .into_iter()
        .map(|(_key, value)| value)
        .collect();
    merged_fields.sort();
    merged_fields
}

fn merge_selections_into_set(
    schema: &ValidatedSchema,
    merged_selection_set: &mut HashMap<NormalizationKey, WithSpan<MergedSelection>>,
    parent_type: &SchemaObject<ValidatedDefinedField>,
    validated_selections: &Vec<WithSpan<ValidatedSelection>>,
) {
    for validated_selection in validated_selections.iter() {
        let span = validated_selection.span;
        match &validated_selection.item {
            ServerField(field) => match field {
                ServerFieldSelection::ScalarField(scalar_field) => {
                    match &scalar_field.field {
                        DefinedField::ServerField(server_field_id) => {
                            let normalization_key =
                                HACK_combine_name_and_variables_into_normalization_alias(
                                    scalar_field.name.map(|x| x.into()),
                                    &scalar_field.arguments,
                                );
                            match merged_selection_set.entry(normalization_key.item) {
                                Entry::Occupied(_) => {
                                    // TODO: do we need to check for merge conflicts on scalars? Not while
                                    // we are auto-aliasing.
                                }
                                Entry::Vacant(vacant_entry) => {
                                    vacant_entry.insert(WithSpan::new(
                                        Selection::ServerField(ServerFieldSelection::ScalarField(
                                            ScalarFieldSelection {
                                                name: scalar_field.name,
                                                reader_alias: scalar_field.reader_alias,
                                                unwraps: scalar_field.unwraps.clone(),
                                                field: *server_field_id,
                                                arguments: scalar_field.arguments.clone(),
                                                normalization_alias: scalar_field
                                                    .normalization_alias,
                                            },
                                        )),
                                        span,
                                    ));
                                }
                            }
                        }
                        DefinedField::ResolverField(_) => {
                            let resolver_field_name = scalar_field.name.item;
                            let parent_field_id = parent_type
                                .resolvers
                                .iter()
                                .find(|parent_field_id| {
                                    let field = schema.resolver(**parent_field_id);
                                    field.name == resolver_field_name.into()
                                })
                                .expect("expect field to exist");
                            let resolver_field = schema.resolver(*parent_field_id);
                            if let Some((ref selection_set, _)) =
                                resolver_field.selection_set_and_unwraps
                            {
                                merge_selections_into_set(
                                    schema,
                                    merged_selection_set,
                                    parent_type,
                                    selection_set,
                                )
                            }
                        }
                    };
                }
                ServerFieldSelection::LinkedField(new_linked_field) => {
                    let normalization_key =
                        HACK_combine_name_and_variables_into_normalization_alias(
                            new_linked_field.name.map(|x| x.into()),
                            &new_linked_field.arguments,
                        );
                    match merged_selection_set.entry(normalization_key.item) {
                        Entry::Occupied(mut occupied) => {
                            let existing_selection = occupied.get_mut();
                            match &mut existing_selection.item {
                                ServerField(field) => match field {
                                    ServerFieldSelection::ScalarField(_) => {
                                        panic!("expected linked, probably a bug in Isograph")
                                    }
                                    ServerFieldSelection::LinkedField(existing_linked_field) => {
                                        let type_id = new_linked_field.field;
                                        let linked_field_parent_type =
                                            schema.schema_data.lookup_type_with_fields(type_id);
                                        HACK__merge_linked_fields(
                                            schema,
                                            &mut existing_linked_field.selection_set,
                                            &new_linked_field.selection_set,
                                            linked_field_parent_type,
                                        );
                                    }
                                },
                            }
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(WithSpan::new(
                                Selection::ServerField(ServerFieldSelection::LinkedField(
                                    LinkedFieldSelection {
                                        name: new_linked_field.name,
                                        reader_alias: new_linked_field.reader_alias,
                                        field: new_linked_field.field,
                                        selection_set: {
                                            let type_id = new_linked_field.field;
                                            let linked_field_parent_type =
                                                schema.schema_data.lookup_type_with_fields(type_id);
                                            merge_selection_set(
                                                schema,
                                                linked_field_parent_type,
                                                &new_linked_field.selection_set,
                                            )
                                        },
                                        // Unwraps **aren't necessary** in the merged data structure. The merged fields
                                        // should either be generic over the type of unwraps or it should be a different
                                        // data structure.
                                        unwraps: new_linked_field
                                            .unwraps
                                            // TODO this sucks
                                            .clone(),
                                        arguments: new_linked_field.arguments.clone(),
                                        normalization_alias: new_linked_field.normalization_alias,
                                    },
                                )),
                                span,
                            ));
                        }
                    };
                }
            },
        }
    }
}

/// In order to avoid requiring a normalization AST, we write the variables
/// used in the alias. Once we have a normalization AST, we can remove this.
#[allow(non_snake_case)]
fn HACK_combine_name_and_variables_into_normalization_alias(
    name: WithSpan<SelectableFieldName>,
    arguments: &[WithSpan<SelectionFieldArgument>],
) -> WithSpan<NormalizationKey> {
    if arguments.is_empty() {
        name.map(|x| x.into())
    } else {
        let mut alias_str = name.item.to_string();

        for argument in arguments {
            alias_str.push_str(&format!(
                "__{}_{}",
                argument.item.name.item,
                &argument.item.value.item.to_string()[1..]
            ));
        }
        name.map(|_| alias_str.intern().into())
    }
}

/// LinkedFieldSelection contains a selection set that is a Vec<...>, but we
/// really want it to be a HashMap<...>. However, we can't really do that because
/// LinkdFieldSelection has both field: TLinkedField and
/// selection_set: Vec<..., TLinkedField, ...>. If we make LinkedFieldSelection
/// generic over both TLinkedField and TSelectionSet, then we get some recursive
/// definition error.
///
/// TODO figure out a way around that!
///
/// In this function, we convert the Vec to a HashMap, do the merging, then
/// convert back. Blah!
fn HACK__merge_linked_fields(
    schema: &ValidatedSchema,
    existing_selection_set: &mut Vec<WithSpan<MergedSelection>>,
    new_selection_set: &Vec<WithSpan<ValidatedSelection>>,
    linked_field_parent_type: &SchemaObject<ValidatedDefinedField>,
) {
    let mut merged_selection_set = HashMap::new();
    for item in existing_selection_set.iter() {
        let span = item.span;
        match &item.item {
            ServerField(ServerFieldSelection::ScalarField(scalar_field)) => {
                let normalization_key = HACK_combine_name_and_variables_into_normalization_alias(
                    scalar_field.name.map(|x| x.into()),
                    &scalar_field.arguments,
                );
                merged_selection_set.insert(
                    normalization_key.item,
                    WithSpan::new(
                        Selection::ServerField(ServerFieldSelection::ScalarField(
                            scalar_field.clone(),
                        )),
                        span,
                    ),
                )
            }
            ServerField(ServerFieldSelection::LinkedField(linked_field)) => {
                let normalization_key = HACK_combine_name_and_variables_into_normalization_alias(
                    linked_field.name.map(|x| x.into()),
                    &linked_field.arguments,
                );
                merged_selection_set.insert(
                    normalization_key.item,
                    WithSpan::new(
                        Selection::ServerField(ServerFieldSelection::LinkedField(
                            linked_field.clone(),
                        )),
                        span,
                    ),
                )
            }
        };
    }

    merge_selections_into_set(
        schema,
        &mut merged_selection_set,
        linked_field_parent_type,
        new_selection_set,
    );

    let mut merged_fields: Vec<_> = merged_selection_set
        .into_iter()
        .map(|(_key, value)| value)
        .collect();
    merged_fields.sort();

    *existing_selection_set = merged_fields;
}
