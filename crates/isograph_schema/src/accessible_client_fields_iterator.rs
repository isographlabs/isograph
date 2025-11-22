use common_lang_types::WithSpan;
use isograph_lang_types::{
    DefinitionLocation, SelectionSet, SelectionTypeContainingSelections, SelectionTypePostfix,
};
use prelude::Postfix;

use crate::{
    ClientSelectableId, IsographDatabase, NetworkProtocol, ObjectSelectableId,
    OwnedClientSelectable, ScalarSelectableId,
    client_object_selectable_selection_set_for_parent_query,
    client_scalar_selectable_selection_set_for_parent_query,
};

use isograph_lang_types::SelectionType;

// This should really be replaced with a proper visitor, or something
pub fn accessible_client_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_type: &OwnedClientSelectable<TNetworkProtocol>,
) -> impl Iterator<Item = ClientSelectableId> {
    let selection_set = match selection_type {
        SelectionType::Scalar(scalar) => client_scalar_selectable_selection_set_for_parent_query(
            db,
            scalar.parent_object_entity_name,
            scalar.name.item,
        )
        .expect("Expected selection set to be valid"),

        SelectionType::Object(object) => client_object_selectable_selection_set_for_parent_query(
            db,
            object.parent_object_entity_name,
            object.name.item,
        )
        .expect("Expected selection set to be valid"),
    };

    AccessibleClientFieldIterator {
        selection_set,
        index: 0,
        sub_iterator: None,
    }
}

struct AccessibleClientFieldIterator {
    // TODO have a reference
    selection_set: WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>,
    index: usize,
    sub_iterator: Option<Box<AccessibleClientFieldIterator>>,
}

impl Iterator for AccessibleClientFieldIterator {
    type Item = ClientSelectableId;

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
            let item = self.selection_set.item.selections.get(self.index);

            if let Some(selection) = item {
                match &selection.item {
                    SelectionTypeContainingSelections::Scalar(scalar) => {
                        match scalar.associated_data {
                            DefinitionLocation::Server(_) => {
                                self.index += 1;
                                continue 'main_loop;
                            }
                            DefinitionLocation::Client((
                                parent_object_entity_name,
                                client_field_name,
                            )) => {
                                self.index += 1;
                                return (parent_object_entity_name, client_field_name)
                                    .scalar_selected()
                                    .some();
                            }
                        }
                    }
                    SelectionTypeContainingSelections::Object(linked_field) => {
                        let mut iterator = AccessibleClientFieldIterator {
                            selection_set: linked_field.selection_set.clone(),
                            index: 0,
                            sub_iterator: None,
                        };

                        match linked_field.associated_data {
                            DefinitionLocation::Client(client_pointer_id) => {
                                // TODO: include pointer target link type
                                // https://github.com/isographlabs/isograph/issues/719
                                self.sub_iterator = Some(iterator.boxed());
                                self.index += 1;
                                return client_pointer_id.object_selected().some();
                            }
                            DefinitionLocation::Server(_) => {}
                        };
                        let next = iterator.next();
                        if next.is_some() {
                            self.sub_iterator = Some(iterator.boxed());
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
