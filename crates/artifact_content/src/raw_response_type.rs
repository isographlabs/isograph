use intern::Lookup;
use isograph_lang_types::TypeAnnotation;
use isograph_schema::{
    IsographDatabase, MergedSelectionMap, MergedServerSelection, NetworkProtocol,
    TYPENAME_FIELD_NAME, server_scalar_entity_javascript_name, server_scalar_selectable_named,
};
use std::collections::BTreeMap;

use crate::generate_artifacts::print_javascript_type_declaration;

pub fn generate_raw_response_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) -> String {
    let indent = &"  ".repeat(indentation_level as usize).to_string();

    let mut raw_response_type = String::new();
    raw_response_type.push_str(&format!("{}{{\n", indent));
    generate_raw_response_type_inner(
        db,
        &mut raw_response_type,
        selection_map,
        indentation_level + 1,
    );
    raw_response_type.push_str(&format!("{}}}\n", indent));
    raw_response_type
}

pub fn generate_raw_response_type_inner<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    raw_response_type: &mut String,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) {
    let indent = &"  ".repeat(indentation_level as usize).to_string();
    let mut raw_response_type_inner = String::new();
    let mut fragments = BTreeMap::new();

    for item in selection_map.values() {
        match &item {
            MergedServerSelection::ScalarField(scalar_field) => {
                raw_response_type_inner.push_str(indent);
                let normalization_alias = scalar_field.normalization_alias();
                let name = normalization_alias
                    .as_deref()
                    .unwrap_or(scalar_field.name.lookup());

                let server_scalar_selectable = server_scalar_selectable_named(
                    db,
                    scalar_field.parent_object_entity_name,
                    scalar_field.name.into(),
                )
                .as_ref()
                .expect(
                    "Expected validation to have succeeded. \
                            This is indicative of a bug in Isograph.",
                )
                .as_ref();

                let server_scalar_selectable = match server_scalar_selectable {
                    Some(server_scalar_selectable) => server_scalar_selectable,
                    None => {
                        if scalar_field.name == *TYPENAME_FIELD_NAME {
                            // this might be __typename defined on an interface, this currently not supported,
                            // we use a work around here
                            raw_response_type_inner.push_str(&format!("{name}: string,\n",));
                            continue;
                        }
                        panic!(
                            "Expected selectable to exist. \
                                    This is indicative of a bug in Isograph.",
                        );
                    }
                };

                let raw_type = server_scalar_selectable.target_scalar_entity.as_ref().map(
                    &mut |scalar_entity_name| match server_scalar_selectable
                        .javascript_type_override
                    {
                        Some(javascript_name) => javascript_name,
                        None => server_scalar_entity_javascript_name(db, *scalar_entity_name)
                            .as_ref()
                            .expect(
                                "Expected parsing to not have failed. \
                                        This is indicative of a bug in Isograph.",
                            )
                            .expect(
                                "Expected entity to exist. \
                                        This is indicative of a bug in Isograph.",
                            ),
                    },
                );

                let is_optional = matches!(
                    server_scalar_selectable.target_scalar_entity,
                    TypeAnnotation::Union(_)
                );

                raw_response_type_inner.push_str(&format!(
                    "{name}{}: {},\n",
                    if is_optional { "?" } else { "" },
                    print_javascript_type_declaration(&raw_type)
                ));
            }
            MergedServerSelection::LinkedField(linked_field) => {
                raw_response_type_inner.push_str(indent);
                let normalization_alias = linked_field.normalization_alias();
                let name = normalization_alias
                    .as_deref()
                    .unwrap_or(linked_field.name.lookup());
                raw_response_type_inner.push_str(&format!("{name}: {{\n"));

                generate_raw_response_type_inner(
                    db,
                    &mut raw_response_type_inner,
                    &linked_field.selection_map,
                    indentation_level + 1,
                );
                raw_response_type_inner.push_str(&format!("{indent}}},\n"));
            }
            MergedServerSelection::ClientPointer(_) => {}
            MergedServerSelection::InlineFragment(inline_fragment) => {
                let mut raw_response_type = String::new();
                let mut inline_fragment_selection_map = BTreeMap::new();

                inline_fragment_selection_map.extend(selection_map.clone().into_iter().filter(
                    |(_, value)| !matches!(value, MergedServerSelection::InlineFragment(_)),
                ));

                inline_fragment_selection_map
                    .extend(inline_fragment.selection_map.clone().into_iter());

                generate_raw_response_type_inner(
                    db,
                    &mut raw_response_type,
                    &inline_fragment_selection_map,
                    indentation_level,
                );

                fragments.insert(inline_fragment.type_to_refine_to, raw_response_type);
            }
        }
    }

    if fragments.is_empty() {
        raw_response_type.push_str(&raw_response_type_inner);
    } else {
        let indent = &"  ".repeat((indentation_level - 1) as usize).to_string();

        let mut iter = fragments.into_iter();
        if let Some((_, fragment)) = iter.next() {
            raw_response_type.push_str(&fragment);
        };

        for (_, fragment) in iter {
            raw_response_type.push_str(&format!("{indent}}} | {{\n"));
            raw_response_type.push_str(&fragment);
        }
    }
}
