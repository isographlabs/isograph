use std::collections::{BTreeSet, HashSet};

use common_lang_types::{
    EmbeddedLocation, EntityName, EntityNameAndSelectableName, SelectableName,
    WithEmbeddedLocation, WithLocationPostfix,
};
use isograph_lang_types::{
    ClientScalarSelectableDirectiveSet, DefinitionLocation, DefinitionLocationPostfix,
    EmptyDirectiveSet, LoadableDirectiveParameters, ObjectSelection, ObjectSelectionDirectiveSet,
    ScalarSelection, ScalarSelectionDirectiveSet, Selection, SelectionSet, SelectionType,
    SelectionTypePostfix,
};
use isograph_schema::{
    BorrowedObjectSelectable, ClientFieldVariant, ClientScalarSelectable, CompilationProfile,
    IsographDatabase, Loadability, NameAndArguments, NormalizationKey, PathToRefetchField,
    RefetchedPathsMap, VariableContext, categorize_field_loadability,
    client_scalar_selectable_selection_set_for_parent_query,
    refetch_strategy_for_client_scalar_selectable_named, selectable_named,
    selectable_reader_selection_set, server_entity_named, transform_arguments_with_child_context,
};
use pico::MemoRef;
use prelude::Postfix;

use crate::{
    generate_artifacts::{ReaderAst, get_serialized_field_arguments},
    import_statements::{ImportedFileCategory, ReaderImports},
};

