use common_lang_types::{
    ArtifactFileName, ArtifactFilePrefix, ArtifactPathAndContent, DiagnosticVecResult,
    EmbeddedLocation, ExpectEntityToExist, ExpectSelectableToExist, VariableName,
    WithLocationPostfix, derive_display,
};
use core::panic;
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientScalarSelectableDirectiveSet, DefinitionLocation,
    DefinitionLocationPostfix, EmptyDirectiveSet, NonConstantValue, SelectionType,
    SelectionTypePostfix, TypeAnnotationDeclaration, UnionVariant, VariableDeclaration,
    VariableNameWrapper,
};
use isograph_schema::{
    ClientFieldVariant, ClientScalarSelectable, CompilationProfile, FieldMapItem,
    FieldTraversalResult, ID_ENTITY_NAME, ID_FIELD_NAME, IsographDatabase, NODE_FIELD_NAME,
    NameAndArguments, NormalizationKey, RefetchStrategy, TargetPlatform, UserWrittenClientTypeInfo,
    accessible_client_selectables, deprecated_client_selectable_map, flattened_entity_named,
    inline_fragment_reader_selection_set, refetch_strategy_for_client_scalar_selectable_named,
    selectable_named, validate_entire_schema, validated_entrypoints,
};
use isograph_schema::{ContainsIsoStats, flattened_selectable_named};
use lazy_static::lazy_static;
use prelude::*;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
};

use crate::{
    eager_reader_artifact::{
        generate_eager_reader_artifacts, generate_eager_reader_condition_artifact,
        generate_eager_reader_output_type_artifact, generate_eager_reader_param_type_artifact,
        generate_link_output_type_artifact,
    },
    entrypoint_artifact::{
        generate_entrypoint_artifacts,
        generate_entrypoint_artifacts_with_client_scalar_selectable_traversal_result,
    },
    iso_overload_file::build_iso_overload_artifact,
    persisted_documents::PersistedDocuments,
    refetch_reader_artifact::{
        generate_refetch_output_type_artifact, generate_refetch_reader_artifact,
    },
    ts_config::generate_ts_config,
};

