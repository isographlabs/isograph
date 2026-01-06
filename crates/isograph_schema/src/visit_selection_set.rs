use common_lang_types::WithEmbeddedLocation;
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, ObjectSelection, ScalarSelection, Selection,
    SelectionType, SelectionTypePostfix,
};
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelEntity, IsographDatabase, flattened_entity_named,
    selectable_named,
};

/// This is visiting an unvalidated selection set, and should not panic.
/// Instead, we simply avoid visiting selections where parents aren't found.
///
/// This function should probably be renamed, as it's not what you expect, otherwise!
pub(crate) fn visit_selection_set<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    selection_set: &[WithEmbeddedLocation<Selection>],
    parent_entity: &FlattenedDataModelEntity<TCompilationProfile>,
    visit_selection: &mut impl FnMut(
        SelectionType<&ScalarSelection, &ObjectSelection>,
        &FlattenedDataModelEntity<TCompilationProfile>,
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
                        let target_entity_name = match selectable.target_entity.item.as_ref() {
                            Ok(annotation) => annotation.inner().0,
                            Err(_) => continue,
                        };
                        let entity = flattened_entity_named(db, target_entity_name);
                        let entity = match entity {
                            Some(entity) => entity.lookup(db),
                            None => {
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
                    DefinitionLocation::Server(s) => match s.target_entity.item.as_ref() {
                        Ok(annotation) => annotation.inner(),
                        Err(_) => continue,
                    },
                    DefinitionLocation::Client(c) => c.lookup(db).target_entity_name.inner(),
                }
                .0;

                let target_entity = match flattened_entity_named(db, target_entity_name) {
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
