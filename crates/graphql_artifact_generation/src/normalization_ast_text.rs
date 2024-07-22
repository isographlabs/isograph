use isograph_schema::{
    MergedInlineFragmentSelection, MergedLinkedFieldSelection, MergedScalarFieldSelection,
    MergedServerSelection, ValidatedSchema,
};

use crate::generate_artifacts::{get_serialized_field_arguments, NormalizationAstText};

pub(crate) fn generate_normalization_ast_text<'schema, 'a>(
    schema: &'schema ValidatedSchema,
    selection_map: impl Iterator<Item = &'a MergedServerSelection> + 'a,
    indentation_level: u8,
) -> NormalizationAstText {
    let mut normalization_ast_text = "[\n".to_string();
    for item in selection_map {
        let s = generate_normalization_ast_node(item, schema, indentation_level + 1);
        normalization_ast_text.push_str(&s);
    }
    normalization_ast_text.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    NormalizationAstText(normalization_ast_text)
}

fn generate_normalization_ast_node(
    item: &MergedServerSelection,
    schema: &ValidatedSchema,
    indentation_level: u8,
) -> String {
    match &item {
        MergedServerSelection::ScalarField(scalar_field) => {
            let MergedScalarFieldSelection {
                name, arguments, ..
            } = scalar_field;
            let indent = "  ".repeat(indentation_level as usize);
            let indent_2 = "  ".repeat((indentation_level + 1) as usize);
            let serialized_arguments =
                get_serialized_field_arguments(arguments, indentation_level + 1);
            // TODO this is bad, name is a WithLocation and impl's Display, we should fix

            format!(
                "{indent}{{\n\
                {indent_2}kind: \"Scalar\",\n\
                {indent_2}fieldName: \"{name}\",\n\
                {indent_2}arguments: {serialized_arguments},\n\
                {indent}}},\n"
            )
        }
        MergedServerSelection::LinkedField(linked_field) => {
            let MergedLinkedFieldSelection {
                name,
                selection_map,
                arguments,
                ..
            } = linked_field;
            let indent = "  ".repeat(indentation_level as usize);
            let indent_2 = "  ".repeat((indentation_level + 1) as usize);
            let serialized_arguments =
                get_serialized_field_arguments(arguments, indentation_level + 1);

            let selections = generate_normalization_ast_text(
                schema,
                selection_map.values(),
                indentation_level + 1,
            );

            format!(
                "{indent}{{\n\
                {indent_2}kind: \"Linked\",\n\
                {indent_2}fieldName: \"{name}\",\n\
                {indent_2}arguments: {serialized_arguments},\n\
                {indent_2}selections: {selections},\n\
                {indent}}},\n"
            )
        }
        MergedServerSelection::InlineFragment(inline_fragment) => {
            let MergedInlineFragmentSelection {
                type_to_refine_to,
                selection_map,
            } = inline_fragment;
            let indent = "  ".repeat(indentation_level as usize);
            let indent_2 = "  ".repeat((indentation_level + 1) as usize);

            let selections = generate_normalization_ast_text(
                schema,
                selection_map.values(),
                indentation_level + 1,
            );

            format!(
                "{indent}{{\n\
                {indent_2}kind: \"InlineFragment\",\n\
                {indent_2}type: \"{type_to_refine_to}\",\n\
                {indent_2}selections: {selections},\n\
                {indent}}},\n"
            )
        }
    }
}