// Can we do this when visiting the client field in when generating entrypoints?
#[expect(clippy::too_many_arguments)]
fn generate_reader_ast_node<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection: &WithEmbeddedLocation<Selection>,
    indentation_level: u8,
    reader_imports: &mut ReaderImports,
    // TODO use this to generate usedRefetchQueries
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NormalizationKey>,
    initial_variable_context: &VariableContext,
) -> String {
    let selectable = selectable_named(db, parent_object_entity_name, selection.item.name())
        .as_ref()
        .expect("Expected parsing to have succeeded. This is indicative of a bug in Isograph.")
        .expect("Expected selectable to exist. This is indicative of a bug in Isograph.");

    match selection.item.reference() {
        SelectionType::Scalar(scalar_field_selection) => {
            let scalar_selectable = match selectable {
                DefinitionLocation::Server(s) => {
                    let selectable = s.lookup(db);
                    let entity = server_entity_named(
                        db,
                        selectable
                            .target_entity
                            .as_ref()
                            .expect("Expected target entity to be valid.")
                            .inner()
                            .0,
                    )
                    .as_ref()
                    .expect("Expected parsing to have succeeded")
                    .expect("Expected target entity to be defined")
                    .lookup(db);

                    // TODO is this already validated?
                    entity
                        .selection_info
                        .as_scalar()
                        .expect("Expected selectable to be a scalar");

                    selectable.server_defined()
                }
                DefinitionLocation::Client(c) => c
                    .as_scalar()
                    .expect("Expected selectable to be scalar.")
                    .client_defined(),
            };

            match scalar_selectable {
                DefinitionLocation::Server(_) => server_defined_scalar_field_ast_node(
                    scalar_field_selection,
                    indentation_level,
                    initial_variable_context,
                ),
                DefinitionLocation::Client(client_scalar_selectable) => {
                    scalar_client_defined_field_ast_node(
                        db,
                        scalar_field_selection,
                        client_scalar_selectable,
                        indentation_level,
                        path,
                        root_refetched_paths,
                        reader_imports,
                        initial_variable_context,
                    )
                }
            }
        }
        SelectionType::Object(object_selection) => {
            let object_selectable = match selectable {
                DefinitionLocation::Server(s) => {
                    let selectable = s.lookup(db);
                    let entity = server_entity_named(
                        db,
                        selectable
                            .target_entity
                            .as_ref()
                            .expect("Expected target entity to be valid.")
                            .inner()
                            .0,
                    )
                    .as_ref()
                    .expect("Expected parsing to have succeeded")
                    .expect("Expected target entity to be defined")
                    .lookup(db);

                    // TODO is this already validated?
                    entity
                        .selection_info
                        .as_object()
                        .expect("Expected selectable to be an object");

                    selectable.server_defined()
                }
                DefinitionLocation::Client(c) => c
                    .as_object()
                    .expect("Expected selectable to be an object.")
                    .client_defined(),
            };

            match object_selectable {
                DefinitionLocation::Client(client_object_selectable) => {
                    let client_object_selectable = client_object_selectable.lookup(db);
                    path.push(NormalizationKey::ClientPointer(NameAndArguments {
                        // TODO use alias
                        name: object_selection.name.item,
                        // TODO this clearly does something, but why are we able to pass
                        // the initial variable context here??
                        arguments: transform_arguments_with_child_context(
                            object_selection
                                .arguments
                                .iter()
                                .map(|x| x.item.into_key_and_value()),
                            // TODO why is this not the transformed context?
                            initial_variable_context,
                        ),
                    }));

                    let inner_reader_ast = generate_reader_ast_with_path(
                        db,
                        client_object_selectable.target_entity_name.inner().0,
                        &object_selection.selection_set,
                        indentation_level + 1,
                        reader_imports,
                        root_refetched_paths,
                        path,
                        initial_variable_context,
                    );

                    path.pop();

                    linked_field_ast_node(
                        object_selection,
                        client_object_selectable.client_defined(),
                        indentation_level,
                        inner_reader_ast,
                        initial_variable_context,
                        reader_imports,
                        root_refetched_paths,
                        path,
                    )
                }
                DefinitionLocation::Server(server_object_selectable) => {
                    let normalization_key = if server_object_selectable.is_inline_fragment.0 {
                        NormalizationKey::InlineFragment(
                            server_object_selectable
                                .target_entity
                                .as_ref()
                                .expect("Expected target entity to be valid.")
                                .inner()
                                .0,
                        )
                    } else {
                        NameAndArguments {
                            // TODO use alias
                            name: object_selection.name.item,
                            // TODO this clearly does something, but why are we able to pass
                            // the initial variable context here??
                            arguments: transform_arguments_with_child_context(
                                object_selection
                                    .arguments
                                    .iter()
                                    .map(|x| x.item.into_key_and_value()),
                                // TODO why is this not the transformed context?
                                initial_variable_context,
                            ),
                        }
                        .normalization_key()
                    };

                    path.push(normalization_key);

                    let inner_reader_ast = generate_reader_ast_with_path(
                        db,
                        server_object_selectable
                            .target_entity
                            .as_ref()
                            .expect("Expected target entity to be valid.")
                            .inner()
                            .0,
                        &object_selection.selection_set,
                        indentation_level + 1,
                        reader_imports,
                        root_refetched_paths,
                        path,
                        initial_variable_context,
                    );

                    path.pop();

                    linked_field_ast_node(
                        object_selection,
                        server_object_selectable.server_defined(),
                        indentation_level,
                        inner_reader_ast,
                        initial_variable_context,
                        reader_imports,
                        root_refetched_paths,
                        path,
                    )
                }
            }
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn linked_field_ast_node<TCompilationProfile: CompilationProfile>(
    object_selection: &ObjectSelection,
    object_selectable: BorrowedObjectSelectable<TCompilationProfile>,
    indentation_level: u8,
    inner_reader_ast: ReaderAst,
    initial_variable_context: &VariableContext,
    reader_imports: &mut ReaderImports,
    root_refetched_paths: &RefetchedPathsMap,
    path: &[NormalizationKey],
) -> String {
    let name = object_selection.name.item;
    let alias = object_selection
        .reader_alias
        .map(|x| format!("\"{}\"", x.item))
        .unwrap_or("null".to_string());

    let arguments = get_serialized_field_arguments(
        &transform_arguments_with_child_context(
            object_selection
                .arguments
                .iter()
                .map(|x| x.item.into_key_and_value()),
            initial_variable_context,
        ),
        indentation_level + 1,
    );
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let condition = match object_selectable {
        DefinitionLocation::Server(server_object_selectable) => {
            if server_object_selectable.is_inline_fragment.0 {
                let type_and_field = EntityNameAndSelectableName {
                    selectable_name: object_selection.name.item,
                    parent_entity_name: server_object_selectable.parent_entity_name,
                };

                let reader_artifact_import_name =
                    format!("{}__resolver_reader", type_and_field.underscore_separated());

                reader_imports.insert((type_and_field, ImportedFileCategory::ResolverReader));

                reader_artifact_import_name
            } else {
                "null".to_string()
            }
        }
        DefinitionLocation::Client(client_object_selectable) => {
            let reader_artifact_import_name = format!(
                "{}__resolver_reader",
                client_object_selectable
                    .entity_name_and_selectable_name()
                    .underscore_separated()
            );

            reader_imports.insert((
                client_object_selectable.entity_name_and_selectable_name(),
                ImportedFileCategory::ResolverReader,
            ));

            reader_artifact_import_name
        }
    };

    let is_updatable = matches!(
        object_selection.object_selection_directive_set,
        ObjectSelectionDirectiveSet::Updatable(_)
    );

    let refetch_query = match object_selectable {
        DefinitionLocation::Server(_) => "null".to_string(),
        DefinitionLocation::Client(_) => {
            let refetch_query_index = find_imperatively_fetchable_query_index(
                root_refetched_paths,
                path,
                object_selection.name.item.unchecked_conversion(),
            );

            format!("{refetch_query_index}")
        }
    };

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Linked\",\n\
        {indent_2}fieldName: \"{name}\",\n\
        {indent_2}alias: {alias},\n\
        {indent_2}arguments: {arguments},\n\
        {indent_2}condition: {condition},\n\
        {indent_2}isUpdatable: {is_updatable},\n\
        {indent_2}refetchQueryIndex: {refetch_query},\n\
        {indent_2}selections: {inner_reader_ast},\n\
        {indent_1}}},\n",
    )
}

#[expect(clippy::too_many_arguments)]
fn scalar_client_defined_field_ast_node<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    scalar_field_selection: &ScalarSelection,
    client_scalar_selectable: MemoRef<ClientScalarSelectable<TCompilationProfile>>,
    indentation_level: u8,
    path: &mut Vec<NormalizationKey>,
    root_refetched_paths: &RefetchedPathsMap,
    reader_imports: &mut ReaderImports,
    parent_variable_context: &VariableContext,
) -> String {
    let client_scalar_selectable = client_scalar_selectable.lookup(db);
    let client_scalar_selectable_variable_context = parent_variable_context.child_variable_context(
        &scalar_field_selection.arguments,
        &client_scalar_selectable.variable_definitions,
        &scalar_field_selection.scalar_selection_directive_set,
    );

    match categorize_field_loadability(
        client_scalar_selectable,
        &scalar_field_selection.scalar_selection_directive_set,
    ) {
        Some(Loadability::LoadablySelectedField(loadable_directive_parameters)) => {
            loadably_selected_field_ast_node(
                db,
                client_scalar_selectable,
                reader_imports,
                indentation_level,
                scalar_field_selection,
                &client_scalar_selectable_variable_context,
                loadable_directive_parameters,
            )
        }
        Some(Loadability::ImperativelyLoadedField(_)) => imperatively_loaded_variant_ast_node(
            client_scalar_selectable,
            reader_imports,
            root_refetched_paths,
            path,
            indentation_level,
            scalar_field_selection,
        ),
        None => match client_scalar_selectable.variant {
            ClientFieldVariant::Link => {
                link_variant_ast_node(scalar_field_selection, indentation_level)
            }
            ClientFieldVariant::UserWritten(_) | ClientFieldVariant::ImperativelyLoadedField(_) => {
                user_written_variant_ast_node(
                    db,
                    scalar_field_selection,
                    indentation_level,
                    client_scalar_selectable,
                    path,
                    root_refetched_paths,
                    reader_imports,
                    &client_scalar_selectable_variable_context,
                    parent_variable_context,
                )
            }
        },
    }
}

fn link_variant_ast_node(
    scalar_field_selection: &ScalarSelection,
    indentation_level: u8,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Link\",\n\
        {indent_2}alias: \"{alias}\",\n\
        {indent_1}}},\n",
    )
}

