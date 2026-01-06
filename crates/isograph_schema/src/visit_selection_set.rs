use common_lang_types::WithEmbeddedLocation;
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, ObjectSelection, ScalarSelection, Selection,
    SelectionType, SelectionTypePostfix,
};
use prelude::{ErrClone, Postfix};

use crate::{
    CompilationProfile, IsographDatabase, ServerEntity, selectable_named, server_entity_named,
};

/// This is visiting an unvalidated selection set, and should not panic.
/// Instead, we simply avoid visiting selections where parents aren't found.
///
/// This function should probably be renamed, as it's not what you expect, otherwise!
pub(crate) fn visit_selection_set<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    selection_set: &[WithEmbeddedLocation<Selection>],
    parent_entity: &ServerEntity<TCompilationProfile>,
    visit_selection: &mut impl FnMut(
        SelectionType<&ScalarSelection, &ObjectSelection>,
        &ServerEntity<TCompilationProfile>,
    ),
) {
    for selection in selection_set.iter() {
        match selection.item.reference() {
            SelectionType::Scalar(scalar_selection) => {
                visit_selection(scalar_selection.scalar_selected(), parent_entity)
            }
            SelectionType::Object(object_selection) => {
                visit_selection(object_selection.object_selected(), parent_entity);

                let selectable =
                    match selectable_named(db, parent_entity.name.item, object_selection.name.item)
                        .as_ref()
                        .expect(
                            "Expected parsing to have succeeded. \
                            This is indicative of a bug in Isograph.",
                        ) {
                        Some(s) => s,
                        None => continue,
                    };

                let target_entity = match selectable {
                    DefinitionLocation::Server(s) => {
                        let selectable = s.lookup(db);
                        let target_entity_name = selectable.target_entity_name.inner().0;
                        let entity = match server_entity_named(db, target_entity_name).clone_err() {
                            Ok(o) => match o {
                                Some(entity) => entity.lookup(db),
                                None => {
                                    continue;
                                }
                            },
                            Err(_) => {
                                continue;
                            }
                        };

                        if entity.selection_info.as_scalar().is_some() {
                            continue;
                        }

                        selectable.server_defined()
                    }
                    DefinitionLocation::Client(c) => match c {
                        SelectionType::Scalar(_) => {
                            continue;
                        }
                        SelectionType::Object(o) => o.client_defined(),
                    },
                };

                let target_entity_name = match target_entity {
                    DefinitionLocation::Server(s) => s.target_entity_name.inner(),
                    DefinitionLocation::Client(c) => c.lookup(db).target_entity_name.inner(),
                }
                .0;

                let target_entity =
                    match server_entity_named(db, target_entity_name).as_ref().expect(
                        "Expected parsing to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    ) {
                        Some(s) => s.lookup(db),
                        None => continue,
                    };

                visit_selection_set(
                    db,
                    &object_selection.selection_set.item.selections,
                    target_entity,
                    visit_selection,
                );
            }
        }
    }
}
