use common_lang_types::{
    derive_display, ArtifactFileName, ArtifactFilePrefix, ArtifactPathAndContent, DescriptionValue,
    Location, ObjectTypeAndFieldName, SelectableNameOrAlias, Span, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation,
};
use intern::{string_key::Intern, Lookup};

use core::panic;
use isograph_config::CompilerConfig;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldId, DefinitionLocation, NonConstantValue,
    ObjectSelectionDirectiveSet, ScalarFieldSelection, ScalarSelectionDirectiveSet, SelectionType,
    SelectionTypeContainingSelections, ServerEntityId, ServerObjectId, TypeAnnotation,
    UnionVariant, VariableDefinition,
};
use isograph_schema::{
    accessible_client_fields, as_server_field, description, get_provided_arguments,
    output_type_annotation, selection_map_wrapped, ClientField, ClientFieldOrPointer,
    ClientFieldVariant, FieldTraversalResult, NameAndArguments, NormalizationKey, OutputFormat,
    RequiresRefinement, Schema, SchemaObject, SchemaServerLinkedFieldFieldVariant,
    UserWrittenClientTypeInfo, UserWrittenComponentVariant, ValidatedScalarSelectionAssociatedData,
    ValidatedSelection, ValidatedVariableDefinition,
};
use lazy_static::lazy_static;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::{Debug, Display},
};

use crate::{
    eager_reader_artifact::{
        generate_eager_reader_artifacts, generate_eager_reader_condition_artifact,
        generate_eager_reader_output_type_artifact, generate_eager_reader_param_type_artifact,
    },
    entrypoint_artifact::{
        generate_entrypoint_artifacts,
        generate_entrypoint_artifacts_with_client_field_traversal_result,
    },
    format_parameter_type::format_parameter_type,
    import_statements::{LinkImports, ParamTypeImports, UpdatableImports},
    iso_overload_file::build_iso_overload_artifact,
    refetch_reader_artifact::{
        generate_refetch_output_type_artifact, generate_refetch_reader_artifact,
    },
};

lazy_static! {
    pub static ref ENTRYPOINT_FILE_NAME: ArtifactFileName = "entrypoint.ts".intern().into();
    pub static ref ENTRYPOINT: ArtifactFilePrefix = "entrypoint".intern().into();
    pub static ref ISO_TS_FILE_NAME: ArtifactFileName = "iso.ts".intern().into();
    pub static ref ISO_TS: ArtifactFilePrefix = "iso".intern().into();
    pub static ref NORMALIZATION_AST_FILE_NAME: ArtifactFileName =
        "normalization_ast.ts".intern().into();
    pub static ref NORMALIZATION_AST: ArtifactFilePrefix = "normalization_ast".intern().into();
    pub static ref QUERY_TEXT_FILE_NAME: ArtifactFileName = "query_text.ts".intern().into();
    pub static ref QUERY_TEXT: ArtifactFilePrefix = "query_text".intern().into();
    pub static ref REFETCH_READER_FILE_NAME: ArtifactFileName = "refetch_reader.ts".intern().into();
    pub static ref REFETCH_READER: ArtifactFilePrefix = "refetch_reader".intern().into();
    pub static ref RESOLVER_OUTPUT_TYPE_FILE_NAME: ArtifactFileName =
        "output_type.ts".intern().into();
    pub static ref RESOLVER_OUTPUT_TYPE: ArtifactFilePrefix = "output_type".intern().into();
    pub static ref RESOLVER_PARAM_TYPE_FILE_NAME: ArtifactFileName =
        "param_type.ts".intern().into();
    pub static ref RESOLVER_PARAM_TYPE: ArtifactFilePrefix = "param_type".intern().into();
    pub static ref RESOLVER_PARAMETERS_TYPE_FILE_NAME: ArtifactFileName =
        "parameters_type.ts".intern().into();
    pub static ref RESOLVER_PARAMETERS_TYPE: ArtifactFilePrefix = "parameters_type".intern().into();
    pub static ref RESOLVER_READER_FILE_NAME: ArtifactFileName =
        "resolver_reader.ts".intern().into();
    pub static ref RESOLVER_READER: ArtifactFilePrefix = "resolver_reader".intern().into();
}

