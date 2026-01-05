use std::{collections::BTreeSet, fmt::Display};

use common_lang_types::{
    EntityName, EntityNameAndSelectableName, SelectableName, SelectableNameOrAlias,
    WithEmbeddedLocation, WithGenericLocation,
};
use intern::Lookup;
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, Description, ObjectSelectionDirectiveSet,
    ScalarSelection, ScalarSelectionDirectiveSet, Selection, SelectionFieldArgument, SelectionSet,
    SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};
use isograph_schema::{
    ClientFieldVariant, CompilationProfile, IsographDatabase, LINK_FIELD_NAME, TargetPlatform,
    client_scalar_selectable_named, description, output_type_annotation, selectable_named,
};
use prelude::Postfix;

use crate::{
    format_parameter_type::format_parameter_type,
    generate_artifacts::{
        ClientScalarSelectableParameterType, ClientScalarSelectableUpdatableDataType,
        print_javascript_type_declaration,
    },
    import_statements::{ParamTypeImports, UpdatableImports},
};

pub(crate) fn generate_client_selectable_parameter_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection_map: &WithEmbeddedLocation<SelectionSet>,
    nested_client_scalar_selectable_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) -> ClientScalarSelectableParameterType {
    // TODO use unwraps
    let mut client_scalar_selectable_parameter_type = "{\n".to_string();

    for selection in selection_map.item.selections.iter() {
        write_param_type_from_selection(
            db,
            parent_object_entity_name,
            &mut client_scalar_selectable_parameter_type,
            selection,
            nested_client_scalar_selectable_imports,
            loadable_fields,
            indentation_level + 1,
        );
    }
    client_scalar_selectable_parameter_type
        .push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientScalarSelectableParameterType(client_scalar_selectable_parameter_type)
}

