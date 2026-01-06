use intern::Lookup;
use isograph_schema::{
    CompilationProfile, IsographDatabase, MergedSelectionMap, MergedServerSelection,
    TargetPlatform, server_selectable_named,
};
use prelude::Postfix;
use std::collections::BTreeMap;

use crate::generate_artifacts::print_javascript_type_declaration;

pub fn generate_raw_response_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
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

pub fn generate_raw_response_type_inner<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    raw_response_type: &mut String,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) {
    let indent = &"  ".repeat(indentation_level as usize).to_string();
    let mut raw_response_type_inner = String::new();
    let mut fragments = BTreeMap::new();

    for item in selection_map.values() {
        match item.reference() {
            MergedServerSelection::ScalarField(scalar_field) => {
                let normalization_alias = scalar_field.normalization_alias();
                let name = normalization_alias
                    .as_deref()
                    .unwrap_or(scalar_field.name.lookup());

                let server_scalar_selectable = server_selectable_named(
                    db,
                    scalar_field.parent_object_entity_name,
                    scalar_field.name,
                )
                .as_ref()
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                )
                .lookup(db);

                let raw_type = server_scalar_selectable.target_entity.clone();

                let inner_text = TCompilationProfile::TargetPlatform::get_inner_text_for_selectable(
                    db,
                    server_scalar_selectable.parent_entity_name.item,
                    server_scalar_selectable.name.item,
                );

                raw_response_type_inner.push_str(&format!(
                    "{indent}{name}{}: {},\n",
                    if server_scalar_selectable
                        .target_entity
                        .item
                        .as_ref()
                        .expect("Expected target entity to be valid.")
                        .is_nullable()
                    {
                        "?"
                    } else {
                        ""
                    },
                    print_javascript_type_declaration(
                        raw_type
                            .item
                            .as_ref()
                            .expect("Expected target entity to be valid.")
                            .reference(),
                        inner_text
                    )
                ));
            }
            MergedServerSelection::LinkedField(linked_field) => {
                let normalization_alias = linked_field.normalization_alias();
                let name = normalization_alias
                    .as_deref()
                    .unwrap_or(linked_field.name.lookup());

                let server_object_selectable = server_selectable_named(
                    db,
                    linked_field.parent_object_entity_name,
                    linked_field.name,
                )
                .as_ref()
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                )
                .lookup(db);

                let raw_type = server_object_selectable.target_entity.clone();

                let inner_text = {
                    let mut raw_response_type_declaration = String::new();
                    raw_response_type_declaration.push_str("{\n");
                    generate_raw_response_type_inner(
                        db,
                        &mut raw_response_type_declaration,
                        &linked_field.selection_map,
                        indentation_level + 1,
                    );
                    raw_response_type_declaration.push_str(&format!("{indent}}}"));
                    raw_response_type_declaration
                };

                raw_response_type_inner.push_str(&format!(
                    "{indent}{name}{}: {},\n",
                    if server_object_selectable
                        .target_entity
                        .item
                        .as_ref()
                        .expect("Expected target entity to be valid.")
                        .is_nullable()
                    {
                        "?"
                    } else {
                        ""
                    },
                    print_javascript_type_declaration(
                        raw_type
                            .item
                            .as_ref()
                            .expect("Expected target entity to be valid.")
                            .reference(),
                        inner_text
                    )
                ));
            }
            MergedServerSelection::ClientObjectSelectable(_) => {}
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