#[expect(clippy::too_many_arguments)]
fn user_written_variant_ast_node<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    scalar_field_selection: &ScalarSelection,
    indentation_level: u8,
    nested_client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
    path: &mut Vec<NormalizationKey>,
    root_refetched_paths: &RefetchedPathsMap,
    reader_imports: &mut ReaderImports,
    client_scalar_selectable_variable_context: &VariableContext,
    initial_variable_context: &VariableContext,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);
    // Note: this is confusing. We're using the parent context to determine the
    // arguments **to** the client field (below), and the child context (here) for
    // the refetch paths **within** the client field.
    let paths_to_refetch_field_in_client_scalar_selectable =
        refetched_paths_for_client_scalar_selectable(
            db,
            nested_client_scalar_selectable,
            path,
            client_scalar_selectable_variable_context,
        );

    let nested_refetch_queries = get_nested_refetch_query_text(
        root_refetched_paths,
        &paths_to_refetch_field_in_client_scalar_selectable,
    );

    let arguments = get_serialized_field_arguments(
        // Note: this is confusing. We're using the parent context to determine the
        // arguments **to** the client field, and the child context (above) for the
        // refetch paths **within** the client field.
        &transform_arguments_with_child_context(
            scalar_field_selection
                .arguments
                .iter()
                .map(|x| x.item.into_key_and_value()),
            initial_variable_context,
        ),
        indentation_level + 1,
    );

    let reader_artifact_import_name = format!(
        "{}__resolver_reader",
        nested_client_scalar_selectable
            .entity_name_and_selectable_name()
            .underscore_separated()
    );

    reader_imports.insert((
        nested_client_scalar_selectable.entity_name_and_selectable_name(),
        ImportedFileCategory::ResolverReader,
    ));

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Resolver\",\n\
        {indent_2}alias: \"{alias}\",\n\
        {indent_2}arguments: {arguments},\n\
        {indent_2}readerArtifact: {reader_artifact_import_name},\n\
        {indent_2}usedRefetchQueries: {nested_refetch_queries},\n\
        {indent_1}}},\n",
    )
}

