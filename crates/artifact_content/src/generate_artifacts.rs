use common_lang_types::{
    ArtifactFileName, ArtifactFilePrefix, ArtifactPathAndContent, ClientScalarSelectableName,
    ParentObjectEntityNameAndSelectableName, SelectableNameOrAlias, ServerObjectEntityName,
    WithLocation, WithSpan, WithSpanPostfix, derive_display,
};
use core::panic;
use graphql_lang_types::{
    GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation,
};
use intern::{Lookup, string_key::Intern};
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientScalarSelectionDirectiveSet, DefinitionLocation,
    DefinitionLocationPostfix, Description, EmptyDirectiveSet, NonConstantValue,
    ObjectSelectionDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionFieldArgument, SelectionSet, SelectionType, SelectionTypeContainingSelections,
    SelectionTypePostfix, TypeAnnotation, UnionVariant, VariableDefinition,
};
use isograph_schema::{
    ClientFieldVariant, ClientScalarSelectable, ClientSelectableId, FieldMapItem,
    FieldTraversalResult, ID_ENTITY_NAME, IsographDatabase, LINK_FIELD_NAME, NODE_FIELD_NAME,
    NameAndArguments, NetworkProtocol, NormalizationKey, RefetchStrategy, ScalarSelectableId,
    SelectableTrait, ServerEntityName, ServerObjectSelectableVariant, UserWrittenClientTypeInfo,
    ValidatedSelection, ValidatedVariableDefinition, ValidationError, WrappedSelectionMapSelection,
    accessible_client_fields, client_object_selectable_named, client_scalar_selectable_named,
    client_selectable_map, client_selectable_named, description, fetchable_types,
    inline_fragment_reader_selection_set, output_type_annotation, selectable_named,
    selection_map_wrapped, server_object_entity_named, server_object_selectable_named,
    server_scalar_entity_javascript_name, server_scalar_selectable_named, validate_entire_schema,
    validated_entrypoints, validated_refetch_strategy_for_client_scalar_selectable_named,
};
use isograph_schema::{ContainsIsoStats, ObjectSelectableId};
use lazy_static::lazy_static;
use prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::{Debug, Display},
};
use thiserror::Error;

use crate::{
    eager_reader_artifact::{
        generate_eager_reader_artifacts, generate_eager_reader_condition_artifact,
        generate_eager_reader_output_type_artifact, generate_eager_reader_param_type_artifact,
        generate_link_output_type_artifact,
    },
    entrypoint_artifact::{
        generate_entrypoint_artifacts,
        generate_entrypoint_artifacts_with_client_field_traversal_result,
    },
    format_parameter_type::format_parameter_type,
    import_statements::{ParamTypeImports, UpdatableImports},
    iso_overload_file::build_iso_overload_artifact,
    persisted_documents::PersistedDocuments,
    refetch_reader_artifact::{
        generate_refetch_output_type_artifact, generate_refetch_reader_artifact,
    },
    ts_config::generate_ts_config,
};

lazy_static! {
    pub static ref ENTRYPOINT_FILE_NAME: ArtifactFileName = "entrypoint.ts".intern().into();
    pub static ref ARTIFACT_LOOKUP_CACHE_FILE_NAME: ArtifactFileName =
        ".artifact_lookup_cache.json".intern().into();
    pub static ref ENTRYPOINT: ArtifactFilePrefix = "entrypoint".intern().into();
    pub static ref ISO_TS_FILE_NAME: ArtifactFileName = "iso.ts".intern().into();
    pub static ref ISO_TS: ArtifactFilePrefix = "iso".intern().into();
    pub static ref NORMALIZATION_AST_FILE_NAME: ArtifactFileName =
        "normalization_ast.ts".intern().into();
    pub static ref RAW_RESPONSE_TYPE: ArtifactFileName = "raw_response_type.ts".intern().into();
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
    pub static ref PERSISTED_DOCUMENT_FILE_NAME: ArtifactFileName =
        "persisted_documents.json".intern().into();
}

