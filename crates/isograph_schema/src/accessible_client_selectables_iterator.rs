use common_lang_types::{EntityName, WithEmbeddedLocation, WithLocationPostfix};
use isograph_lang_types::{DefinitionLocation, SelectionSet, SelectionTypePostfix};
use prelude::Postfix;

use crate::{
    ClientSelectableId, IsographDatabase, MemoRefClientSelectable, NetworkProtocol,
    client_scalar_selectable_selection_set_for_parent_query, selectable_named,
    selectable_reader_selection_set,
};

use isograph_lang_types::SelectionType;

// This should really be replaced with a proper visitor, or something
pub fn accessible_client_selectables<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_type: MemoRefClientSelectable<TNetworkProtocol>,
) -> impl Iterator<Item = WithEmbeddedLocation<ClientSelectableId>> {
    let (selection_set, parent_entity_name) = match selection_type {
        SelectionType::Scalar(scalar) => {
            let scalar = scalar.lookup(db);
            (
                client_scalar_selectable_selection_set_for_parent_query(
                    db,
                    scalar.parent_entity_name,
                    scalar.name,
                )
                .expect("Expected selection set to be valid"),
                scalar.parent_entity_name,
            )
        }

        SelectionType::Object(object) => {
            let object = object.lookup(db);
            let parent_object_entity_name = object.parent_entity_name;
            let client_object_selectable_name = object.name;
            (
                selectable_reader_selection_set(
                    db,
                    parent_object_entity_name,
                    client_object_selectable_name,
                )
                .expect("Expected selection set to be valid")
                .lookup(db)
                .clone()
                .note_todo("Do not clone"),
                object.parent_entity_name,
            )
        }
    };

    AccessibleClientSelectableIterator {
        selection_set,
        index: 0,
        sub_iterator: None,
        parent_entity_name,
        db,
    }
}

struct AccessibleClientSelectableIterator<'db, TNetworkProtocol: NetworkProtocol> {
    // TODO have a reference
    db: &'db IsographDatabase<TNetworkProtocol>,
    selection_set: WithEmbeddedLocation<SelectionSet>,
    index: usize,
    sub_iterator: Option<Box<AccessibleClientSelectableIterator<'db, TNetworkProtocol>>>,
    parent_entity_name: EntityName,
}

impl<'db, TNetworkProtocol: NetworkProtocol> Iterator
    for AccessibleClientSelectableIterator<'db, TNetworkProtocol>
{
    type Item = WithEmbeddedLocation<ClientSelectableId>;

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
                let selectable =
                    selectable_named(self.db, self.parent_entity_name, selection.item.name())
                        .as_ref()
                        .expect(
                            "Expected parsing to have succeeded. \
                            This is indicative of a bug in Isograph.",
                        )
                        .expect(
                            "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                        );
                match &selection.item {
                    SelectionType::Scalar(scalar_selection) => {
                        match selectable.as_scalar().expect(
                            "Expected selectable to be a scalar. \
                            This is indicative of a bug in Isograph.",
                        ) {
                            DefinitionLocation::Server(_) => {
                                self.index += 1;
                                continue 'main_loop;
                            }
                            DefinitionLocation::Client(_) => {
                                self.index += 1;
                                return (self.parent_entity_name, selection.item.name())
                                    .scalar_selected()
                                    .with_embedded_location(scalar_selection.name.embedded_location)
                                    .wrap_some();
                            }
                        };
                    }
                    SelectionType::Object(object_selection) => {
                        let object_selectable = selectable.as_object().expect(
                            "Expected selectable to be an object. \
                            This is indicative of a bug in Isograph.",
                        );

                        // TODO don't match on object_selectable twice
                        let target_entity_name = match object_selectable {
                            DefinitionLocation::Server(s) => {
                                s.lookup(self.db).target_entity_name.inner().dereference()
                            }
                            DefinitionLocation::Client(c) => {
                                c.lookup(self.db).target_entity_name.inner().dereference()
                            }
                        };

                        let mut iterator = AccessibleClientSelectableIterator {
                            selection_set: object_selection.selection_set.clone(),
                            index: 0,
                            sub_iterator: None,
                            db: self.db,
                            parent_entity_name: target_entity_name,
                        };

                        match object_selectable {
                            DefinitionLocation::Server(_) => {}
                            DefinitionLocation::Client(_) => {
                                self.sub_iterator = Some(iterator.boxed());
                                self.index += 1;
                                return (self.parent_entity_name, object_selection.name.item)
                                    .object_selected()
                                    .with_embedded_location(object_selection.name.embedded_location)
                                    .wrap_some();
                            }
                        }
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