fn imperatively_loaded_variant_ast_node<TCompilationProfile: CompilationProfile>(
    nested_client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
    reader_imports: &mut ReaderImports,
    root_refetched_paths: &RefetchedPathsMap,
    path: &[NormalizationKey],
    indentation_level: u8,
    scalar_field_selection: &ScalarSelection,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let refetch_reader_artifact_import_name = format!(
        "{}__refetch_reader",
        nested_client_scalar_selectable
            .entity_name_and_selectable_name()
            .underscore_separated()
    );

    reader_imports.insert((
        nested_client_scalar_selectable.entity_name_and_selectable_name(),
        ImportedFileCategory::RefetchReader,
    ));

    let refetch_query_index = find_imperatively_fetchable_query_index(
        root_refetched_paths,
        path,
        scalar_field_selection.name.item.unchecked_conversion(),
    );

    // TODO we also need to account for arguments here.
    // Note that scalar_field_selection.arguments includes an id argument, which
    // may or may not be what we want here.
    let name = scalar_field_selection.name.item;

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"ImperativelyLoadedField\",\n\
        {indent_2}alias: \"{alias}\",\n\
        {indent_2}refetchReaderArtifact: {refetch_reader_artifact_import_name},\n\
        {indent_2}refetchQueryIndex: {refetch_query_index},\n\
        {indent_2}name: \"{name}\",\n\
        {indent_1}}},\n",
    )
}

