use common_lang_types::WithSpan;
use isograph_lang_types::{
    ObjectSelection, ScalarSelection, SelectionType, SelectionTypeContainingSelections,
    SelectionTypePostFix,
};

pub(crate) fn visit_selection_set<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
>(
    selection_set: &[WithSpan<
        SelectionTypeContainingSelections<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >],
    visit_selection: &mut impl FnMut(
        SelectionType<
            &ScalarSelection<TSelectionTypeSelectionScalarFieldAssociatedData>,
            &ObjectSelection<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    ),
) {
    for selection in selection_set.iter() {
        match &selection.item {
            SelectionType::Scalar(scalar) => visit_selection(scalar.scalar_selected()),
            SelectionType::Object(object) => {
                visit_selection(object.object_selected());
                visit_selection_set(&object.selection_set, visit_selection);
            }
        }
    }
}