lazy_static! {
    pub static ref ENTRYPOINT_FILE_NAME: ArtifactFileName = "entrypoint.ts".intern().into();
    pub static ref ENTRYPOINT: ArtifactFilePrefix = "entrypoint".intern().into();
    pub static ref ISO_TS_FILE_NAME: ArtifactFileName = "iso.ts".intern().into();
    pub static ref ISO_TS: ArtifactFilePrefix = "iso".intern().into();
    pub static ref NORMALIZATION_AST_FILE_NAME: ArtifactFileName =
        "normalization_ast.ts".intern().into();
    pub static ref RAW_RESPONSE_TYPE: ArtifactFilePrefix = "raw_response_type".intern().into();
    pub static ref RAW_RESPONSE_TYPE_FILE_NAME: ArtifactFileName =
        "raw_response_type.ts".intern().into();
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
#[tracing::instrument(skip_all)]
pub fn get_artifact_path_and_content<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticVecResult<(Vec<ArtifactPathAndContent>, ContainsIsoStats)> {
    let config = db.get_isograph_config();

    let stats = validate_entire_schema(db).to_owned()?;

    let mut artifact_path_and_content = get_artifact_path_and_content_impl(db);
    if let Some(header) = config.options.generated_file_header {
        for artifact_path_and_content in artifact_path_and_content.iter_mut() {
            artifact_path_and_content.file_content =
                format!("// {header}\n{}", artifact_path_and_content.file_content).into();
        }
    }
    (artifact_path_and_content, stats).wrap_ok()
}

fn get_artifact_path_and_content_impl<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<ArtifactPathAndContent> {
    let config = db.get_isograph_config();
    let mut encountered_client_type_map = BTreeMap::new();
    let mut path_and_contents = vec![];
    let mut encountered_output_types = HashSet::new();
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
        match encountered_field_id.dereference() {
            DefinitionLocation::Server((
                parent_object_entity_name,
                server_object_selectable_name,
            )) => {
                let server_object_selectable = flattened_selectable_named(
                    db,
                    parent_object_entity_name,
                    server_object_selectable_name,
                )
                .expect_selectable_to_exist(
                    parent_object_entity_name,
                    server_object_selectable_name,
                )
                .lookup(db);

                if server_object_selectable.is_inline_fragment.0 {
                    path_and_contents.push(generate_eager_reader_condition_artifact(
                        db,
                        server_object_selectable,
                        &inline_fragment_reader_selection_set(),
                        &traversal_state.refetch_paths,
                        config.options.include_file_extensions_in_import_statements,
                    ));
                }
            }

            DefinitionLocation::Client(SelectionType::Object((
                parent_object_entity_name,
                client_object_selectable_name,
            ))) => {
                let client_object_selectable =
                    selectable_named(db, parent_object_entity_name, client_object_selectable_name)
                        .as_ref()
                        .expect(
                            "Expected selectable to be valid. \
                            This is indicative of a bug in Isograph.",
                        )
                        .expect_selectable_to_exist(
                            parent_object_entity_name,
                            client_object_selectable_name,
                        )
                        .as_client()
                        .expect(
                            "Expected client selectable. \
                            This is indicative of a bug in Isograph.",
                        )
                        .as_object()
                        .expect(
                            "Expected client object selectable. \
                            This is indicative of a bug in Isograph.",
                        )
                        .lookup(db);

                path_and_contents.extend(generate_eager_reader_artifacts(
                    db,
                    &client_object_selectable.object_selected(),
                    config,
                    &UserWrittenClientTypeInfo {
                        const_export_name: client_object_selectable.info.const_export_name,
                        file_path: client_object_selectable.info.file_path,
                        client_scalar_selectable_directive_set:
                            ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {}).wrap_ok(),
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
                let client_scalar_selectable =
                    selectable_named(db, parent_object_entity_name, client_scalar_selectable_name)
                        .as_ref()
                        .expect(
                            "Expected selectable to be valid. \
                            This is indicative of a bug in Isograph.",
                        )
                        .expect_selectable_to_exist(
                            parent_object_entity_name,
                            client_scalar_selectable_name,
                        )
                        .as_client()
                        .expect(
                            "Expected client selectable. \
                            This is indicative of a bug in Isograph.",
                        )
                        .as_scalar()
                        .expect(
                            "Expected client scalar selectable. \
                            This is indicative of a bug in Isograph.",
                        )
                        .lookup(db);

                match client_scalar_selectable.variant.reference() {
                    ClientFieldVariant::Link => (),
                    ClientFieldVariant::UserWritten(info) => {
                        path_and_contents.extend(generate_eager_reader_artifacts(
                            db,
                            &client_scalar_selectable.scalar_selected(),
                            config,
                            info,
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
                                    from: ID_FIELD_NAME.unchecked_conversion(),
                                    to: ID_FIELD_NAME.unchecked_conversion(),
                                }],
                            ));

                            // Everything about this is quite sus
                            let id_arg = ArgumentKeyAndValue {
                                key: ID_FIELD_NAME.unchecked_conversion(),
                                value: NonConstantValue::Variable(
                                    ID_FIELD_NAME.unchecked_conversion::<VariableName>().into(),
                                ),
                            };

                            let type_to_refine_to = &flattened_entity_named(
                                db,
                                client_scalar_selectable.parent_entity_name,
                            )
                            .expect_entity_to_exist(client_scalar_selectable.parent_entity_name)
                            .lookup(db);

                            let variable_definitions_iter =
                                client_scalar_selectable.arguments.iter();

                            let id_var = VariableDeclaration {
                                name: (*ID_FIELD_NAME)
                                    .unchecked_conversion::<VariableName>()
                                    .to::<VariableNameWrapper>()
                                    .with_location(EmbeddedLocation::todo_generated()),

                                type_: TypeAnnotationDeclaration::Scalar((*ID_ENTITY_NAME).into())
                                    .with_location(EmbeddedLocation::todo_generated()),
                                default_value: None,
                            };

                            let refetch_strategy =
                                refetch_strategy_for_client_scalar_selectable_named(
                                    db,
                                    client_scalar_selectable.parent_entity_name,
                                    client_scalar_selectable.name,
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

                            let variable_definitions_iter = match refetch_strategy {
                                RefetchStrategy::RefetchFromRoot => {
                                    variable_definitions_iter.collect::<Vec<_>>()
                                }
                                RefetchStrategy::UseRefetchField(_) => variable_definitions_iter
                                    .chain(std::iter::once(&id_var))
                                    .collect(),
                            };

                            let mut traversal_state = traversal_state.clone();
                            traversal_state.refetch_paths = traversal_state
                                .refetch_paths
                                .into_iter()
                                .map(|(mut key, value)| {
                                    key.0.linked_fields.insert(
                                        0,
                                        NormalizationKey::InlineFragment(
                                            type_to_refine_to.name.item,
                                        ),
                                    );
                                    key.0.linked_fields.insert(
                                        0,
                                        NormalizationKey::ServerField(NameAndArguments {
                                            name: (*NODE_FIELD_NAME),
                                            arguments: vec![id_arg.clone()],
                                        }),
                                    );
                                    (key, value)
                                })
                                .collect();

                            path_and_contents.extend(
                                generate_entrypoint_artifacts_with_client_scalar_selectable_traversal_result(
                                    db,
                                    client_scalar_selectable,
                                    None,
                                    merged_selection_map.clone(),
                                    &traversal_state,
                                    &encountered_client_type_map,
                                    variable_definitions_iter,
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

    for (client_type_name, user_written_client_type) in deprecated_client_selectable_map(db)
        .as_ref()
        .expect("Expected client selectable map to be valid.")
        .iter()
        .flat_map(|(key, value)| {
            let value = value
                .as_ref()
                .expect("Expected client selectable to be valid");

            match value.reference() {
                SelectionType::Scalar(s) => match s.lookup(db).variant.reference() {
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

            (client_type_name, value).wrap_some()
        })
    {
        // For each user-written client types, generate a param type artifact
        path_and_contents.push(generate_eager_reader_param_type_artifact(
            db,
            user_written_client_type.dereference(),
            config.options.include_file_extensions_in_import_statements,
        ));

        match encountered_client_type_map.get(&client_type_name.client_defined()) {
            Some(FieldTraversalResult {
                traversal_state, ..
            }) => {
                // If this user-written client field is reachable from an entrypoint,
                // we've already noted the accessible client fields
                encountered_output_types
                    .extend(traversal_state.accessible_client_scalar_selectables.iter())
            }
            None => {
                // If this field is not reachable from an entrypoint, we need to
                // encounter all the client fields
                for nested_client_selectable_id in
                    accessible_client_selectables(db, user_written_client_type.dereference())
                {
                    encountered_output_types.insert(nested_client_selectable_id.item);
                }
            }
        }
    }

    for output_type_id in encountered_output_types {
        let (parent_object_entity_name, client_selectable_name) = match output_type_id {
            SelectionType::Scalar(s) => (s.0, s.1),
            SelectionType::Object(o) => (o.0, o.1),
        };
        let client_selectable =
            selectable_named(db, parent_object_entity_name, client_selectable_name)
                .as_ref()
                .expect(
                    "Expected selectable to be valid. \
                    This is indicative of a bug in Isograph.",
                )
                .expect_selectable_to_exist(parent_object_entity_name, client_selectable_name)
                .as_client()
                .expect(
                    "Expected client selectable. \
                    This is indicative of a bug in Isograph.",
                );

        let artifact_path_and_content = match client_selectable {
            SelectionType::Object(client_object_selectable) => {
                let client_object_selectable = client_object_selectable.lookup(db);
                generate_eager_reader_output_type_artifact(
                        db,
                        &client_object_selectable.object_selected(),
                        config,
                        &UserWrittenClientTypeInfo {
                            const_export_name: client_object_selectable.info.const_export_name,
                            file_path: client_object_selectable.info.file_path,
                            client_scalar_selectable_directive_set:
                                ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
                                    .wrap_ok(),
                        },
                        config.options.include_file_extensions_in_import_statements,
                    )
                    .wrap_some()
            }
            SelectionType::Scalar(client_scalar_selectable) => {
                let client_scalar_selectable = client_scalar_selectable.lookup(db);
                match client_scalar_selectable.variant.reference() {
                    ClientFieldVariant::Link => {
                        generate_link_output_type_artifact(db, client_scalar_selectable).wrap_some()
                    }
                    ClientFieldVariant::UserWritten(info) => {
                        generate_eager_reader_output_type_artifact(
                            db,
                            &client_scalar_selectable.scalar_selected(),
                            config,
                            info,
                            config.options.include_file_extensions_in_import_statements,
                        )
                        .wrap_some()
                    }
                    ClientFieldVariant::ImperativelyLoadedField(_) => {
                        generate_refetch_output_type_artifact(db, client_scalar_selectable)
                            .wrap_some()
                    }
                }
            }
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

    match argument.value.reference() {
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
                .collect::<String>()
        ),
    }
}

pub(crate) fn generate_output_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
) -> ClientScalarSelectableOutputType {
    let variant = &client_scalar_selectable.variant;
    match variant {
        ClientFieldVariant::Link => ClientScalarSelectableOutputType(
            TCompilationProfile::TargetPlatform::generate_link_type(
                db,
                &client_scalar_selectable.parent_entity_name,
            ),
        ),
        ClientFieldVariant::UserWritten(info) => match info
            .client_scalar_selectable_directive_set
            .clone()
            .expect(
                "Expected client scalar selectable directive set to have been validated. \
                This is indicative of a bug in Isograph.",
            ) {
            ClientScalarSelectableDirectiveSet::None(_) => {
                ClientScalarSelectableOutputType("ReturnType<typeof resolver>".to_string())
            }
            ClientScalarSelectableDirectiveSet::Component(_) => ClientScalarSelectableOutputType(
                "(React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>)"
                    .to_string(),
            ),
        },
        ClientFieldVariant::ImperativelyLoadedField(_) => {
            // TODO - we should not type params as any, but instead use some generated type
            // N.B. the string is a stable id for deduplicating
            ClientScalarSelectableOutputType("(params?: any) => [string, () => void]".to_string())
        }
    }
}

// TODO accept an inner param... this is broken right now
pub(crate) fn print_javascript_type_declaration<T: std::fmt::Display>(
    type_annotation: &TypeAnnotationDeclaration,
    inner_text: T,
) -> String {
    let mut s = String::new();
    print_javascript_type_declaration_impl(type_annotation, &mut s, &inner_text);
    s
}

// Note: we unwrap the type_annotation, but we must be careful to not accidentally
// print the inner EntityName! That's wrong. The entity's name is a concept in the
// type system (i.e. in the schema), and is not a valid Javascript type.
fn print_javascript_type_declaration_impl<T: std::fmt::Display>(
    type_annotation: &TypeAnnotationDeclaration,
    s: &mut String,
    inner_text: &T,
) {
    match type_annotation.reference() {
        TypeAnnotationDeclaration::Scalar(_) => {
            s.push_str(&inner_text.to_string());
        }
        TypeAnnotationDeclaration::Union(union_type_annotation) => {
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
                        UnionVariant::Scalar(_) => {
                            s.push_str(&inner_text.to_string());
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            print_javascript_type_declaration_impl(
                                type_annotation.item.reference(),
                                s,
                                inner_text,
                            );
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
                    UnionVariant::Scalar(_) => {
                        s.push_str(&inner_text.to_string());
                    }
                    UnionVariant::Plural(type_annotation) => {
                        s.push_str("ReadonlyArray<");
                        print_javascript_type_declaration_impl(
                            type_annotation.item.reference(),
                            s,
                            inner_text,
                        );
                        s.push('>');
                    }
                }
            }
        }
        TypeAnnotationDeclaration::Plural(type_annotation) => {
            s.push_str("ReadonlyArray<");
            print_javascript_type_declaration_impl(type_annotation.item.reference(), s, inner_text);
            s.push('>');
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClientScalarSelectableParameterType(pub String);
derive_display!(ClientScalarSelectableParameterType);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClientScalarSelectableUpdatableDataType(pub String);
derive_display!(ClientScalarSelectableUpdatableDataType);

#[derive(Debug)]
pub(crate) struct ClientScalarSelectableFunctionImportStatement(pub String);
derive_display!(ClientScalarSelectableFunctionImportStatement);

#[derive(Debug)]
pub(crate) struct ClientScalarSelectableOutputType(pub String);
derive_display!(ClientScalarSelectableOutputType);

#[derive(Debug)]
pub(crate) struct ReaderAst(pub String);
derive_display!(ReaderAst);

#[derive(Debug)]
pub(crate) struct NormalizationAstText(pub String);
derive_display!(NormalizationAstText);

#[derive(Debug)]
pub(crate) struct RefetchQueryArtifactImport(pub String);
derive_display!(RefetchQueryArtifactImport);
