use common_lang_types::{QueryOperationName, QueryText};
use isograph_lang_types::VariableDeclaration;
use isograph_schema::{Format, MergedSelectionMap, MergedServerSelection};
use prelude::*;

pub(crate) fn generate_query_text<'a>(
    _query_name: QueryOperationName,
    selection_map: &MergedSelectionMap,
    _query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
    format: Format,
) -> QueryText {
    let mut query_text = String::new();

    query_text.push_str("select");

    match format {
        Format::Pretty => query_text.push_str("\\\n"),
        Format::Compact => query_text.push(' '),
    }
    write_selections_for_query_text(&mut query_text, selection_map, 1, format);

    // TODO: from table
    QueryText(query_text)
}

fn write_selections_for_query_text(
    query_text: &mut String,
    items: &MergedSelectionMap,
    indentation_level: u8,
    format: Format,
) {
    let (new_line, indent) = match format {
        Format::Pretty => ("\\\n", &"  ".repeat(indentation_level as usize).to_string()),
        Format::Compact => (" ", &"".to_string()),
    };

    for item in items.values() {
        match item.reference() {
            MergedServerSelection::ScalarField(scalar_field) => {
                query_text.push_str(indent);
                query_text.push_str(&format!("{}", scalar_field.name));
                query_text.push_str(&format!(",{new_line}"));
            }
            MergedServerSelection::LinkedField(_) => {
                // TODO:
                // linked field represents a relationship, so we need to join tables
                // which means we can't just select fields here and this function
                // probably should return some structure that collects various parts of the query
                // that can be serialized later
                todo!();
            }
            MergedServerSelection::ClientObjectSelectable(_) => {}
            MergedServerSelection::InlineFragment(_) => {}
        }
    }
}
