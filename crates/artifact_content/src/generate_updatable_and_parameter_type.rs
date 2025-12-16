use std::collections::BTreeSet;

use common_lang_types::{
    EntityName, ParentObjectEntityNameAndSelectableName, SelectableName, SelectableNameOrAlias,
    WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};
use intern::Lookup;
use isograph_lang_types::{
    DefinitionLocation, Description, ObjectSelectionDirectiveSet, ScalarSelection,
    ScalarSelectionDirectiveSet, SelectionFieldArgument, SelectionSet, SelectionType,
    TypeAnnotation,
};
use isograph_schema::{
    ClientFieldVariant, ClientScalarOrObjectSelectable, IsographDatabase, LINK_FIELD_NAME,
    NetworkProtocol, ObjectSelectableId, ScalarSelectableId, ServerEntityName, ValidatedSelection,
    ValidatedVariableDefinition, client_scalar_selectable_named, description,
    output_type_annotation, selectable_named, server_scalar_entity_javascript_name,
};
use prelude::Postfix;

use crate::{
    generate_artifacts::{
        ClientScalarSelectableParameterType, ClientScalarSelectableUpdatableDataType,
        print_javascript_type_declaration,
    },
    import_statements::{ParamTypeImports, UpdatableImports},
};

pub(crate) fn generate_client_selectable_parameter_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: EntityName,
    selection_map: &WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>,
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

