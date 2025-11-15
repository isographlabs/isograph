use intern::Lookup;
use isograph_schema::{MergedSelectionMap, MergedServerSelection, NetworkProtocol, Schema};

pub fn generate_raw_response_type<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) -> String {
    let indent = &"  ".repeat(indentation_level as usize).to_string();

    let mut raw_response_type = String::new();
    raw_response_type.push_str(&format!("{}{{\n", indent));
    generate_raw_response_type_inner(
        &mut raw_response_type,
        schema,
        selection_map,
        indentation_level + 1,
    );
    raw_response_type.push_str(&format!("{}}}\n", indent));
    raw_response_type
}

pub fn generate_raw_response_type_inner<TNetworkProtocol: NetworkProtocol>(
    raw_response_type: &mut String,
    schema: &Schema<TNetworkProtocol>,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) {
    let indent = &"  ".repeat(indentation_level as usize).to_string();

    for item in selection_map.values() {
        match &item {
            MergedServerSelection::ScalarField(scalar_field) => {
                raw_response_type.push_str(indent);
                let normalization_alias = scalar_field.normalization_alias();
                let name = normalization_alias
                    .as_deref()
                    .unwrap_or(scalar_field.name.lookup());

                raw_response_type.push_str(&format!("{name}: unknown,\n"));
            }
            MergedServerSelection::LinkedField(linked_field) => {
                raw_response_type.push_str(indent);
                let normalization_alias = linked_field.normalization_alias();
                let name = normalization_alias
                    .as_deref()
                    .unwrap_or(linked_field.name.lookup());
                raw_response_type.push_str(&format!("{name}: {{\n"));

                generate_raw_response_type_inner(
                    raw_response_type,
                    schema,
                    &linked_field.selection_map,
                    indentation_level + 1,
                );
                raw_response_type.push_str(&format!("{indent}}},\n"));
            }
            MergedServerSelection::ClientPointer(_) => {}
            MergedServerSelection::InlineFragment(inline_fragment) => {
                generate_raw_response_type_inner(
                    raw_response_type,
                    schema,
                    &inline_fragment.selection_map,
                    indentation_level,
                );
            }
        }
    }
}