/// Get all artifacts according to the following scheme:
///
/// For each entrypoint, generate an entrypoint artifact. This involves
/// generating the merged selection map.
///
/// - While creating a client field's merged selection map, whenever we enter
///   a client field, we check a cache (`encountered_client_field_map`). If that
///   cache is empty, we populate it with the client field's merged selection
///   map, reachable variables, and paths to refetch fields.
/// - If that cache is full, we reuse the values in the cache.
/// - Using that value, we merge it the child field's selection map into the
///   parent's selection map, merge variables, etc.
///
/// For each field that we encounter in this way (including the entrypoint
/// itself), we must generate a resolver or refetch reader artifact. (Currently,
/// each field will have only one or the other; but once we have loadable fields,
/// the loadable field will have both a refetch and resolver reader artifact.)
///
/// Also, for each user-written resolver, we must generate a param_type artifact.
/// For each resolver that is reachable from a reader, we must also generate an
/// output_type artifact.
///
/// TODO this should go through OutputFormat
pub fn get_artifact_path_and_content<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    config: &CompilerConfig,
) -> Vec<ArtifactPathAndContent> {
    let mut artifact_path_and_content = get_artifact_path_and_content_impl(schema, config);
    if let Some(header) = config.options.generated_file_header {
        for artifact_path_and_content in artifact_path_and_content.iter_mut() {
            artifact_path_and_content.file_content =
                format!("// {header}\n{}", artifact_path_and_content.file_content);
        }
    }
    artifact_path_and_content
}

