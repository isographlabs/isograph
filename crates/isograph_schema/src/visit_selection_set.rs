use common_lang_types::WithSpan;
use isograph_lang_types::{
    ObjectSelection, ScalarFieldSelection, SelectionType, SelectionTypeContainingSelections,
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
            &ScalarFieldSelection<TSelectionTypeSelectionScalarFieldAssociatedData>,
            &ObjectSelection<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    ),
) {
    for selection in selection_set.iter() {
        match &selection.item {
            SelectionType::Scalar(scalar) => visit_selection(SelectionType::Scalar(scalar)),
            SelectionType::Object(object) => {
                visit_selection(SelectionType::Object(object));
                visit_selection_set(&object.selection_set, visit_selection);
            }
        }
    }
}
