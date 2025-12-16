use common_lang_types::{
    Diagnostic, EntityName, IsographCodeAction, Location, ParentObjectEntityNameAndSelectableName,
    SelectableName,
};
use isograph_lang_types::SelectionType;
use prelude::Postfix;

pub fn get_all_errors_or_all_ok<T, E>(
    items: impl Iterator<Item = Result<T, Vec<E>>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut oks = vec![];
    let mut errors = vec![];

    for item in items {
        match item {
            Ok(ok) => oks.push(ok),
            Err(e) => errors.extend(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

#[expect(unused)]
fn selection_does_not_exist_diagnostic(
    client_type: &str,
    declaration_parent_object_entity_name: EntityName,
    declaration_selectable_name: SelectableName,
    selectable_parent_object_entity_name: EntityName,
    selectable_name: SelectableName,
    location: Location,
    selection_type: SelectionType<(), ()>,
) -> Diagnostic {
    Diagnostic::new_with_code_actions(
        format!(
            "In the client {client_type} `{declaration_parent_object_entity_name}.{declaration_selectable_name}`, \
            the field `{selectable_parent_object_entity_name}.{selectable_name}` is selected, but that \
            field does not exist on `{selectable_parent_object_entity_name}`"
        ),
        location.wrap_some(),
        match selection_type {
            SelectionType::Scalar(_) => IsographCodeAction::CreateNewScalarSelectable(
                ParentObjectEntityNameAndSelectableName {
                    parent_object_entity_name: selectable_parent_object_entity_name,
                    selectable_name,
                },
            ),
            SelectionType::Object(_) => IsographCodeAction::CreateNewObjectSelectable(
                ParentObjectEntityNameAndSelectableName {
                    parent_object_entity_name: selectable_parent_object_entity_name,
                    selectable_name,
                },
            ),
        }
        .wrap_vec(),
    )
}

#[expect(unused)]
#[expect(clippy::too_many_arguments)]
fn selection_wrong_selection_type_diagnostic(
    client_type: &str,
    declaration_entity_name: EntityName,
    declaration_selectable_name: SelectableName,
    selectable_entity_name: EntityName,
    selectable_name: SelectableName,
    selected_as: &str,
    proper_way_to_select: &str,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "In the client {client_type} \
            `{declaration_entity_name}.{declaration_selectable_name}`, \
            the field `{selectable_entity_name}.{selectable_name}` \
            is selected as {selected_as}. Instead, that field should be selected \
            as {proper_way_to_select}"
        ),
        location.wrap_some(),
    )
}
