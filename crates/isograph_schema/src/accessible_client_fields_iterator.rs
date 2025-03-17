use crate::{
    ClientFieldOrPointer, OutputFormat, ValidatedSchema, ValidatedSelection, ValidatedSelectionType,
};
use common_lang_types::WithSpan;
use isograph_lang_types::{DefinitionLocation, SelectionType, SelectionTypeContainingSelections};

// This should really be replaced with a proper visitor, or something
pub fn accessible_client_fields<'a, TOutputFormat: OutputFormat>(
    selection_type: &'a ValidatedSelectionType<'a, TOutputFormat>,
    schema: &'a ValidatedSchema<TOutputFormat>,
) -> impl Iterator<Item = ValidatedSelectionType<'a, TOutputFormat>> + 'a {
    AccessibleClientFieldIterator {
        selection_set: selection_type.selection_set_for_parent_query(),
        index: 0,
        schema,
        sub_iterator: None,
    }
}

struct AccessibleClientFieldIterator<'a, TOutputFormat: OutputFormat> {
    selection_set: &'a [WithSpan<ValidatedSelection>],
    schema: &'a ValidatedSchema<TOutputFormat>,
    index: usize,
    sub_iterator: Option<Box<AccessibleClientFieldIterator<'a, TOutputFormat>>>,
}

impl<'a, TOutputFormat: OutputFormat> Iterator
    for AccessibleClientFieldIterator<'a, TOutputFormat>
{
    type Item = ValidatedSelectionType<'a, TOutputFormat>;

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
                    SelectionTypeContainingSelections::Scalar(scalar) => {
                        match scalar.associated_data.location {
                            DefinitionLocation::Server(_) => {
                                self.index += 1;
                                continue 'main_loop;
                            }
                            DefinitionLocation::Client(client_field_id) => {
                                let nested_client_field = self.schema.client_field(client_field_id);
                                self.index += 1;
                                return Some(SelectionType::Scalar(nested_client_field));
                            }
                        }
                    }
                    SelectionTypeContainingSelections::Object(linked_field) => {
                        let mut iterator = AccessibleClientFieldIterator {
                            selection_set: &linked_field.selection_set,
                            schema: self.schema,
                            index: 0,
                            sub_iterator: None,
                        };

                        match linked_field.associated_data.field_id {
                            DefinitionLocation::Client(client_pointer_id) => {
                                let nested_client_pointer =
                                    self.schema.client_pointer(client_pointer_id);
                                self.sub_iterator = Some(Box::new(iterator));
                                self.index += 1;
                                return Some(SelectionType::Object(nested_client_pointer));
                            }
                            DefinitionLocation::Server(_) => {}
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
                }
            } else {
                return None;
            }
        }
    }
}
