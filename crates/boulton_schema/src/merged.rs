use std::collections::{hash_map::Entry, HashMap};

use boulton_lang_types::{
    FieldSelection, LinkedFieldSelection, ScalarFieldSelection,
    Selection::{self, Field},
    SelectionSetAndUnwraps,
};
use common_lang_types::{
    DefinedField, FieldNameOrAlias, TypeWithFieldsId, TypeWithoutFieldsId, WithSpan,
};

use crate::{SchemaTypeWithFields, ValidatedSchema, ValidatedSelectionSetAndUnwraps};

pub fn merge_selection_set(
    schema: &ValidatedSchema,
    parent_type: SchemaTypeWithFields,
    selection_set: &ValidatedSelectionSetAndUnwraps,
) -> Vec<WithSpan<Selection<TypeWithoutFieldsId, TypeWithFieldsId>>> {
    // TODO restructure types such that
    let mut merged_selection_set: HashMap<
        FieldNameOrAlias,
        WithSpan<Selection<TypeWithoutFieldsId, TypeWithFieldsId>>,
    > = HashMap::new();

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
    merged_selection_set: &mut HashMap<
        FieldNameOrAlias,
        WithSpan<Selection<TypeWithoutFieldsId, TypeWithFieldsId>>,
    >,
    parent_type: SchemaTypeWithFields,
    value: &ValidatedSelectionSetAndUnwraps,
) {
    for item in value.selection_set.iter() {
        let span = item.span;
        match &item.item {
            Field(field) => match field {
                FieldSelection::ScalarField(scalar_field) => {
                    match &scalar_field.field {
                        DefinedField::ServerField(server_field_id) => {
                            match merged_selection_set.entry(scalar_field.name.item.into()) {
                                Entry::Occupied(_) => {
                                    // TODO check for merge conflicts, or transform them to not be merge
                                    // conflicts by auto-aliasing and the like.
                                }
                                Entry::Vacant(vacant_entry) => {
                                    vacant_entry.insert(WithSpan::new(
                                        Selection::Field(FieldSelection::ScalarField(
                                            ScalarFieldSelection {
                                                name: scalar_field.name,
                                                alias: scalar_field.alias,
                                                unwraps: scalar_field.unwraps.clone(),
                                                field: *server_field_id,
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
                                .fields()
                                .iter()
                                .find(|parent_field_id| {
                                    let field = schema.field(**parent_field_id);
                                    field.name == resolver_field_name.into()
                                })
                                .expect("expect field to exist");
                            let field = schema.field(*parent_field_id);
                            match &field.field_type {
                                DefinedField::ServerField(_) => panic!("Expected resolver"),
                                DefinedField::ResolverField(r) => {
                                    if let Some(ref selection_set_and_unwraps) =
                                        r.selection_set_and_unwraps
                                    {
                                        merge_selections_into_set(
                                            schema,
                                            merged_selection_set,
                                            parent_type,
                                            selection_set_and_unwraps,
                                        )
                                    }
                                }
                            }
                        }
                    };
                }
                FieldSelection::LinkedField(linked_field) => {
                    match merged_selection_set.entry(linked_field.name.item.into()) {
                        Entry::Occupied(_) => {
                            // TODO check for merge conflicts, or transform them to not be merge
                            // conflicts by auto-aliasing and the like.
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(WithSpan::new(
                                Selection::Field(FieldSelection::LinkedField(
                                    LinkedFieldSelection {
                                        name: linked_field.name,
                                        alias: linked_field.alias,
                                        field: linked_field.field,
                                        selection_set_and_unwraps: {
                                            let type_id = linked_field.field;
                                            let linked_field_parent_type =
                                                schema.schema_data.lookup_type_with_fields(type_id);
                                            SelectionSetAndUnwraps {
                                                selection_set: merge_selection_set(
                                                    schema,
                                                    linked_field_parent_type,
                                                    &linked_field.selection_set_and_unwraps,
                                                ),
                                                unwraps: linked_field
                                                    .selection_set_and_unwraps
                                                    .unwraps
                                                    // TODO this sucks
                                                    .clone(),
                                            }
                                        },
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