fn get_artifact_path_and_content_impl<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    config: &CompilerConfig,
) -> Vec<ArtifactPathAndContent> {
    let mut encountered_client_type_map = BTreeMap::new();
    let mut path_and_contents = vec![];
    let mut encountered_output_types = HashSet::<ClientFieldId>::new();

    // For each entrypoint, generate an entrypoint artifact and refetch artifacts
    for entrypoint_id in schema.entrypoints.keys() {
        let entrypoint_path_and_content = generate_entrypoint_artifacts(
            schema,
            *entrypoint_id,
            &mut encountered_client_type_map,
            config.options.include_file_extensions_in_import_statements,
        );
        path_and_contents.extend(entrypoint_path_and_content);

        // We also need to generate output types for entrypoints
        encountered_output_types.insert(*entrypoint_id);
    }

    for (
        encountered_field_id,
        FieldTraversalResult {
            traversal_state,
            merged_selection_map,
            was_ever_selected_loadably,
            ..
        },
    ) in &encountered_client_type_map
    {
        match encountered_field_id {
            DefinitionLocation::Server(encountered_server_field_id) => {
                let encountered_server_field = schema.server_field(*encountered_server_field_id);

                match &encountered_server_field.target_server_entity {
                    SelectionType::Scalar(_) => {}
                    SelectionType::Object((linked_field_variant, _)) => match &linked_field_variant
                    {
                        SchemaServerLinkedFieldFieldVariant::LinkedField => {}
                        SchemaServerLinkedFieldFieldVariant::InlineFragment(inline_fragment) => {
                            path_and_contents.push(generate_eager_reader_condition_artifact(
                                schema,
                                encountered_server_field,
                                inline_fragment,
                                &traversal_state.refetch_paths,
                                config.options.include_file_extensions_in_import_statements,
                            ));
                        }
                    },
                };
            }

            DefinitionLocation::Client(SelectionType::Object(encountered_client_pointer_id)) => {
                let encountered_client_pointer =
                    schema.client_pointer(*encountered_client_pointer_id);
                path_and_contents.extend(generate_eager_reader_artifacts(
                    schema,
                    &SelectionType::Object(encountered_client_pointer),
                    config,
                    UserWrittenClientTypeInfo {
                        const_export_name: encountered_client_pointer.info.const_export_name,
                        file_path: encountered_client_pointer.info.file_path,
                        user_written_component_variant: UserWrittenComponentVariant::Eager,
                    },
                    &traversal_state.refetch_paths,
                    config.options.include_file_extensions_in_import_statements,
                    traversal_state.has_updatable,
                ));
            }
            DefinitionLocation::Client(SelectionType::Scalar(encountered_client_field_id)) => {
                let encountered_client_field = schema.client_field(*encountered_client_field_id);

                match &encountered_client_field.variant {
                    ClientFieldVariant::Link => (),
                    ClientFieldVariant::UserWritten(info) => {
                        path_and_contents.extend(generate_eager_reader_artifacts(
                            schema,
                            &SelectionType::Scalar(encountered_client_field),
                            config,
                            *info,
                            &traversal_state.refetch_paths,
                            config.options.include_file_extensions_in_import_statements,
                            traversal_state.has_updatable,
                        ));

                        if *was_ever_selected_loadably {
                            path_and_contents.push(generate_refetch_reader_artifact(
                                schema,
                                encountered_client_field,
                                None,
                                &traversal_state.refetch_paths,
                                true,
                                config.options.include_file_extensions_in_import_statements,
                            ));

                            // Everything about this is quite sus
                            let id_arg = ArgumentKeyAndValue {
                                key: "id".intern().into(),
                                value: NonConstantValue::Variable("id".intern().into()),
                            };

                            let type_to_refine_to = schema
                                .server_field_data
                                .object(encountered_client_field.parent_object_id);

                            if schema
                                .fetchable_types
                                .contains_key(&encountered_client_field.parent_object_id)
                            {
                                panic!("Loadable fields on root objects are not yet supported");
                            }

                            let wrapped_map = selection_map_wrapped(
                                merged_selection_map.clone(),
                                "node".intern().into(),
                                vec![id_arg.clone()],
                                None,
                                None,
                                None,
                                RequiresRefinement::Yes(type_to_refine_to.name),
                            );
                            let id_var = ValidatedVariableDefinition {
                                name: WithLocation::new("id".intern().into(), Location::Generated),
                                type_: GraphQLTypeAnnotation::NonNull(Box::new(
                                    GraphQLNonNullTypeAnnotation::Named(
                                        GraphQLNamedTypeAnnotation(WithSpan::new(
                                            ServerEntityId::Scalar(
                                                schema.server_field_data.id_type_id,
                                            ),
                                            Span::todo_generated(),
                                        )),
                                    ),
                                )),
                                default_value: None,
                            };
                            let variable_definitions_iter = encountered_client_field
                                .variable_definitions
                                .iter()
                                .map(|variable_definition| &variable_definition.item)
                                .chain(std::iter::once(&id_var));
                            let mut traversal_state = traversal_state.clone();
                            traversal_state.refetch_paths = traversal_state
                                .refetch_paths
                                .into_iter()
                                .map(|(mut key, value)| {
                                    key.0.linked_fields.insert(
                                        0,
                                        NormalizationKey::InlineFragment(type_to_refine_to.name),
                                    );
                                    key.0.linked_fields.insert(
                                        0,
                                        NormalizationKey::ServerField(NameAndArguments {
                                            name: "node".intern().into(),
                                            arguments: vec![id_arg.clone()],
                                        }),
                                    );
                                    (key, value)
                                })
                                .collect();

                            path_and_contents.extend(
                                generate_entrypoint_artifacts_with_client_field_traversal_result(
                                    schema,
                                    encountered_client_field,
                                    &wrapped_map,
                                    &traversal_state,
                                    &encountered_client_type_map,
                                    variable_definitions_iter,
                                    &schema.find_query(),
                                    config.options.include_file_extensions_in_import_statements,
                                ),
                            );
                        }
                    }
                    ClientFieldVariant::ImperativelyLoadedField(variant) => {
                        path_and_contents.push(generate_refetch_reader_artifact(
                            schema,
                            encountered_client_field,
                            variant.primary_field_info.as_ref(),
                            &traversal_state.refetch_paths,
                            false,
                            config.options.include_file_extensions_in_import_statements,
                        ));
                    }
                };
            }
        }
    }

    for user_written_client_type in schema.client_types.iter().flat_map(|field| match field {
        SelectionType::Object(pointer) => Some(SelectionType::Object(pointer)),
        SelectionType::Scalar(field) => match field.variant {
            ClientFieldVariant::Link => None,
            ClientFieldVariant::UserWritten(_) => Some(SelectionType::Scalar(field)),
            ClientFieldVariant::ImperativelyLoadedField(_) => None,
        },
    }) {
        // For each user-written client types, generate a param type artifact
        path_and_contents.push(generate_eager_reader_param_type_artifact(
            schema,
            &user_written_client_type,
            config.options.include_file_extensions_in_import_statements,
        ));

        match encountered_client_type_map
            .get(&DefinitionLocation::Client(user_written_client_type.id()))
        {
            Some(FieldTraversalResult {
                traversal_state, ..
            }) => {
                // If this user-written client field is reachable from an entrypoint,
                // we've already noted the accessible client fields
                encountered_output_types.extend(traversal_state.accessible_client_fields.iter())
            }
            None => {
                // If this field is not reachable from an entrypoint, we need to
                // encounter all the client fields
                for nested_client_field in
                    accessible_client_fields(&user_written_client_type, schema)
                {
                    encountered_output_types.insert(nested_client_field.id);
                }
            }
        }
    }

    for output_type_id in encountered_output_types {
        let client_field = schema.client_field(output_type_id);
        let artifact_path_and_content = match client_field.variant {
            ClientFieldVariant::Link => None,
            ClientFieldVariant::UserWritten(info) => {
                Some(generate_eager_reader_output_type_artifact(
                    schema,
                    client_field,
                    config,
                    info,
                    config.options.include_file_extensions_in_import_statements,
                ))
            }
            ClientFieldVariant::ImperativelyLoadedField(_) => {
                Some(generate_refetch_output_type_artifact(schema, client_field))
            }
        };
        if let Some(path_and_content) = artifact_path_and_content {
            path_and_contents.push(path_and_content);
        };
    }

    path_and_contents.push(build_iso_overload_artifact(
        schema,
        config.options.include_file_extensions_in_import_statements,
        config.options.no_babel_transform,
    ));

    path_and_contents
}

