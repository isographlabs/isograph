use crate::{
    ClientFieldDeclarationPath, ClientObjectSelectableNameWrapperPath,
    ClientPointerDeclarationPath, ClientScalarSelectableNameWrapperPath, DescriptionPath,
    EntrypointDeclarationPath, ObjectSelectionPath, ScalarSelectionPath, SelectionParentType,
    ServerObjectEntityNameWrapperPath,
};

#[derive(Debug)]
pub enum IsographResolvedNode<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ServerObjectEntityNameWrapper(ServerObjectEntityNameWrapperPath<'a>),
    Description(DescriptionPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ScalarSelection(ScalarSelectionPath<'a>),
    ObjectSelection(ObjectSelectionPath<'a>),
    ClientScalarSelectableNameWrapper(ClientScalarSelectableNameWrapperPath<'a>),
    ClientObjectSelectableNameWrapper(ClientObjectSelectableNameWrapperPath<'a>),
}

// TODO remove this, this is just a demonstration.
pub fn get_path_to_root_from_scalar<'a>(scalar_path: &ScalarSelectionPath<'a>) -> Vec<String> {
    let mut string_vec = vec![scalar_path.inner.name.item.to_string()];
    get_path_using_selection_parent(&scalar_path.parent, &mut string_vec);
    string_vec.reverse();
    string_vec
}

pub fn get_path_to_root_from_object<'a>(object_path: &ObjectSelectionPath<'a>) -> Vec<String> {
    let mut string_vec = vec![object_path.inner.name.item.to_string()];
    get_path_using_selection_parent(&object_path.parent, &mut string_vec);
    string_vec.reverse();
    string_vec
}

fn get_path_using_selection_parent<'a>(
    selection_parent: &SelectionParentType<'a>,
    string_vec: &mut Vec<String>,
) {
    match &selection_parent {
        crate::SelectionParentType::ObjectSelection(object_path) => {
            string_vec.push(object_path.inner.name.item.to_string());
            get_path_using_selection_parent(&object_path.parent, string_vec);
        }
        crate::SelectionParentType::ClientFieldDeclaration(client_field_declaration) => {
            string_vec.push(format!(
                "{}.{}",
                client_field_declaration.inner.parent_type,
                client_field_declaration.inner.client_field_name
            ));
        }
        crate::SelectionParentType::ClientPointerDeclaration(client_pointer_declaration) => {
            string_vec.push(format!(
                "{}.{}",
                client_pointer_declaration.inner.parent_type,
                client_pointer_declaration.inner.client_pointer_name
            ));
        }
    };
}
