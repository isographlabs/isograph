use common_lang_types::WithSpan;
use isograph_lang_types::{
    DefinitionLocation, ObjectSelection, ScalarSelection, SelectionType,
    SelectionTypeContainingSelections, SelectionTypePostfix,
};
use prelude::Postfix;

use crate::{
    IsographDatabase, NetworkProtocol, ServerObjectEntity, selectable_named,
    server_object_entity_named,
};

pub(crate) fn visit_selection_set<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_set: &[WithSpan<
        SelectionTypeContainingSelections<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >],
    parent_entity: &ServerObjectEntity<TNetworkProtocol>,
    visit_selection: &mut impl FnMut(
        SelectionType<
            &ScalarSelection<TSelectionTypeSelectionScalarFieldAssociatedData>,
            &ObjectSelection<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
        &ServerObjectEntity<TNetworkProtocol>,
    ),
) {
    for selection in selection_set.iter() {
        match &selection.item {
            SelectionType::Scalar(scalar_selection) => {
                visit_selection(scalar_selection.scalar_selected(), parent_entity)
            }
            SelectionType::Object(object_selection) => {
                visit_selection(object_selection.object_selected(), parent_entity);

                let target_entity =
                    selectable_named(db, parent_entity.name, object_selection.name.item)
                        .as_ref()
                        .expect(
                            "Expected parsing to have succeeded. \
                            This is indicative of a bug in Isograph.",
                        )
                        .expect(
                            "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                        )
                        .as_object()
                        .expect(
                            "Expected selectable to be an object. \
                            This is indicative of a bug in Isograph.",
                        );

                let target_entity_name = match target_entity {
                    DefinitionLocation::Server(s) => s.lookup(db).target_object_entity.inner(),
                    DefinitionLocation::Client(c) => c.lookup(db).target_object_entity_name.inner(),
                }
                .dereference();

                let target_entity = server_object_entity_named(db, target_entity_name)
                    .as_ref()
                    .expect(
                        "Expected parsing to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    )
                    .expect(
                        "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .lookup(db);

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