fn write_param_type_from_selection<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    query_type_declaration: &mut String,
    selection: &WithEmbeddedLocation<Selection>,
    nested_client_scalar_selectable_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) {
    let selectable = selectable_named(db, parent_object_entity_name, selection.item.name())
        .as_ref()
        .expect("Expected parsing to have succeeded. This is indicative of a bug in Isograph.")
        .expect("Expected selectable to exist. This is indicative of a bug in Isograph.");
    match selection.item.reference() {
        SelectionType::Scalar(scalar_field_selection) => {
            let scalar_selectable = match selectable {
                DefinitionLocation::Server(s) => match s.lookup(db).is_inline_fragment.reference() {
                    SelectionType::Scalar(_) => s.server_defined(),
                    SelectionType::Object(_) => {
                        panic!("Expected selectable to be a scalar.")
                    }
                },
                DefinitionLocation::Client(c) => match c {
                    SelectionType::Scalar(s) => s.client_defined(),
                    SelectionType::Object(_) => panic!("Expected selectable to be a scalar."),
                },
            };

            match scalar_selectable {
                DefinitionLocation::Server(server_scalar_selectable) => {
                    let server_scalar_selectable = server_scalar_selectable.lookup(db);
                    write_optional_description(
                        server_scalar_selectable
                            .description
                            .map(WithGenericLocation::item),
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias: SelectableNameOrAlias =
                        (*scalar_field_selection).name_or_alias().item;

                    let inner_text =
                        TCompilationProfile::TargetPlatform::get_inner_text_for_selectable(
                            db,
                            server_scalar_selectable.parent_entity_name,
                            server_scalar_selectable.name,
                        );

                    query_type_declaration.push_str(&format!(
                        "{}readonly {}: {},\n",
                        "  ".repeat(indentation_level as usize),
                        name_or_alias,
                        print_javascript_type_declaration(
                            server_scalar_selectable.target_entity_name.reference(),
                            inner_text
                        )
                    ));
                }
                DefinitionLocation::Client(_) => write_param_type_from_client_scalar_selectable(
                    db,
                    query_type_declaration,
                    nested_client_scalar_selectable_imports,
                    loadable_fields,
                    indentation_level,
                    scalar_field_selection,
                    parent_object_entity_name,
                    scalar_field_selection.name.item,
                ),
            }
        }
        SelectionType::Object(object_selection) => {
            let object_selectable = match selectable {
                DefinitionLocation::Server(s) => match s.lookup(db).is_inline_fragment.reference() {
                    SelectionType::Scalar(_) => {
                        panic!("Expected selectable to be an object.")
                    }
                    SelectionType::Object(_) => s.server_defined(),
                },
                DefinitionLocation::Client(c) => c
                    .as_object()
                    .expect("Expected selectable to be an object.")
                    .client_defined(),
            };

            write_optional_description(
                description(db, object_selectable),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = (*object_selection).name_or_alias().item;

            let new_parent_object_entity_name = match object_selectable {
                DefinitionLocation::Server(s) => s.lookup(db).target_entity_name.inner(),
                DefinitionLocation::Client(c) => c.lookup(db).target_entity_name.inner(),
            }
            .0;

            let type_annotation = output_type_annotation(db, object_selectable);

            query_type_declaration.push_str(&format!(
                "readonly {}: {},\n",
                name_or_alias,
                match object_selectable {
                    DefinitionLocation::Client(client_object_selectable) => {
                        let client_object_selectable = client_object_selectable.lookup(db);
                        loadable_fields
                            .insert(client_object_selectable.entity_name_and_selectable_name());

                        let inner = format!(
                            "LoadableField<{}__param, {}>",
                            client_object_selectable
                                .entity_name_and_selectable_name()
                                .underscore_separated(),
                            generate_client_selectable_parameter_type(
                                db,
                                new_parent_object_entity_name,
                                &object_selection.selection_set,
                                nested_client_scalar_selectable_imports,
                                loadable_fields,
                                indentation_level,
                            )
                        );

                        print_javascript_type_declaration(type_annotation, inner)
                    }
                    DefinitionLocation::Server(_) => {
                        let inner_text = generate_client_selectable_parameter_type(
                            db,
                            new_parent_object_entity_name,
                            &object_selection.selection_set,
                            nested_client_scalar_selectable_imports,
                            loadable_fields,
                            indentation_level,
                        );
                        print_javascript_type_declaration(type_annotation, inner_text)
                    }
                }
            ));
        }
    }
}

pub(crate) fn generate_client_selectable_updatable_data_type<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection_map: &WithEmbeddedLocation<SelectionSet>,
    nested_client_scalar_selectable_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    updatable_fields: &mut UpdatableImports,
) -> ClientScalarSelectableUpdatableDataType {
    // TODO use unwraps

    let mut client_scalar_selectable_updatable_data_type = "{\n".to_string();

    for selection in selection_map.item.selections.iter() {
        write_updatable_data_type_from_selection(
            db,
            parent_object_entity_name,
            &mut client_scalar_selectable_updatable_data_type,
            selection,
            nested_client_scalar_selectable_imports,
            loadable_fields,
            indentation_level + 1,
            updatable_fields,
        );
    }

    client_scalar_selectable_updatable_data_type
        .push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientScalarSelectableUpdatableDataType(client_scalar_selectable_updatable_data_type)
}

#[expect(clippy::too_many_arguments)]
fn write_updatable_data_type_from_selection<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    query_type_declaration: &mut String,
    selection: &WithEmbeddedLocation<Selection>,
    nested_client_scalar_selectable_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    updatable_fields: &mut UpdatableImports,
) {
    let selectable = selectable_named(db, parent_object_entity_name, selection.item.name())
        .as_ref()
        .expect(
            "Expected validation to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    match selection.item.reference() {
        SelectionType::Scalar(scalar_selection) => {
            let scalar_selectable = match selectable {
                DefinitionLocation::Server(s) => match s.lookup(db).is_inline_fragment.reference() {
                    SelectionType::Scalar(_) => s.server_defined(),
                    SelectionType::Object(_) => panic!("Expected selectable to be a scalar."),
                },
                DefinitionLocation::Client(c) => c
                    .as_scalar()
                    .expect("Expected selectable to be a scalar")
                    .client_defined(),
            };

            match scalar_selectable {
                DefinitionLocation::Server(server_scalar_selectable) => {
                    let server_scalar_selectable = server_scalar_selectable.lookup(db);
                    write_optional_description(
                        server_scalar_selectable
                            .description
                            .map(WithGenericLocation::item),
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias = selection.item.name_or_alias().item;

                    let output_type = server_scalar_selectable.target_entity_name.clone();

                    let inner_text =
                        TCompilationProfile::TargetPlatform::get_inner_text_for_selectable(
                            db,
                            server_scalar_selectable.parent_entity_name,
                            server_scalar_selectable.name,
                        );

                    if selection.item.is_updatable() {
                        *updatable_fields = true;
                        query_type_declaration
                            .push_str(&"  ".repeat(indentation_level as usize).to_string());
                        query_type_declaration.push_str(&format!(
                            "{}: {},\n",
                            name_or_alias,
                            print_javascript_type_declaration(output_type.reference(), inner_text)
                        ));
                    } else {
                        query_type_declaration.push_str(&format!(
                            "{}readonly {}: {},\n",
                            "  ".repeat(indentation_level as usize),
                            name_or_alias,
                            print_javascript_type_declaration(output_type.reference(), inner_text)
                        ));
                    }
                }
                DefinitionLocation::Client(_) => {
                    write_param_type_from_client_scalar_selectable(
                        db,
                        query_type_declaration,
                        nested_client_scalar_selectable_imports,
                        loadable_fields,
                        indentation_level,
                        scalar_selection,
                        parent_object_entity_name,
                        selection.item.name(),
                    );
                }
            }
        }
        SelectionType::Object(object_selection) => {
            let object_selectable = match selectable {
                DefinitionLocation::Server(s) => match s.lookup(db).is_inline_fragment.reference() {
                    SelectionType::Scalar(_) => {
                        panic!("Expected selectable to be object selectable")
                    }
                    SelectionType::Object(_) => s.server_defined(),
                },
                DefinitionLocation::Client(c) => c
                    .as_object()
                    .expect("Expected selectable to be object")
                    .client_defined(),
            };

            write_optional_description(
                description(db, object_selectable),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = (*object_selection).name_or_alias().item;

            let type_annotation = output_type_annotation(db, object_selectable).clone();

            let new_parent_object_entity_name = match object_selectable {
                DefinitionLocation::Server(s) => s.lookup(db).target_entity_name.inner(),
                DefinitionLocation::Client(c) => c.lookup(db).target_entity_name.inner(),
            }
            .0;
            let inner_text = generate_client_selectable_updatable_data_type(
                db,
                new_parent_object_entity_name,
                &object_selection.selection_set,
                nested_client_scalar_selectable_imports,
                loadable_fields,
                indentation_level,
                updatable_fields,
            );

            match object_selection.object_selection_directive_set {
                ObjectSelectionDirectiveSet::Updatable(_) => {
                    *updatable_fields = true;
                    write_getter_and_setter(
                        query_type_declaration,
                        indentation_level,
                        name_or_alias,
                        output_type_annotation(db, object_selectable),
                        &type_annotation,
                        inner_text,
                    );
                }
                ObjectSelectionDirectiveSet::None(_) => {
                    query_type_declaration.push_str(&format!(
                        "readonly {}: {},\n",
                        name_or_alias,
                        print_javascript_type_declaration(&type_annotation, inner_text),
                    ));
                }
            }
        }
    }
}

fn write_getter_and_setter(
    query_type_declaration: &mut String,
    indentation_level: u8,
    name_or_alias: SelectableNameOrAlias,
    output_type_annotation: &TypeAnnotationDeclaration,
    type_annotation: &TypeAnnotationDeclaration,
    getter_inner_text: impl Display,
) {
    query_type_declaration.push_str(&format!(
        "get {}(): {},\n",
        name_or_alias,
        print_javascript_type_declaration(type_annotation, getter_inner_text),
    ));
    let setter_type_annotation = output_type_annotation.clone();
    query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());

    let link_field_name = *LINK_FIELD_NAME;
    let setter_inner_text = format!(
        "{{ {link_field_name}: {}__{link_field_name}__output_type }}",
        type_annotation.inner()
    );

    query_type_declaration.push_str(&format!(
        "set {}(value: {}),\n",
        name_or_alias,
        print_javascript_type_declaration(&setter_type_annotation, setter_inner_text),
    ));
}

#[expect(clippy::too_many_arguments)]
fn write_param_type_from_client_scalar_selectable<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    query_type_declaration: &mut String,
    nested_client_scalar_selectable_imports: &mut BTreeSet<EntityNameAndSelectableName>,
    loadable_fields: &mut BTreeSet<EntityNameAndSelectableName>,
    indentation_level: u8,
    scalar_selection: &ScalarSelection,
    parent_object_entity_name: EntityName,
    client_scalar_selectable_name: SelectableName,
) {
    let client_scalar_selectable = client_scalar_selectable_named(
        db,
        parent_object_entity_name,
        client_scalar_selectable_name,
    )
    .as_ref()
    .expect(
        "Expected parsing to have succeeded by this point. \
        This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect(
        "Expected selectable to exist. \
        This is indicative of a bug in Isograph.",
    )
    .lookup(db);

    write_optional_description(
        client_scalar_selectable
            .description
            .map(WithGenericLocation::item),
        query_type_declaration,
        indentation_level,
    );
    query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
    match client_scalar_selectable.variant {
        ClientFieldVariant::Link
        | ClientFieldVariant::UserWritten(_)
        | ClientFieldVariant::ImperativelyLoadedField(_) => {
            nested_client_scalar_selectable_imports
                .insert(client_scalar_selectable.entity_name_and_selectable_name());
            let inner_output_type = format!(
                "{}__output_type",
                client_scalar_selectable
                    .entity_name_and_selectable_name()
                    .underscore_separated()
            );
            let output_type = match scalar_selection.scalar_selection_directive_set {
                ScalarSelectionDirectiveSet::Updatable(_)
                | ScalarSelectionDirectiveSet::None(_) => inner_output_type,
                ScalarSelectionDirectiveSet::Loadable(_) => {
                    loadable_fields
                        .insert(client_scalar_selectable.entity_name_and_selectable_name());
                    let provided_arguments = get_provided_arguments(
                        client_scalar_selectable.variable_definitions.iter(),
                        &scalar_selection.arguments,
                    );

                    let indent = "  ".repeat((indentation_level + 1) as usize);
                    let provided_args_type = if provided_arguments.is_empty() {
                        "".to_string()
                    } else {
                        format!(
                            ",\n{indent}Omit<ExtractParameters<{}__param>, keyof {}>",
                            client_scalar_selectable
                                .entity_name_and_selectable_name()
                                .underscore_separated(),
                            get_loadable_field_type_from_arguments(db, provided_arguments)
                        )
                    };

                    format!(
                        "LoadableField<\n\
                        {indent}{}__param,\n\
                        {indent}{inner_output_type}\
                        {provided_args_type}\n\
                        {}>",
                        client_scalar_selectable
                            .entity_name_and_selectable_name()
                            .underscore_separated(),
                        "  ".repeat(indentation_level as usize),
                    )
                }
            };
            query_type_declaration.push_str(
                &(format!(
                    "readonly {}: {},\n",
                    scalar_selection.name_or_alias().item,
                    output_type
                )),
            );
        }
    }
}