pub(crate) fn get_serialized_field_arguments(
    // TODO make this an iterator
    arguments: &[ArgumentKeyAndValue],
    indentation_level: u8,
) -> String {
    if arguments.is_empty() {
        return "null".to_string();
    }

    let mut s = "[".to_string();

    for argument in arguments {
        s.push_str(&get_serialized_field_argument(argument, indentation_level));
    }

    s.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    s
}

fn get_serialized_field_argument(
    // TODO make this an iterator
    argument: &ArgumentKeyAndValue,
    indentation_level: u8,
) -> String {
    let indent_1 = "  ".repeat((indentation_level + 1) as usize);
    let indent_2 = "  ".repeat((indentation_level + 2) as usize);
    let indent_3 = "  ".repeat((indentation_level + 3) as usize);

    let argument_name = argument.key;

    match &argument.value {
        NonConstantValue::Variable(variable_name) => {
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"Variable\", name: \"{variable_name}\" }},\n\
                {indent_1}],\n",
            )
        }
        NonConstantValue::Integer(int_value) => {
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"Literal\", value: {int_value} }},\n\
                {indent_1}],\n"
            )
        }
        NonConstantValue::Boolean(bool) => {
            let bool_string = bool.to_string();
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"Literal\", value: {bool_string} }},\n\
                {indent_1}],\n"
            )
        }
        NonConstantValue::String(s) => {
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"String\", value: \"{s}\" }},\n\
                {indent_1}],\n"
            )
        }
        NonConstantValue::Float(f) => {
            let float = f.as_float();
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"Literal\", value: {float} }},\n\
                {indent_1}],\n"
            )
        }
        NonConstantValue::Null => {
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"Literal\", value: null }},\n\
                {indent_1}],\n"
            )
        }
        NonConstantValue::Enum(e) => {
            format!(
                "\n\
                {indent_1}[\n\
                {indent_2}\"{argument_name}\",\n\
                {indent_2}{{ kind: \"Enum\", value: \"{e}\" }},\n\
                {indent_1}],\n"
            )
        }
        NonConstantValue::List(_) => panic!("Lists are not supported here"),
        NonConstantValue::Object(object) => format!(
            "\n\
            {indent_1}[\n\
            {indent_2}\"{argument_name}\",\n\
            {indent_2}{{\n\
            {indent_3}kind: \"Object\",\n\
            {indent_3}value: [{}\n\
            {indent_3}]\n\
            {indent_2}}},\n\
            {indent_1}],\n",
            object
                .iter()
                .map(|entry| {
                    get_serialized_field_argument(
                        &ArgumentKeyAndValue {
                            key: entry.name.item.unchecked_conversion(),
                            value: entry.value.item.clone(),
                        },
                        indentation_level + 3,
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        ),
    }
}

pub(crate) fn generate_output_type<TOutputFormat: OutputFormat>(
    client_field: &ClientField<TOutputFormat>,
) -> ClientFieldOutputType {
    let variant = &client_field.variant;
    match variant {
        ClientFieldVariant::Link => ClientFieldOutputType("Link".to_string()),
        ClientFieldVariant::UserWritten(info) => match info.user_written_component_variant {
            UserWrittenComponentVariant::Eager => {
                ClientFieldOutputType("ReturnType<typeof resolver>".to_string())
            }
            UserWrittenComponentVariant::Component => ClientFieldOutputType(
                "(React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>)"
                    .to_string(),
            ),
        },
        ClientFieldVariant::ImperativelyLoadedField(params) => {
            // N.B. the string is a stable id for deduplicating
            match params.primary_field_info {
                Some(_) => {
                    ClientFieldOutputType("(params: any) => [string, () => void]".to_string())
                }
                None => ClientFieldOutputType("() => [string, () => void]".to_string()),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_client_field_parameter_type<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    selection_map: &[WithSpan<ValidatedSelection>],
    parent_type: &SchemaObject<TOutputFormat>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    link_fields: &mut LinkImports,
) -> ClientFieldParameterType {
    // TODO use unwraps
    let mut client_field_parameter_type = "{\n".to_string();

    for selection in selection_map.iter() {
        write_param_type_from_selection(
            schema,
            &mut client_field_parameter_type,
            selection,
            parent_type,
            nested_client_field_imports,
            loadable_fields,
            indentation_level + 1,
            link_fields,
        );
    }
    client_field_parameter_type.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientFieldParameterType(client_field_parameter_type)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_client_field_updatable_data_type<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    selection_map: &[WithSpan<ValidatedSelection>],
    parent_type: &SchemaObject<TOutputFormat>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    link_fields: &mut LinkImports,
    updatable_fields: &mut UpdatableImports,
) -> ClientFieldUpdatableDataType {
    // TODO use unwraps

    let mut client_field_updatable_data_type = "{\n".to_string();

    for selection in selection_map.iter() {
        write_updatable_data_type_from_selection(
            schema,
            &mut client_field_updatable_data_type,
            selection,
            parent_type,
            nested_client_field_imports,
            loadable_fields,
            indentation_level + 1,
            link_fields,
            updatable_fields,
        );
    }

    client_field_updatable_data_type
        .push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientFieldUpdatableDataType(client_field_updatable_data_type)
}

#[allow(clippy::too_many_arguments)]
fn write_param_type_from_selection<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    parent_type: &SchemaObject<TOutputFormat>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    link_fields: &mut LinkImports,
) {
    match &selection.item {
        SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
            match scalar_field_selection.associated_data.location {
                DefinitionLocation::Server(_server_field) => {
                    let parent_field = as_server_field(
                        parent_type
                            .encountered_fields
                            .get(&scalar_field_selection.name.item.into())
                            .expect("parent_field should exist 1"),
                    )
                    .expect("parent_field should exist and be server field");
                    let field = schema.server_field(*parent_field);

                    write_optional_description(
                        field.description,
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias = scalar_field_selection.name_or_alias().item;

                    let output_type = match &field.target_server_entity {
                        SelectionType::Scalar(type_annotation) => {
                            type_annotation.clone().map(&mut |scalar_id| {
                                schema.server_field_data.scalar(scalar_id).javascript_name
                            })
                        }
                        SelectionType::Object(_) => {
                            panic!("Output type should be a scalar");
                        }
                    };

                    query_type_declaration.push_str(&format!(
                        "{}readonly {}: {},\n",
                        "  ".repeat(indentation_level as usize),
                        name_or_alias,
                        print_javascript_type_declaration(&output_type)
                    ));
                }
                DefinitionLocation::Client(client_field_id) => write_param_type_from_client_field(
                    schema,
                    query_type_declaration,
                    nested_client_field_imports,
                    loadable_fields,
                    indentation_level,
                    link_fields,
                    scalar_field_selection,
                    client_field_id,
                ),
            }
        }
        SelectionTypeContainingSelections::Object(linked_field) => {
            let field = match linked_field.associated_data.field_id {
                DefinitionLocation::Server(server_field_id) => {
                    DefinitionLocation::Server(schema.server_field(server_field_id))
                }
                DefinitionLocation::Client(client_pointer_id) => {
                    DefinitionLocation::Client(schema.client_pointer(client_pointer_id))
                }
            };

            write_optional_description(
                description(&field),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = linked_field.name_or_alias().item;

            let type_annotation =
                output_type_annotation(&field)
                    .clone()
                    .map(&mut |output_type_id| {
                        let object_id = output_type_id;
                        let object = schema.server_field_data.object(object_id);
                        generate_client_field_parameter_type(
                            schema,
                            &linked_field.selection_set,
                            object,
                            nested_client_field_imports,
                            loadable_fields,
                            indentation_level,
                            link_fields,
                        )
                    });

            query_type_declaration.push_str(&format!(
                "readonly {}: {},\n",
                name_or_alias,
                print_javascript_type_declaration(&type_annotation),
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_param_type_from_client_field<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    query_type_declaration: &mut String,
    nested_client_field_imports: &mut BTreeSet<ObjectTypeAndFieldName>,
    loadable_fields: &mut BTreeSet<ObjectTypeAndFieldName>,
    indentation_level: u8,
    link_fields: &mut bool,
    scalar_field_selection: &ScalarFieldSelection<ValidatedScalarSelectionAssociatedData>,
    client_field_id: ClientFieldId,
) {
    let client_field = schema.client_field(client_field_id);
    write_optional_description(
        client_field.description,
        query_type_declaration,
        indentation_level,
    );
    query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
    match client_field.variant {
        ClientFieldVariant::Link => {
            *link_fields = true;
            let output_type = "Link";
            query_type_declaration.push_str(
                &(format!(
                    "readonly {}: {},\n",
                    scalar_field_selection.name_or_alias().item,
                    output_type
                )),
            );
        }
        ClientFieldVariant::UserWritten(_) | ClientFieldVariant::ImperativelyLoadedField(_) => {
            nested_client_field_imports.insert(client_field.type_and_field);
            let inner_output_type = format!(
                "{}__output_type",
                client_field.type_and_field.underscore_separated()
            );
            let output_type = match scalar_field_selection.associated_data.selection_variant {
                ScalarSelectionDirectiveSet::Updatable(_)
                | ScalarSelectionDirectiveSet::None(_) => inner_output_type,
                ScalarSelectionDirectiveSet::Loadable(_) => {
                    loadable_fields.insert(client_field.type_and_field);
                    let provided_arguments = get_provided_arguments(
                        client_field.variable_definitions.iter().map(|x| &x.item),
                        &scalar_field_selection.arguments,
                    );

                    let indent = "  ".repeat((indentation_level + 1) as usize);
                    let provided_args_type = if provided_arguments.is_empty() {
                        "".to_string()
                    } else {
                        format!(
                            ",\n{indent}Omit<ExtractParameters<{}__param>, keyof {}>",
                            client_field.type_and_field.underscore_separated(),
                            get_loadable_field_type_from_arguments(schema, provided_arguments)
                        )
                    };

                    format!(
                        "LoadableField<\n\
                                                    {indent}{}__param,\n\
                                                    {indent}{inner_output_type}\
                                                    {provided_args_type}\n\
                                                    {}>",
                        client_field.type_and_field.underscore_separated(),
                        "  ".repeat(indentation_level as usize),
                    )
                }
            };
            query_type_declaration.push_str(
                &(format!(
                    "readonly {}: {},\n",
                    scalar_field_selection.name_or_alias().item,
                    output_type
                )),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_updatable_data_type_from_selection<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    parent_type: &SchemaObject<TOutputFormat>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    link_fields: &mut LinkImports,
    updatable_fields: &mut UpdatableImports,
) {
    match &selection.item {
        SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
            match scalar_field_selection.associated_data.location {
                DefinitionLocation::Server(_server_field) => {
                    let parent_field = as_server_field(
                        parent_type
                            .encountered_fields
                            .get(&scalar_field_selection.name.item.into())
                            .expect("parent_field should exist 1"),
                    )
                    .expect("parent_field should exist and be server field");
                    let field = schema.server_field(*parent_field);

                    write_optional_description(
                        field.description,
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias = scalar_field_selection.name_or_alias().item;

                    let output_type = match &field.target_server_entity {
                        SelectionType::Scalar(type_annotation) => {
                            type_annotation.clone().map(&mut |scalar_id| {
                                schema.server_field_data.scalar(scalar_id).javascript_name
                            })
                        }
                        SelectionType::Object(_) => {
                            panic!("Output type should be a scalar");
                        }
                    };

                    match scalar_field_selection.associated_data.selection_variant {
                        ScalarSelectionDirectiveSet::Updatable(_) => {
                            *updatable_fields = true;
                            query_type_declaration
                                .push_str(&"  ".repeat(indentation_level as usize).to_string());
                            query_type_declaration.push_str(&format!(
                                "{}: {},\n",
                                name_or_alias,
                                print_javascript_type_declaration(&output_type)
                            ));
                        }
                        ScalarSelectionDirectiveSet::Loadable(_) => {
                            panic!("@loadable server fields are not supported")
                        }
                        ScalarSelectionDirectiveSet::None(_) => {
                            query_type_declaration.push_str(&format!(
                                "{}readonly {}: {},\n",
                                "  ".repeat(indentation_level as usize),
                                name_or_alias,
                                print_javascript_type_declaration(&output_type)
                            ));
                        }
                    }
                }
                DefinitionLocation::Client(client_field_id) => {
                    write_param_type_from_client_field(
                        schema,
                        query_type_declaration,
                        nested_client_field_imports,
                        loadable_fields,
                        indentation_level,
                        link_fields,
                        scalar_field_selection,
                        client_field_id,
                    );
                }
            }
        }
        SelectionTypeContainingSelections::Object(linked_field) => {
            let field = schema.linked_type(linked_field.associated_data.field_id);

            write_optional_description(
                description(&field),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = linked_field.name_or_alias().item;

            let type_annotation =
                output_type_annotation(&field)
                    .clone()
                    .map(&mut |output_type_id| {
                        let object_id = output_type_id;
                        let object = schema.server_field_data.object(object_id);
                        generate_client_field_updatable_data_type(
                            schema,
                            &linked_field.selection_set,
                            object,
                            nested_client_field_imports,
                            loadable_fields,
                            indentation_level,
                            link_fields,
                            updatable_fields,
                        )
                    });

            match linked_field.associated_data.selection_variant {
                ObjectSelectionDirectiveSet::Updatable(_) => {
                    *updatable_fields = true;
                    write_getter_and_setter(
                        query_type_declaration,
                        indentation_level,
                        name_or_alias,
                        output_type_annotation(&field),
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
    output_type_annotation: &TypeAnnotation<ServerObjectId>,
    type_annotation: &TypeAnnotation<ClientFieldUpdatableDataType>,
) {
    query_type_declaration.push_str(&format!(
        "get {}(): {},\n",
        name_or_alias,
        print_javascript_type_declaration(type_annotation),
    ));
    let setter_type_annotation = output_type_annotation
        .clone()
        .map(&mut |_| "{ link: Link }");
    query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
    query_type_declaration.push_str(&format!(
        "set {}(value: {}),\n",
        name_or_alias,
        print_javascript_type_declaration(&setter_type_annotation),
    ));
}

fn get_loadable_field_type_from_arguments<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
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
            format_type_for_js(schema, arg.type_.clone())
        ));
    }
    loadable_field_type.push('}');
    loadable_field_type
}

fn format_type_for_js<TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    type_: GraphQLTypeAnnotation<ServerEntityId>,
) -> String {
    let new_type = type_.map(
        |selectable_server_field_id| match selectable_server_field_id {
            ServerEntityId::Object(_) => {
                panic!(
                    "Unexpected object. Objects are not supported as parameters, yet. \
                    This is indicative of an unimplemented feature in Isograph."
                )
            }
            ServerEntityId::Scalar(scalar_id) => {
                schema.server_field_data.scalar(scalar_id).javascript_name
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

pub(crate) fn generate_parameters<'a, TOutputFormat: OutputFormat>(
    schema: &Schema<TOutputFormat>,
    argument_definitions: impl Iterator<Item = &'a VariableDefinition<ServerEntityId>>,
) -> String {
    let mut s = "{\n".to_string();
    let indent = "  ";
    for arg in argument_definitions {
        let is_optional = !matches!(arg.type_, GraphQLTypeAnnotation::NonNull(_));
        s.push_str(&format!(
            "{indent}readonly {}{}: {},\n",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_parameter_type(schema, arg.type_.clone(), 1)
        ));
    }
    s.push_str("};");
    s
}

fn write_optional_description(
    description: Option<DescriptionValue>,
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

fn print_javascript_type_declaration<T: Display + Ord + Debug>(
    type_annotation: &TypeAnnotation<T>,
) -> String {
    let mut s = String::new();
    print_javascript_type_declaration_impl(type_annotation, &mut s);
    s
}

fn print_javascript_type_declaration_impl<T: Display + Ord + Debug>(
    type_annotation: &TypeAnnotation<T>,
    s: &mut String,
) {
    match &type_annotation {
        TypeAnnotation::Scalar(scalar) => {
            s.push_str(&scalar.to_string());
        }
        TypeAnnotation::Union(union_type_annotation) => {
            if union_type_annotation.variants.is_empty() {
                panic!("Unexpected union with not enough variants.");
            }

            if union_type_annotation.variants.len() > 1 || union_type_annotation.nullable {
                s.push('(');
                for (index, variant) in union_type_annotation.variants.iter().enumerate() {
                    if index != 0 {
                        s.push_str(" | ");
                    }

                    match variant {
                        UnionVariant::Scalar(scalar) => {
                            s.push_str(&scalar.to_string());
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            print_javascript_type_declaration_impl(type_annotation, s);
                            s.push('>');
                        }
                    }
                }
                if union_type_annotation.nullable {
                    s.push_str(" | null");
                }
                s.push(')');
            } else {
                let variant = union_type_annotation
                    .variants
                    .first()
                    .expect("Expected variant to exist");
                match variant {
                    UnionVariant::Scalar(scalar) => {
                        s.push_str(&scalar.to_string());
                    }
                    UnionVariant::Plural(type_annotation) => {
                        s.push_str("ReadonlyArray<");
                        print_javascript_type_declaration_impl(type_annotation, s);
                        s.push('>');
                    }
                }
            }
        }
        TypeAnnotation::Plural(type_annotation) => {
            s.push_str("ReadonlyArray<");
            print_javascript_type_declaration_impl(type_annotation, s);
            s.push('>');
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClientFieldParameterType(pub String);
derive_display!(ClientFieldParameterType);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClientFieldUpdatableDataType(pub String);
derive_display!(ClientFieldUpdatableDataType);

#[derive(Debug)]
pub(crate) struct ClientFieldFunctionImportStatement(pub String);
derive_display!(ClientFieldFunctionImportStatement);

#[derive(Debug)]
pub(crate) struct ClientFieldOutputType(pub String);
derive_display!(ClientFieldOutputType);

#[derive(Debug)]
pub(crate) struct ReaderAst(pub String);
derive_display!(ReaderAst);

#[derive(Debug)]
pub(crate) struct NormalizationAstText(pub String);
derive_display!(NormalizationAstText);

#[derive(Debug)]
pub(crate) struct RefetchQueryArtifactImport(pub String);
derive_display!(RefetchQueryArtifactImport);