fn write_param_type_from_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: EntityName,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    nested_client_scalar_selectable_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) {
    let selectable = selectable_named(db, parent_object_entity_name, selection.item.name())
        .as_ref()
        .expect("Expected parsing to have succeeded. This is indicative of a bug in Isograph.")
        .expect("Expected selectable to exist. This is indicative of a bug in Isograph.");
    match &selection.item {
        SelectionType::Scalar(scalar_field_selection) => {
            let scalar_selectable = selectable.as_scalar().expect(
                "Expected selectable to be a scalar. \
                This is indicative of a bug in Isograph.",
            );

            match scalar_selectable {
                DefinitionLocation::Server(server_scalar_selectable) => {
                    let server_scalar_selectable = server_scalar_selectable.lookup(db);
                    write_optional_description(
                        server_scalar_selectable.description,
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias = scalar_field_selection.name_or_alias().item;

                    let output_type = server_scalar_selectable.target_scalar_entity.as_ref().map(
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

                    query_type_declaration.push_str(&format!(
                        "{}readonly {}: {},\n",
                        "  ".repeat(indentation_level as usize),
                        name_or_alias,
                        print_javascript_type_declaration(&output_type)
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
            let object_selectable = selectable.as_object().expect(
                "Expected selectable to be an object. \
                This is indicative of a bug in Isograph.",
            );

            write_optional_description(
                description(db, object_selectable),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = object_selection.name_or_alias().item;

            let new_parent_object_entity_name = match object_selectable {
                DefinitionLocation::Server(s) => s.lookup(db).target_object_entity.inner(),
                DefinitionLocation::Client(c) => c.lookup(db).target_object_entity_name.inner(),
            }
            .dereference();

            let type_annotation =
                output_type_annotation(db, object_selectable)
                    .clone()
                    .map(&mut |_| {
                        generate_client_selectable_parameter_type(
                            db,
                            new_parent_object_entity_name,
                            &object_selection.selection_set,
                            nested_client_scalar_selectable_imports,
                            loadable_fields,
                            indentation_level,
                        )
                    });

            query_type_declaration.push_str(&format!(
                "readonly {}: {},\n",
                name_or_alias,
                match object_selectable {
                    DefinitionLocation::Client(client_object_selectable) => {
                        let client_object_selectable = client_object_selectable.lookup(db);
                        loadable_fields.insert(client_object_selectable.type_and_field());

                        print_javascript_type_declaration(&type_annotation.map(&mut |target| {
                            format!(
                                "LoadableField<{}__param, {target}>",
                                client_object_selectable
                                    .type_and_field()
                                    .underscore_separated(),
                            )
                        }))
                    }
                    DefinitionLocation::Server(_) =>
                        print_javascript_type_declaration(&type_annotation),
                }
            ));
        }
    }
}

pub(crate) fn generate_client_selectable_updatable_data_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: EntityName,
    selection_map: &WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>,
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
fn write_updatable_data_type_from_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: EntityName,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
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

    match &selection.item {
        SelectionType::Scalar(scalar_selection) => {
            match selectable.as_scalar().expect(
                "Expected selectable to be a scalar. \
                This is indicative of a bug in Isograph.",
            ) {
                DefinitionLocation::Server(server_scalar_selectable) => {
                    let server_scalar_selectable = server_scalar_selectable.lookup(db);
                    write_optional_description(
                        server_scalar_selectable.description,
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias = selection.item.name_or_alias().item;

                    let output_type = server_scalar_selectable.target_scalar_entity.clone().map(
                        &mut |scalar_entity_name| {
                            server_scalar_entity_javascript_name(db, scalar_entity_name)
                                .as_ref()
                                .expect(
                                    "Expected parsing to not have failed. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .expect(
                                    "Expected entity to exist. \
                                    This is indicative of a bug in Isograph.",
                                )
                        },
                    );

                    if selection.item.is_updatable() {
                        *updatable_fields = true;
                        query_type_declaration
                            .push_str(&"  ".repeat(indentation_level as usize).to_string());
                        query_type_declaration.push_str(&format!(
                            "{}: {},\n",
                            name_or_alias,
                            print_javascript_type_declaration(&output_type)
                        ));
                    } else {
                        query_type_declaration.push_str(&format!(
                            "{}readonly {}: {},\n",
                            "  ".repeat(indentation_level as usize),
                            name_or_alias,
                            print_javascript_type_declaration(&output_type)
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
            let object_selectable = selectable.as_object().expect(
                "Expected selectable to be object. \
                This is indicative of a bug in Isograph.",
            );

            write_optional_description(
                description(db, object_selectable),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = object_selection.name_or_alias().item;

            let new_parent_object_entity_name = match object_selectable {
                DefinitionLocation::Server(s) => s.lookup(db).target_object_entity.inner(),
                DefinitionLocation::Client(c) => c.lookup(db).target_object_entity_name.inner(),
            }
            .dereference();

            let type_annotation =
                output_type_annotation(db, object_selectable)
                    .clone()
                    .map(&mut |_| {
                        generate_client_selectable_updatable_data_type(
                            db,
                            new_parent_object_entity_name,
                            &object_selection.selection_set,
                            nested_client_scalar_selectable_imports,
                            loadable_fields,
                            indentation_level,
                            updatable_fields,
                        )
                    });

            match object_selection.object_selection_directive_set {
                ObjectSelectionDirectiveSet::Updatable(_) => {
                    *updatable_fields = true;
                    write_getter_and_setter(
                        query_type_declaration,
                        indentation_level,
                        name_or_alias,
                        output_type_annotation(db, object_selectable),
                        &type_annotation,
                    );
                }
                ObjectSelectionDirectiveSet::None(_) => {
                    query_type_declaration.push_str(&format!(
                        "readonly {}: {},\n",
                        name_or_alias,
                        print_javascript_type_declaration(&type_annotation),
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
    output_type_annotation: &TypeAnnotation<EntityName>,
    type_annotation: &TypeAnnotation<ClientScalarSelectableUpdatableDataType>,
) {
    query_type_declaration.push_str(&format!(
        "get {}(): {},\n",
        name_or_alias,
        print_javascript_type_declaration(type_annotation),
    ));
    let setter_type_annotation =
        output_type_annotation
            .clone()
            .map(&mut |server_object_entity_name| {
                let link_field_name = *LINK_FIELD_NAME;
                format!("{{ {link_field_name}: {server_object_entity_name}__{link_field_name}__output_type }}")
            });
    query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
    query_type_declaration.push_str(&format!(
        "set {}(value: {}),\n",
        name_or_alias,
        print_javascript_type_declaration(&setter_type_annotation),
    ));
}

#[expect(clippy::too_many_arguments)]
fn write_param_type_from_client_scalar_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    query_type_declaration: &mut String,
    nested_client_scalar_selectable_imports: &mut BTreeSet<ParentObjectEntityNameAndSelectableName>,
    loadable_fields: &mut BTreeSet<ParentObjectEntityNameAndSelectableName>,
    indentation_level: u8,
    scalar_selection: &ScalarSelection<ScalarSelectableId>,
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
        client_scalar_selectable.description,
        query_type_declaration,
        indentation_level,
    );
    query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
    match client_scalar_selectable.variant {
        ClientFieldVariant::Link
        | ClientFieldVariant::UserWritten(_)
        | ClientFieldVariant::ImperativelyLoadedField(_) => {
            nested_client_scalar_selectable_imports
                .insert(client_scalar_selectable.type_and_field());
            let inner_output_type = format!(
                "{}__output_type",
                client_scalar_selectable
                    .type_and_field()
                    .underscore_separated()
            );
            let output_type = match scalar_selection.scalar_selection_directive_set {
                ScalarSelectionDirectiveSet::Updatable(_)
                | ScalarSelectionDirectiveSet::None(_) => inner_output_type,
                ScalarSelectionDirectiveSet::Loadable(_) => {
                    loadable_fields.insert(client_scalar_selectable.type_and_field());
                    let provided_arguments = get_provided_arguments(
                        client_scalar_selectable
                            .variable_definitions
                            .iter()
                            .map(|x| &x.item),
                        &scalar_selection.arguments,
                    );

                    let indent = "  ".repeat((indentation_level + 1) as usize);
                    let provided_args_type = if provided_arguments.is_empty() {
                        "".to_string()
                    } else {
                        format!(
                            ",\n{indent}Omit<ExtractParameters<{}__param>, keyof {}>",
                            client_scalar_selectable
                                .type_and_field()
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
                            .type_and_field()
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

fn get_loadable_field_type_from_arguments<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    arguments: Vec<ValidatedVariableDefinition>,
) -> String {
    let mut loadable_field_type = "{".to_string();
    let mut is_first = true;
    for arg in arguments.iter() {
        if !is_first {
            loadable_field_type.push_str(", ");
        }
        is_first = false;
        let is_optional = !matches!(arg.type_, GraphQLTypeAnnotation::NonNull(_));
        loadable_field_type.push_str(&format!(
            "readonly {}{}: {}",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_type_for_js(db, arg.type_.clone())
        ));
    }
    loadable_field_type.push('}');
    loadable_field_type
}

fn format_type_for_js<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    type_: GraphQLTypeAnnotation<ServerEntityName>,
) -> String {
    let new_type = type_.map(
        |selectable_server_field_id| match selectable_server_field_id {
            ServerEntityName::Object(_) => {
                panic!(
                    "Unexpected object. Objects are not supported as parameters, yet. \
                    This is indicative of an unimplemented feature in Isograph."
                )
            }
            ServerEntityName::Scalar(scalar_entity_name) => {
                server_scalar_entity_javascript_name(db, scalar_entity_name)
                    .as_ref()
                    .expect(
                        "Expected parsing to not have failed. \
                        This is indicative of a bug in Isograph.",
                    )
                    .expect(
                        "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                    )
            }
        },
    );

    format_type_for_js_inner(new_type)
}

fn format_type_for_js_inner(
    new_type: GraphQLTypeAnnotation<common_lang_types::JavascriptName>,
) -> String {
    match new_type {
        GraphQLTypeAnnotation::Named(named_inner_type) => {
            format!("{} | null | void", named_inner_type.0.item)
        }
        GraphQLTypeAnnotation::List(list) => {
            format!("ReadonlyArray<{}> | null", format_type_for_js_inner(list.0))
        }
        GraphQLTypeAnnotation::NonNull(non_null) => match *non_null {
            GraphQLNonNullTypeAnnotation::Named(named_inner_type) => {
                named_inner_type.0.item.to_string()
            }
            GraphQLNonNullTypeAnnotation::List(list) => {
                format!("ReadonlyArray<{}>", format_type_for_js_inner(list.0))
            }
        },
    }
}

fn get_provided_arguments<'a>(
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> Vec<ValidatedVariableDefinition> {
    argument_definitions
        .filter_map(|definition| {
            let user_has_supplied_argument = arguments
                .iter()
                .any(|arg| definition.name.item == arg.item.name.item);
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