fn loadably_selected_field_ast_node<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
    reader_imports: &mut ReaderImports,
    indentation_level: u8,
    scalar_field_selection: &ScalarSelection,
    client_scalar_selectable_variable_context: &VariableContext,
    loadable_directive_parameters: &LoadableDirectiveParameters,
) -> String {
    let name = scalar_field_selection.name.item;
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let type_and_field = client_scalar_selectable
        .entity_name_and_selectable_name()
        .underscore_separated();
    let entrypoint_text = if !loadable_directive_parameters.lazy_load_artifact {
        reader_imports.insert((
            client_scalar_selectable.entity_name_and_selectable_name(),
            ImportedFileCategory::Entrypoint,
        ));
        format!("{type_and_field}__entrypoint")
    } else {
        let indent_3 = "  ".repeat((indentation_level + 2) as usize);
        let field_parent_type = client_scalar_selectable
            .entity_name_and_selectable_name()
            .parent_entity_name;
        let field_directive_set = match client_scalar_selectable.variant.reference() {
            ClientFieldVariant::UserWritten(info) => {
                info.client_scalar_selectable_directive_set.clone().expect(
                    "Expected directive set to have been validated. \
                    This is indicative of a bug in Isograph.",
                )
            }
            ClientFieldVariant::ImperativelyLoadedField(_) => {
                ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
            }
            ClientFieldVariant::Link => {
                ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
            }
        };
        let reader_artifact_kind =
            if let ClientScalarSelectableDirectiveSet::None(_) = field_directive_set {
                "EagerReaderArtifact"
            } else {
                "ComponentReaderArtifact"
            };
        format!(
            "{{\n\
            {indent_3}kind: \"EntrypointLoader\",\n\
            {indent_3}typeAndField: \"{type_and_field}\",\n\
            {indent_3}readerArtifactKind: \"{reader_artifact_kind}\",\n\
            {indent_3}loader: () => import(\"../../{field_parent_type}/{name}/entrypoint\").then(module => module.default),\n\
            {indent_2}}}"
        )
    };

    let arguments = get_serialized_field_arguments(
        &transform_arguments_with_child_context(
            scalar_field_selection
                .arguments
                .iter()
                .map(|x| x.item.into_key_and_value()),
            client_scalar_selectable_variable_context,
        ),
        indentation_level + 1,
    );

    let refetch_strategy = refetch_strategy_for_client_scalar_selectable_named(
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

    let empty_selection_set =
        SelectionSet { selections: vec![] }.with_location(EmbeddedLocation::todo_generated());
    let (reader_ast, additional_reader_imports) = generate_reader_ast(
        db,
        client_scalar_selectable.parent_entity_name,
        refetch_strategy
            .refetch_selection_set()
            .unwrap_or(&empty_selection_set),
        indentation_level + 1,
        // This is weird!
        &Default::default(),
        client_scalar_selectable_variable_context,
    );

    // N.B. additional_reader_imports will be empty for now, but at some point, we may have
    // refetch selection sets that import other things! Who knows!
    for import in additional_reader_imports {
        reader_imports.insert(import);
    }

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"LoadablySelectedField\",\n\
        {indent_2}alias: \"{alias}\",\n\
        {indent_2}name: \"{name}\",\n\
        {indent_2}queryArguments: {arguments},\n\
        {indent_2}refetchReaderAst: {reader_ast},\n\
        {indent_2}entrypoint: {entrypoint_text},\n\
        {indent_1}}},\n"
    )
}