/// Get all artifacts according to the following scheme:
///
/// For each entrypoint, generate an entrypoint artifact. This involves
/// generating the merged selection map.
///
/// - While creating a client field's merged selection map, whenever we enter
///   a client field, we check a cache (`encountered_client_type_map`). If that
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
#[tracing::instrument]
pub fn get_artifact_path_and_content<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(Vec<ArtifactPathAndContent>, ContainsIsoStats), GetArtifactPathAndContentError> {
    let config = db.get_isograph_config();

    let stats = validate_entire_schema(db)
        .to_owned()
        .map_err(|errors| GetArtifactPathAndContentError::ValidationError { errors })?;

    let mut artifact_path_and_content = get_artifact_path_and_content_impl(db);
    if let Some(header) = config.options.generated_file_header {
        for artifact_path_and_content in artifact_path_and_content.iter_mut() {
            artifact_path_and_content.file_content =
                format!("// {header}\n{}", artifact_path_and_content.file_content);
        }
    }
    (artifact_path_and_content, stats.clone()).ok()
}

fn get_artifact_path_and_content_impl<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Vec<ArtifactPathAndContent> {
    let config = db.get_isograph_config();
    let mut encountered_client_type_map = BTreeMap::new();
    let mut path_and_contents = vec![];
    let mut encountered_output_types = HashSet::<ClientSelectableId>::new();
    let mut persisted_documents =
        config
            .options
            .persisted_documents
            .as_ref()
            .map(|options| PersistedDocuments {
                options,
                documents: BTreeMap::new(),
            });

    // For each entrypoint, generate an entrypoint artifact and refetch artifacts
    for ((parent_object_entity_name, entrypoint_selectable_name), entrypoint_info) in
        validated_entrypoints(db)
    {
        let entrypoint_info = entrypoint_info
            .as_ref()
            .expect("Expected entrypoints to be validated");

        let entrypoint_path_and_content = generate_entrypoint_artifacts(
            db,
            *parent_object_entity_name,
            *entrypoint_selectable_name,
            entrypoint_info,
            &mut encountered_client_type_map,
            config.options.include_file_extensions_in_import_statements,
            &mut persisted_documents,
        );
        path_and_contents.extend(entrypoint_path_and_content);

        // We also need to generate output types for entrypoints
        encountered_output_types
            .insert((*parent_object_entity_name, *entrypoint_selectable_name).scalar_selected());
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
            DefinitionLocation::Server((
                parent_object_entity_name,
                server_object_selectable_name,
            )) => {
                let server_object_selectable = server_object_selectable_named(
                    db,
                    *parent_object_entity_name,
                    (*server_object_selectable_name).into(),
                )
                .as_ref()
                .expect(
                    "Expected validation to have succeeded. \
                    This is indicative of a bug in Isograph.",
                )
                .as_ref()
                .expect(
                    "Expected selectable to exist. \
                        This is indicative of a bug in Isograph.",
                );

                match &server_object_selectable.object_selectable_variant {
                    ServerObjectSelectableVariant::LinkedField => {}
                    ServerObjectSelectableVariant::InlineFragment => {
                        path_and_contents.push(generate_eager_reader_condition_artifact(
                            db,
                            server_object_selectable,
                            &inline_fragment_reader_selection_set(server_object_selectable),
                            &traversal_state.refetch_paths,
                            config.options.include_file_extensions_in_import_statements,
                        ));
                    }
                }
            }

            DefinitionLocation::Client(SelectionType::Object((
                parent_object_entity_name,
                client_object_selectable_name,
            ))) => {
                let client_object_selectable = client_object_selectable_named(
                    db,
                    *parent_object_entity_name,
                    *client_object_selectable_name,
                )
                .as_ref()
                .expect(
                    "Expected selectable to be valid. \
                    This is indicative of a bug in Isograph.",
                )
                .as_ref()
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                );

                path_and_contents.extend(generate_eager_reader_artifacts(
                    db,
                    &client_object_selectable.object_selected(),
                    config,
                    UserWrittenClientTypeInfo {
                        const_export_name: client_object_selectable.info.const_export_name,
                        file_path: client_object_selectable.info.file_path,
                        client_field_directive_set: ClientScalarSelectionDirectiveSet::None(
                            EmptyDirectiveSet {},
                        ),
                    },
                    &traversal_state.refetch_paths,
                    config.options.include_file_extensions_in_import_statements,
                    traversal_state.has_updatable,
                ));
            }
            DefinitionLocation::Client(SelectionType::Scalar((
                parent_object_entity_name,
                client_scalar_selectable_name,
            ))) => {
                let client_scalar_selectable = client_scalar_selectable_named(
                    db,
                    *parent_object_entity_name,
                    *client_scalar_selectable_name,
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
                );

                match &client_scalar_selectable.variant {
                    ClientFieldVariant::Link => (),
                    ClientFieldVariant::UserWritten(info) => {
                        path_and_contents.extend(generate_eager_reader_artifacts(
                            db,
                            &client_scalar_selectable.scalar_selected(),
                            config,
                            *info,
                            &traversal_state.refetch_paths,
                            config.options.include_file_extensions_in_import_statements,
                            traversal_state.has_updatable,
                        ));

                        if *was_ever_selected_loadably {
                            path_and_contents.push(generate_refetch_reader_artifact(
                                db,
                                client_scalar_selectable,
                                &traversal_state.refetch_paths,
                                true,
                                config.options.include_file_extensions_in_import_statements,
                                &[FieldMapItem {
                                    from: "id".intern().into(),
                                    to: "id".intern().into(),
                                }],
                            ));

                            // Everything about this is quite sus
                            let id_arg = ArgumentKeyAndValue {
                                key: "id".intern().into(),
                                value: NonConstantValue::Variable("id".intern().into()),
                            };

                            let type_to_refine_to = &server_object_entity_named(
                                db,
                                client_scalar_selectable.parent_object_entity_name(),
                            )
                            .as_ref()
                            .expect(
                                "Expected validation to have worked. \
                                    This is indicative of a bug in Isograph.",
                            )
                            .as_ref()
                            .expect(
                                "Expected entity to exist. \
                                    This is indicative of a bug in Isograph.",
                            );

                            let variable_definitions_iter = client_scalar_selectable
                                .variable_definitions
                                .iter()
                                .map(|variable_definition| &variable_definition.item);

                            let id_var = ValidatedVariableDefinition {
                                name: WithLocation::new_generated("id".intern().into()),
                                type_: GraphQLTypeAnnotation::NonNull(
                                    GraphQLNonNullTypeAnnotation::Named(
                                        GraphQLNamedTypeAnnotation(
                                            ServerEntityName::Scalar(*ID_ENTITY_NAME)
                                                .with_generated_span(),
                                        ),
                                    )
                                    .boxed(),
                                ),
                                default_value: None,
                            };

                            let validated_refetch_strategy =
                                validated_refetch_strategy_for_client_scalar_selectable_named(
                                    db,
                                    client_scalar_selectable.parent_object_entity_name,
                                    client_scalar_selectable.name.item,
                                )
                                .as_ref()
                                .expect(
                                    "Expected refetch strategy to be valid. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .as_ref()
                                .expect(
                                    "Expected refetch strategy. \
                                    This is indicative of a bug in Isograph.",
                                );

                            let (wrapped_map, variable_definitions_iter) =
                                match validated_refetch_strategy {
                                    RefetchStrategy::RefetchFromRoot => (
                                        selection_map_wrapped(merged_selection_map.clone(), vec![]),
                                        variable_definitions_iter.collect::<Vec<_>>(),
                                    ),
                                    RefetchStrategy::UseRefetchField(_) => {
                                        let fetchable_types_map =
                                            fetchable_types(db).as_ref().expect(
                                                "Expected parsing to have succeeded. \
                                                This is indicative of a bug in Isograph.",
                                            );

                                        let query_id = fetchable_types_map
                                            .iter()
                                            .find(|(_, root_operation_name)| {
                                                root_operation_name.0 == "query"
                                            })
                                            .expect("Expected query to be found")
                                            .0;

                                        let wrapped_map = selection_map_wrapped(
                                            merged_selection_map.clone(),
                                            vec![
                                                WrappedSelectionMapSelection::InlineFragment(
                                                    type_to_refine_to.name,
                                                ),
                                                WrappedSelectionMapSelection::LinkedField {
                                                    parent_object_entity_name: *query_id,
                                                    server_object_selectable_name: *NODE_FIELD_NAME,
                                                    arguments: vec![id_arg.clone()],
                                                    concrete_type: None,
                                                },
                                            ],
                                        );

                                        (
                                            wrapped_map,
                                            variable_definitions_iter
                                                .chain(std::iter::once(&id_var))
                                                .collect(),
                                        )
                                    }
                                };

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

                            let query_type = fetchable_types(db)
                                .as_ref()
                                .expect(
                                    "Expected parsing to have succeeded. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .iter()
                                .find(|(_, root_operation_name)| root_operation_name.0 == "query");

                            path_and_contents.extend(
                                generate_entrypoint_artifacts_with_client_field_traversal_result(
                                    db,
                                    client_scalar_selectable,
                                    None,
                                    &wrapped_map,
                                    &traversal_state,
                                    &encountered_client_type_map,
                                    variable_definitions_iter,
                                    &query_type,
                                    config.options.include_file_extensions_in_import_statements,
                                    &mut persisted_documents,
                                ),
                            );
                        }
                    }
                    ClientFieldVariant::ImperativelyLoadedField(s) => {
                        path_and_contents.push(generate_refetch_reader_artifact(
                            db,
                            client_scalar_selectable,
                            &traversal_state.refetch_paths,
                            false,
                            config.options.include_file_extensions_in_import_statements,
                            &s.field_map,
                        ));
                    }
                };
            }
        }
    }

    for (client_type_name, user_written_client_type) in client_selectable_map(db)
        .as_ref()
        .expect("Expected client selectable map to be valid.")
        .iter()
        .flat_map(|(key, value)| {
            let value = value
                .as_ref()
                .expect("Expected client selectable to be valid");

            match &value {
                SelectionType::Scalar(s) => match &s.variant {
                    isograph_schema::ClientFieldVariant::UserWritten(_) => {}
                    isograph_schema::ClientFieldVariant::ImperativelyLoadedField(_) => return None,
                    isograph_schema::ClientFieldVariant::Link => return None,
                },
                SelectionType::Object(_) => {}
            };

            let client_type_name = match value {
                SelectionType::Scalar(_) => (key.0, key.1.unchecked_conversion()).scalar_selected(),
                SelectionType::Object(_) => (key.0, key.1.unchecked_conversion()).object_selected(),
            };

            (client_type_name, value).some()
        })
    {
        // For each user-written client types, generate a param type artifact
        path_and_contents.push(generate_eager_reader_param_type_artifact(
            db,
            user_written_client_type,
            config.options.include_file_extensions_in_import_statements,
        ));

        match encountered_client_type_map.get(&client_type_name.client_defined()) {
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
                for nested_client_field_id in accessible_client_fields(db, user_written_client_type)
                {
                    encountered_output_types.insert(nested_client_field_id);
                }
            }
        }
    }

    for output_type_id in encountered_output_types {
        let (parent_object_entity_name, client_selectable_name) = match output_type_id {
            SelectionType::Scalar(s) => (s.0, s.1.into()),
            SelectionType::Object(o) => (o.0, o.1.into()),
        };
        let client_selectable =
            client_selectable_named(db, parent_object_entity_name, client_selectable_name)
                .as_ref()
                .expect(
                    "Expected selectable to be valid. \
                    This is indicative of a bug in Isograph.",
                )
                .as_ref()
                .expect(
                    "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
                );

        let artifact_path_and_content = match client_selectable {
            SelectionType::Object(client_pointer) => generate_eager_reader_output_type_artifact(
                db,
                &client_pointer.object_selected(),
                config,
                UserWrittenClientTypeInfo {
                    const_export_name: client_pointer.info.const_export_name,
                    file_path: client_pointer.info.file_path,
                    client_field_directive_set: ClientScalarSelectionDirectiveSet::None(
                        EmptyDirectiveSet {},
                    ),
                },
                config.options.include_file_extensions_in_import_statements,
            )
            .some(),
            SelectionType::Scalar(client_field) => match client_field.variant {
                ClientFieldVariant::Link => {
                    generate_link_output_type_artifact(db, client_field).some()
                }
                ClientFieldVariant::UserWritten(info) => {
                    generate_eager_reader_output_type_artifact(
                        db,
                        &client_field.scalar_selected(),
                        config,
                        info,
                        config.options.include_file_extensions_in_import_statements,
                    )
                    .some()
                }
                ClientFieldVariant::ImperativelyLoadedField(_) => {
                    generate_refetch_output_type_artifact(db, client_field).some()
                }
            },
        };

        if let Some(path_and_content) = artifact_path_and_content {
            path_and_contents.push(path_and_content);
        }
    }

    path_and_contents.push(build_iso_overload_artifact(
        db,
        config.options.include_file_extensions_in_import_statements,
        config.options.no_babel_transform,
    ));
    path_and_contents.push(generate_ts_config());

    if let Some(persisted_documents) = persisted_documents {
        path_and_contents.push(persisted_documents.path_and_content());
    }

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

