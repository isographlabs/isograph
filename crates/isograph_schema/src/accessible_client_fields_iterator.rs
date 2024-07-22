use common_lang_types::WithSpan;
use isograph_lang_types::{Selection, ServerFieldSelection};

use crate::{FieldDefinitionLocation, ValidatedClientField, ValidatedSchema, ValidatedSelection};

impl ValidatedClientField {
    // This should really be replaced with a proper visitor, or something
    pub fn accessible_client_fields<'a>(
        &'a self,
        schema: &'a ValidatedSchema,
    ) -> impl Iterator<Item = &'a ValidatedClientField> + 'a {
        AccessibleClientFieldIterator {
            selection_set: self.selection_set_for_parent_query(),
            index: 0,
            schema,
            sub_iterator: None,
        }
    }
}
struct AccessibleClientFieldIterator<'a> {
    selection_set: &'a Vec<WithSpan<ValidatedSelection>>,
    schema: &'a ValidatedSchema,
    index: usize,
    sub_iterator: Option<Box<AccessibleClientFieldIterator<'a>>>,
}

impl<'a> Iterator for AccessibleClientFieldIterator<'a> {
    type Item = &'a ValidatedClientField;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iterator) = &mut self.sub_iterator {
            let next = iterator.next();
            if next.is_some() {
                return next;
            } else {
                self.sub_iterator = None;
            }
        }

        'main_loop: loop {
            let item = self.selection_set.get(self.index);

            if let Some(selection) = item {
                match &selection.item {
                    Selection::ServerField(server_field) => match server_field {
                        ServerFieldSelection::ScalarField(scalar) => {
                            match scalar.associated_data.location {
                                FieldDefinitionLocation::Server(_) => {
                                    self.index += 1;
                                    continue 'main_loop;
                                }
                                FieldDefinitionLocation::Client(client_field_id) => {
                                    let nested_client_field =
                                        self.schema.client_field(client_field_id);
                                    self.index += 1;
                                    return Some(nested_client_field);
                                }
                            }
                        }
                        ServerFieldSelection::LinkedField(linked_field) => {
                            let mut iterator = AccessibleClientFieldIterator {
                                selection_set: &linked_field.selection_set,
                                schema: self.schema,
                                index: 0,
                                sub_iterator: None,
                            };
                            let next = iterator.next();
                            if next.is_some() {
                                self.sub_iterator = Some(Box::new(iterator));
                                // When we exhaust the iterator, we don't want to re-create and
                                // re-iterate sub_iterator, so we also advance the index.
                                self.index += 1;
                                return next;
                            }
                            self.index += 1;
                            continue 'main_loop;
                        }
                    },
                }
            } else {
                return None;
            }
        }
    }
}