fn server_defined_scalar_field_ast_node(
    scalar_field_selection: &ScalarSelection,
    indentation_level: u8,
    initial_variable_context: &VariableContext,
) -> String {
    let field_name = scalar_field_selection.name.item;
    let alias = scalar_field_selection
        .reader_alias
        .map(|x| format!("\"{}\"", x.item))
        .unwrap_or("null".to_string());
    let arguments = get_serialized_field_arguments(
        &transform_arguments_with_child_context(
            scalar_field_selection
                .arguments
                .iter()
                .map(|x| x.item.into_key_and_value()),
            initial_variable_context,
        ),
        indentation_level + 1,
    );
    let is_updatable = matches!(
        scalar_field_selection.scalar_selection_directive_set,
        ScalarSelectionDirectiveSet::Updatable(_)
    );
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Scalar\",\n\
        {indent_2}fieldName: \"{field_name}\",\n\
        {indent_2}alias: {alias},\n\
        {indent_2}arguments: {arguments},\n\
        {indent_2}isUpdatable: {is_updatable},\n\
        {indent_1}}},\n",
    )
}

#[expect(clippy::too_many_arguments)]
fn generate_reader_ast_with_path<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection_set: &WithEmbeddedLocation<SelectionSet>,
    indentation_level: u8,
    nested_client_scalar_selectable_imports: &mut ReaderImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable client field :(
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NormalizationKey>,
    initial_variable_context: &VariableContext,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in &selection_set.item.selections {
        let s = generate_reader_ast_node(
            db,
            parent_object_entity_name,
            item,
            indentation_level + 1,
            nested_client_scalar_selectable_imports,
            root_refetched_paths,
            path,
            initial_variable_context,
        );
        reader_ast.push_str(&s);
    }
    reader_ast.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    ReaderAst(reader_ast)
}

fn get_nested_refetch_query_text(
    root_refetched_paths: &RefetchedPathsMap,
    paths_to_refetch_fields_in_client_scalar_selectable: &[PathToRefetchField],
) -> String {
    let mut s = "[".to_string();
    for nested_refetch_query in paths_to_refetch_fields_in_client_scalar_selectable.iter() {
        let mut found_at_least_one = false;
        for index in root_refetched_paths
            .keys()
            .enumerate()
            .filter_map(|(index, (path, _))| {
                if path == nested_refetch_query {
                    Some(index)
                } else {
                    None
                }
            })
        {
            found_at_least_one = true;
            s.push_str(&format!("{index}, "));
        }

        assert!(
            found_at_least_one,
            "nested refetch query should be in root refetched paths. \
            This is indicative of a bug in Isograph."
        );
    }
    s.push(']');
    s
}

fn find_imperatively_fetchable_query_index(
    paths: &RefetchedPathsMap,
    outer_path: &[NormalizationKey],
    imperatively_fetchable_field_name: SelectableName,
) -> usize {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, ((path, _), root_refetch_path))| {
            if path.linked_fields == outer_path
                && root_refetch_path.field_name == imperatively_fetchable_field_name
            {
                Some(index)
            } else {
                None
            }
        })
        .expect(
            "Expected refetch query to be found. \
            This is indicative of a bug in Isograph.",
        )
}

pub(crate) fn generate_reader_ast<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection_set: &WithEmbeddedLocation<SelectionSet>,
    indentation_level: u8,
    // N.B. this is not root_refetched_paths when we're generating an entrypoint :(
    // ????
    root_refetched_paths: &RefetchedPathsMap,
    initial_variable_context: &VariableContext,
) -> (ReaderAst, ReaderImports) {
    let mut client_scalar_selectable_imports = BTreeSet::new();
    let reader_ast = generate_reader_ast_with_path(
        db,
        parent_object_entity_name,
        selection_set,
        indentation_level,
        &mut client_scalar_selectable_imports,
        root_refetched_paths,
        // TODO we are not starting at the root when generating ASTs for reader artifacts
        // (and in theory some entrypoints).
        &mut vec![],
        initial_variable_context,
    );
    (reader_ast, client_scalar_selectable_imports)
}