pub(crate) fn generate_output_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_field: &ClientScalarSelectable<TNetworkProtocol>,
) -> ClientFieldOutputType {
    let variant = &client_field.variant;
    match variant {
        ClientFieldVariant::Link => ClientFieldOutputType(TNetworkProtocol::generate_link_type(
            db,
            &client_field.parent_object_entity_name,
        )),
        ClientFieldVariant::UserWritten(info) => match info.client_field_directive_set {
            ClientScalarSelectionDirectiveSet::None(_) => {
                ClientFieldOutputType("ReturnType<typeof resolver>".to_string())
            }
            ClientScalarSelectionDirectiveSet::Component(_) => ClientFieldOutputType(
                "(React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>)"
                    .to_string(),
            ),
        },
        ClientFieldVariant::ImperativelyLoadedField(_) => {
            // TODO - we should not type params as any, but instead use some generated type
            // N.B. the string is a stable id for deduplicating
            ClientFieldOutputType("(params?: any) => [string, () => void]".to_string())
        }
    }
}

pub(crate) fn generate_client_field_parameter_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_map: &WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) -> ClientFieldParameterType {
    // TODO use unwraps
    let mut client_field_parameter_type = "{\n".to_string();

    for selection in selection_map.item.selections.iter() {
        write_param_type_from_selection(
            db,
            &mut client_field_parameter_type,
            selection,
            nested_client_field_imports,
            loadable_fields,
            indentation_level + 1,
        );
    }
    client_field_parameter_type.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientFieldParameterType(client_field_parameter_type)
}

