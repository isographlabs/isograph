use common_lang_types::EntityName;
use intern::Lookup;
use isograph_schema::{
    CompilationProfile, IsographDatabase, MergedSelectionMap, MergedServerSelection,
    TargetPlatform, flattened_selectable_named,
};
use prelude::Postfix;
use std::collections::BTreeMap;

use crate::generate_artifacts::print_javascript_type_declaration;

pub fn generate_raw_response_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) -> String {
    let indent = &"  ".repeat(indentation_level as usize).to_string();

    let mut raw_response_type = String::new();
    raw_response_type.push_str(&format!("{}{{\n", indent));
    generate_raw_response_type_inner(
        db,
        &mut raw_response_type,
        parent_object_entity_name,
        selection_map,
        indentation_level + 1,
    );
    raw_response_type.push_str(&format!("{}}}\n", indent));
    raw_response_type
}

pub fn generate_raw_response_type_inner<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    raw_response_type: &mut String,
    parent_object_entity_name: EntityName,
    selection_map: &MergedSelectionMap,
    indentation_level: u8,
) {
    let indent = &"  ".repeat(indentation_level as usize).to_string();
    let mut inline_fragments = BTreeMap::new();
    let mut rest = BTreeMap::new();

    for (key, value) in selection_map.iter() {
        match value.reference() {
            MergedServerSelection::InlineFragment(inline_fragment) => {
                inline_fragments.insert(inline_fragment.type_to_refine_to, inline_fragment);
            }
            _ => {
                rest.insert(key.clone(), value.clone());
            }
        }
    }

    if inline_fragments.is_empty() {
        for item in rest.values() {
            match item.reference() {
                MergedServerSelection::ScalarField(scalar_field) => {
                    let normalization_alias = scalar_field.normalization_alias();
                    let name = normalization_alias
                        .as_deref()
                        .unwrap_or(scalar_field.name.lookup());

                    let server_scalar_selectable = flattened_selectable_named(
                        db,
                        parent_object_entity_name,
                        scalar_field.name,
                    )
                    .as_ref()
                    .expect(
                        "Expected selectable to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .lookup(db);

                    let raw_type = server_scalar_selectable.target_entity.clone();

                    let inner_text =
                        TCompilationProfile::TargetPlatform::get_inner_text_for_selectable(
                            db,
                            server_scalar_selectable.parent_entity_name.item,
                            server_scalar_selectable.name.item,
                        );

                    raw_response_type.push_str(&format!(
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

                    let server_object_selectable = flattened_selectable_named(
                        db,
                        parent_object_entity_name,
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
                        let nested_parent_object_entity_name = server_object_selectable
                            .target_entity
                            .item
                            .as_ref()
                            .expect("Expected target entity to be valid.")
                            .inner()
                            .0;

                        let mut raw_response_type_declaration = String::new();
                        raw_response_type_declaration.push_str("{\n");
                        generate_raw_response_type_inner(
                            db,
                            &mut raw_response_type_declaration,
                            nested_parent_object_entity_name,
                            &linked_field.selection_map,
                            indentation_level + 1,
                        );
                        raw_response_type_declaration.push_str(&format!("{indent}}}"));
                        raw_response_type_declaration
                    };

                    raw_response_type.push_str(&format!(
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
                _ => {}
            }
        }

        return;
    }

    let mut fragments = BTreeMap::new();
    for (type_to_refine_to, inline_fragment) in inline_fragments {
        let mut combined_selection_map = rest.clone();
        combined_selection_map.extend(inline_fragment.selection_map.clone());

        let mut fragment_raw_response_type = String::new();
        generate_raw_response_type_inner(
            db,
            &mut fragment_raw_response_type,
            type_to_refine_to,
            &combined_selection_map,
            indentation_level,
        );
        fragments.insert(type_to_refine_to, fragment_raw_response_type);
    }

    let indent = &"  ".repeat((indentation_level - 1) as usize).to_string();

    let mut iter = fragments.into_iter();
    if let Some((_, fragment)) = iter.next() {
        raw_response_type.push_str(&fragment);
    }

    for (_, fragment) in iter {
        raw_response_type.push_str(&format!("{indent}}} | {{\n"));
        raw_response_type.push_str(&fragment);
    }
}