fn refetched_paths_for_client_scalar_selectable<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    nested_client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
    path: &mut Vec<NormalizationKey>,
    client_scalar_selectable_variable_context: &VariableContext,
) -> Vec<PathToRefetchField> {
    // Here, path is acting as a prefix. We will receive (for example) foo.bar, and
    // the client field may have a refetch query at baz.__refetch. In this case,
    // this method would return something containing foo.bar.baz.__refetch
    // TODO return a BTreeSet
    let path_set = refetched_paths_with_path(
        db,
        nested_client_scalar_selectable.parent_entity_name,
        &client_scalar_selectable_selection_set_for_parent_query(
            db,
            nested_client_scalar_selectable.parent_entity_name,
            nested_client_scalar_selectable.name,
        )
        .expect("Expected selection set to be valid."),
        path,
        client_scalar_selectable_variable_context,
    );

    let mut paths: Vec<_> = path_set.into_iter().collect();
    paths.sort();
    paths
}

fn refetched_paths_with_path<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    selection_set: &WithEmbeddedLocation<SelectionSet>,
    path: &mut Vec<NormalizationKey>,
    initial_variable_context: &VariableContext,
) -> HashSet<PathToRefetchField> {
    let mut paths = HashSet::default();

    for selection in &selection_set.item.selections {
        let selectable = selectable_named(db, parent_object_entity_name, selection.item.name())
            .as_ref()
            .expect("Expected parsing to have succeeded. This is indicative of a bug in Isograph.")
            .expect("Expected selectable to exist. This is indicative of a bug in Isograph.");

        match selection.item.reference() {
            SelectionType::Scalar(scalar_field_selection) => {
                let scalar_selectable = match selectable {
                    DefinitionLocation::Server(s) => {
                        let selectable = s.lookup(db);
                        let entity = server_entity_named(
                            db,
                            selectable
                                .target_entity
                                .as_ref()
                                .expect("Expected target entity to be valid.")
                                .inner()
                                .0,
                        )
                        .as_ref()
                        .expect("Expected parsing to have succeeded")
                        .expect("Expected target entity to be defined")
                        .lookup(db);

                        // TODO is this already validated?
                        entity
                            .selection_info
                            .as_scalar()
                            .expect("Expected selectable to be a scalar");

                        selectable.server_defined()
                    }
                    DefinitionLocation::Client(c) => c
                        .as_scalar()
                        .expect("Expected selectable to be a scalar")
                        .client_defined(),
                };

                match scalar_selectable {
                    DefinitionLocation::Server(_) => {
                        // Do nothing, we encountered a server field
                    }
                    DefinitionLocation::Client(client_scalar_selectable) => {
                        let client_scalar_selectable = client_scalar_selectable.lookup(db);
                        match categorize_field_loadability(
                            client_scalar_selectable,
                            &scalar_field_selection.scalar_selection_directive_set,
                        ) {
                            Some(Loadability::ImperativelyLoadedField(_)) => {
                                paths.insert(PathToRefetchField {
                                    linked_fields: path.clone(),
                                    field_name: client_scalar_selectable.name.scalar_selected(),
                                });
                            }
                            Some(Loadability::LoadablySelectedField(_)) => {
                                // Do not recurse into selections of loadable fields
                            }
                            None => {
                                let new_paths = refetched_paths_with_path(
                                    db,
                                    client_scalar_selectable.parent_entity_name,
                                    &client_scalar_selectable_selection_set_for_parent_query(
                                        db,
                                        client_scalar_selectable.parent_entity_name,
                                        client_scalar_selectable.name,
                                    )
                                    .expect("Expected selection set to be valid."),
                                    path,
                                    &initial_variable_context.child_variable_context(
                                        &scalar_field_selection.arguments,
                                        &client_scalar_selectable.variable_definitions,
                                        &ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
                                    ),
                                );

                                paths.extend(new_paths.into_iter());
                            }
                        }
                    }
                }
            }
            SelectionType::Object(object_selection) => {
                let object_selectable = match selectable {
                    DefinitionLocation::Server(s) => {
                        let selectable = s.lookup(db);
                        let entity = server_entity_named(
                            db,
                            selectable
                                .target_entity
                                .as_ref()
                                .expect("Expected target entity to be valid.")
                                .inner()
                                .0,
                        )
                        .as_ref()
                        .expect("Expected parsing to have succeeded")
                        .expect("Expected target entity to be defined")
                        .lookup(db);

                        // TODO is this already validated?
                        entity
                            .selection_info
                            .as_object()
                            .expect("Expected selectable to be an object");

                        selectable.server_defined()
                    }
                    DefinitionLocation::Client(c) => c
                        .as_object()
                        .expect("Expected selectable to be an object.")
                        .client_defined(),
                };
                match object_selectable {
                    DefinitionLocation::Client(client_object_selectable) => {
                        let client_object_selectable = client_object_selectable.lookup(db);
                        let parent_object_entity_name = client_object_selectable.parent_entity_name;
                        let client_object_selectable_name = client_object_selectable.name;
                        let new_paths = refetched_paths_with_path(
                            db,
                            client_object_selectable.target_entity_name.inner().0,
                            selectable_reader_selection_set(
                                db,
                                parent_object_entity_name,
                                client_object_selectable_name,
                            )
                            .expect("Expected selection set to be valid.")
                            .lookup(db),
                            path,
                            &initial_variable_context.child_variable_context(
                                &object_selection.arguments,
                                &client_object_selectable.variable_definitions,
                                &ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
                            ),
                        );

                        paths.extend(new_paths.into_iter());

                        let name_and_arguments = NameAndArguments {
                            // TODO use alias
                            name: object_selection.name.item,
                            arguments: transform_arguments_with_child_context(
                                object_selection
                                    .arguments
                                    .iter()
                                    .map(|x| x.item.into_key_and_value()),
                                // TODO this clearly does something, but why are we able to pass
                                // the initial variable context here??
                                initial_variable_context,
                            ),
                        };

                        paths.insert(PathToRefetchField {
                            linked_fields: path.clone(),
                            field_name: name_and_arguments.clone().object_selected(),
                        });

                        let normalization_key = NormalizationKey::ClientPointer(name_and_arguments);

                        path.push(normalization_key);

                        let new_paths = refetched_paths_with_path(
                            db,
                            client_object_selectable.target_entity_name.inner().0,
                            &object_selection.selection_set,
                            path,
                            initial_variable_context,
                        );

                        paths.extend(new_paths.into_iter());

                        path.pop();
                    }
                    DefinitionLocation::Server(server_object_selectable) => {
                        let normalization_key = if server_object_selectable.is_inline_fragment.0 {
                            NormalizationKey::InlineFragment(
                                server_object_selectable
                                    .target_entity
                                    .as_ref()
                                    .expect("Expected target entity to be valid.")
                                    .inner()
                                    .0,
                            )
                        } else {
                            NameAndArguments {
                                // TODO use alias
                                name: object_selection.name.item,
                                arguments: transform_arguments_with_child_context(
                                    object_selection
                                        .arguments
                                        .iter()
                                        .map(|x| x.item.into_key_and_value()),
                                    // TODO this clearly does something, but why are we able to pass
                                    // the initial variable context here??
                                    initial_variable_context,
                                ),
                            }
                            .normalization_key()
                        };

                        path.push(normalization_key);

                        let new_paths = refetched_paths_with_path(
                            db,
                            server_object_selectable
                                .target_entity
                                .as_ref()
                                .expect("Expected target entity to be valid.")
                                .inner()
                                .0,
                            &object_selection.selection_set,
                            path,
                            initial_variable_context,
                        );

                        paths.extend(new_paths.into_iter());

                        path.pop();
                    }
                };
            }
        };
    }

    paths
}