fn get_loadable_field_type_from_arguments<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    arguments: Vec<VariableDeclaration>,
) -> String {
    let mut loadable_field_type = "{".to_string();
    let mut is_first = true;
    for arg in arguments.iter() {
        if !is_first {
            loadable_field_type.push_str(", ");
        }
        is_first = false;
        let is_optional = arg.type_.item.is_nullable();
        loadable_field_type.push_str(&format!(
            "readonly {}{}: {}",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_parameter_type(db, arg.type_.item.reference(), 1)
        ));
    }
    loadable_field_type.push('}');
    loadable_field_type
}

fn get_provided_arguments<'a>(
    argument_definitions: impl Iterator<Item = &'a VariableDeclaration> + 'a,
    arguments: &[WithEmbeddedLocation<SelectionFieldArgument>],
) -> Vec<VariableDeclaration> {
    argument_definitions
        .filter_map(|definition| {
            let user_has_supplied_argument = arguments
                .iter()
                .any(|arg| definition.name.item.0 == arg.item.name.item);
            if user_has_supplied_argument {
                definition.clone().wrap_some()
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn write_optional_description(
    description: Option<Description>,
    query_type_declaration: &mut String,
    indentation_level: u8,
) {
    if let Some(description) = description {
        query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
        query_type_declaration.push_str("/**\n");
        query_type_declaration.push_str(description.lookup());
        query_type_declaration.push('\n');
        query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
        query_type_declaration.push_str("*/\n");
    }
}