pub(crate) fn generate_client_field_updatable_data_type<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_map: &WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    updatable_fields: &mut UpdatableImports,
) -> ClientFieldUpdatableDataType {
    // TODO use unwraps

    let mut client_field_updatable_data_type = "{\n".to_string();

    for selection in selection_map.item.selections.iter() {
        write_updatable_data_type_from_selection(
            db,
            &mut client_field_updatable_data_type,
            selection,
            nested_client_field_imports,
            loadable_fields,
            indentation_level + 1,
            updatable_fields,
        );
    }

    client_field_updatable_data_type
        .push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientFieldUpdatableDataType(client_field_updatable_data_type)
}

fn write_param_type_from_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) {
    match &selection.item {
        SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
            match scalar_field_selection.associated_data {
                DefinitionLocation::Server((
                    parent_object_entity_name,
                    server_scalar_selectable_name,
                )) => {
                    let server_scalar_selectable = server_scalar_selectable_named(
                        db,
                        parent_object_entity_name,
                        server_scalar_selectable_name.into(),
                    )
                    .as_ref()
                    .expect(
                        "Expected validation to have succeeded. \
                            This is indicative of a bug in Isograph.",
                    )
                    .as_ref()
                    .expect(
                        "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                    );

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
                DefinitionLocation::Client((parent_object_entity_name, client_field_name)) => {
                    write_param_type_from_client_field(
                        db,
                        query_type_declaration,
                        nested_client_field_imports,
                        loadable_fields,
                        indentation_level,
                        scalar_field_selection,
                        parent_object_entity_name,
                        client_field_name,
                    )
                }
            }
        }
        SelectionTypeContainingSelections::Object(linked_field) => {
            let (parent_object_entity_name, object_selectable_name) =
                match linked_field.associated_data {
                    DefinitionLocation::Server((
                        parent_object_entity_name,
                        server_object_selectable_name,
                    )) => (
                        parent_object_entity_name,
                        server_object_selectable_name.into(),
                    ),
                    DefinitionLocation::Client((
                        parent_object_entity_name,
                        client_object_selectable_name,
                    )) => (
                        parent_object_entity_name,
                        client_object_selectable_name.into(),
                    ),
                };

            let object_selectable =
                selectable_named(db, parent_object_entity_name, object_selectable_name)
                    // TODO why do we have to clone?
                    .clone()
                    .expect(
                        "Expected selectable to be valid. \
                        This is indicative of a bug in Isograph.",
                    )
                    .expect(
                        "Expected selectable to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .as_object()
                    // TODO have an object_selectable_named method
                    .expect(
                        "Expected selectable to be object. \
                        This is indicative of a bug in Isograph.",
                    );

            write_optional_description(
                description(&object_selectable),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = linked_field.name_or_alias().item;

            let type_annotation =
                output_type_annotation(&object_selectable)
                    .clone()
                    .map(&mut |_| {
                        generate_client_field_parameter_type(
                            db,
                            &linked_field.selection_set,
                            nested_client_field_imports,
                            loadable_fields,
                            indentation_level,
                        )
                    });

            query_type_declaration.push_str(&format!(
                "readonly {}: {},\n",
                name_or_alias,
                match object_selectable {
                    DefinitionLocation::Client(client_pointer) => {
                        loadable_fields.insert(client_pointer.type_and_field);

                        print_javascript_type_declaration(&type_annotation.map(&mut |target| {
                            format!(
                                "LoadableField<{}__param, {target}>",
                                client_pointer.type_and_field.underscore_separated(),
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

#[expect(clippy::too_many_arguments)]
fn write_param_type_from_client_field<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    query_type_declaration: &mut String,
    nested_client_field_imports: &mut BTreeSet<ParentObjectEntityNameAndSelectableName>,
    loadable_fields: &mut BTreeSet<ParentObjectEntityNameAndSelectableName>,
    indentation_level: u8,
    scalar_field_selection: &ScalarSelection<ScalarSelectableId>,
    parent_object_entity_name: ServerObjectEntityName,
    client_scalar_selectable_name: ClientScalarSelectableName,
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
    );

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
            nested_client_field_imports.insert(client_scalar_selectable.type_and_field);
            let inner_output_type = format!(
                "{}__output_type",
                client_scalar_selectable
                    .type_and_field
                    .underscore_separated()
            );
            let output_type = match scalar_field_selection.scalar_selection_directive_set {
                ScalarSelectionDirectiveSet::Updatable(_)
                | ScalarSelectionDirectiveSet::None(_) => inner_output_type,
                ScalarSelectionDirectiveSet::Loadable(_) => {
                    loadable_fields.insert(client_scalar_selectable.type_and_field);
                    let provided_arguments = get_provided_arguments(
                        client_scalar_selectable
                            .variable_definitions
                            .iter()
                            .map(|x| &x.item),
                        &scalar_field_selection.arguments,
                    );

                    let indent = "  ".repeat((indentation_level + 1) as usize);
                    let provided_args_type = if provided_arguments.is_empty() {
                        "".to_string()
                    } else {
                        format!(
                            ",\n{indent}Omit<ExtractParameters<{}__param>, keyof {}>",
                            client_scalar_selectable
                                .type_and_field
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
                            .type_and_field
                            .underscore_separated(),
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

fn write_updatable_data_type_from_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
    updatable_fields: &mut UpdatableImports,
) {
    match &selection.item {
        SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
            match scalar_field_selection.associated_data {
                DefinitionLocation::Server((
                    parent_object_entity_name,
                    server_scalar_selectable_name,
                )) => {
                    let server_scalar_selectable = server_scalar_selectable_named(
                        db,
                        parent_object_entity_name,
                        server_scalar_selectable_name.into(),
                    )
                    .as_ref()
                    .expect(
                        "Expected validation to have succeeded. \
                            This is indicative of a bug in Isograph.",
                    )
                    .as_ref()
                    .expect(
                        "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                    );

                    write_optional_description(
                        server_scalar_selectable.description,
                        query_type_declaration,
                        indentation_level,
                    );

                    let name_or_alias = scalar_field_selection.name_or_alias().item;

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

                    match scalar_field_selection.scalar_selection_directive_set {
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
                DefinitionLocation::Client((parent_object_entity_name, client_field_id)) => {
                    write_param_type_from_client_field(
                        db,
                        query_type_declaration,
                        nested_client_field_imports,
                        loadable_fields,
                        indentation_level,
                        scalar_field_selection,
                        parent_object_entity_name,
                        client_field_id,
                    );
                }
            }
        }
        SelectionTypeContainingSelections::Object(linked_field) => {
            let (parent_object_entity_name, object_selectable_name) =
                match linked_field.associated_data {
                    DefinitionLocation::Server((
                        parent_object_entity_name,
                        server_object_selectable_name,
                    )) => (
                        parent_object_entity_name,
                        server_object_selectable_name.into(),
                    ),
                    DefinitionLocation::Client((
                        parent_object_entity_name,
                        client_object_selectable_name,
                    )) => (
                        parent_object_entity_name,
                        client_object_selectable_name.into(),
                    ),
                };

            let object_selectable =
                selectable_named(db, parent_object_entity_name, object_selectable_name)
                    // TODO Why do we have to clone here, instead of calling as_ref?
                    .clone()
                    .expect(
                        "Expected selectable to be valid. \
                        This is indicative of a bug in Isograph.",
                    )
                    .expect(
                        "Expected selectable to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .as_object()
                    // TODO have an object_selectable_named method
                    .expect(
                        "Expected selectable to be object. \
                        This is indicative of a bug in Isograph.",
                    );

            write_optional_description(
                description(&object_selectable),
                query_type_declaration,
                indentation_level,
            );
            query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
            let name_or_alias = linked_field.name_or_alias().item;

            let type_annotation =
                output_type_annotation(&object_selectable)
                    .clone()
                    .map(&mut |_| {
                        generate_client_field_updatable_data_type(
                            db,
                            &linked_field.selection_set,
                            nested_client_field_imports,
                            loadable_fields,
                            indentation_level,
                            updatable_fields,
                        )
                    });

            match linked_field.object_selection_directive_set {
                ObjectSelectionDirectiveSet::Updatable(_) => {
                    *updatable_fields = true;
                    write_getter_and_setter(
                        query_type_declaration,
                        indentation_level,
                        name_or_alias,
                        output_type_annotation(&object_selectable),
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
    output_type_annotation: &TypeAnnotation<ServerObjectEntityName>,
    type_annotation: &TypeAnnotation<ClientFieldUpdatableDataType>,
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

pub(crate) fn generate_parameters<'a, TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    argument_definitions: impl Iterator<Item = &'a VariableDefinition<ServerEntityName>>,
) -> String {
    let mut s = "{\n".to_string();
    let indent = "  ";
    for arg in argument_definitions {
        let is_optional = !matches!(arg.type_, GraphQLTypeAnnotation::NonNull(_));
        s.push_str(&format!(
            "{indent}readonly {}{}: {},\n",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_parameter_type(db, arg.type_.clone(), 1)
        ));
    }
    s.push_str("};");
    s
}

fn write_optional_description(
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

pub(crate) fn print_javascript_type_declaration<T: Display + Ord + Debug>(
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

pub fn get_provided_arguments<'a>(
    argument_definitions: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> Vec<ValidatedVariableDefinition> {
    argument_definitions
        .filter_map(|definition| {
            let user_has_supplied_argument = arguments
                .iter()
                .any(|arg| definition.name.item == arg.item.name.item);
            if user_has_supplied_argument {
                definition.clone().some()
            } else {
                None
            }
        })
        .collect()
}

#[derive(Error, Debug, Clone)]
pub enum GetArtifactPathAndContentError {
    #[error(
        "{}",
        errors.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    ValidationError { errors: Vec<ValidationError> },
}
