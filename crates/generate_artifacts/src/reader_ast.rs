use std::collections::{BTreeSet, HashSet};

use common_lang_types::{ClientScalarSelectableName, ObjectTypeAndFieldName, WithSpan};
use isograph_lang_types::{
    DefinitionLocation, EmptyDirectiveSet, LoadableDirectiveParameters,
    ObjectSelectionDirectiveSet, RefetchQueryIndex, ScalarSelectionDirectiveSet,
    SelectionTypeContainingSelections,
};
use isograph_schema::{
    categorize_field_loadability, transform_arguments_with_child_context, ClientFieldVariant,
    ClientScalarOrObjectSelectable, ClientScalarSelectable, Loadability, NameAndArguments,
    NetworkProtocol, NormalizationKey, PathToRefetchField, RefetchedPathsMap, Schema,
    ServerObjectSelectableVariant, ValidatedObjectSelection, ValidatedScalarSelection,
    ValidatedSelection, VariableContext,
};

use crate::{
    generate_artifacts::{get_serialized_field_arguments, ReaderAst},
    import_statements::{ImportedFileCategory, ReaderImports},
};

// Can we do this when visiting the client field in when generating entrypoints?
fn generate_reader_ast_node<TNetworkProtocol: NetworkProtocol>(
    selection: &WithSpan<ValidatedSelection>,
    schema: &Schema<TNetworkProtocol>,
    indentation_level: u8,
    reader_imports: &mut ReaderImports,
    // TODO use this to generate usedRefetchQueries
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NormalizationKey>,
    initial_variable_context: &VariableContext,
) -> String {
    match &selection.item {
        SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
            match scalar_field_selection.associated_data {
                DefinitionLocation::Server(_) => server_defined_scalar_field_ast_node(
                    scalar_field_selection,
                    indentation_level,
                    initial_variable_context,
                ),
                DefinitionLocation::Client((parent_object_entity_name, client_field_name)) => {
                    let client_field =
                        schema.client_field(parent_object_entity_name, client_field_name);
                    scalar_client_defined_field_ast_node(
                        scalar_field_selection,
                        schema,
                        client_field,
                        indentation_level,
                        path,
                        root_refetched_paths,
                        reader_imports,
                        initial_variable_context,
                    )
                }
            }
        }
        SelectionTypeContainingSelections::Object(linked_field_selection) => {
            match linked_field_selection.associated_data {
                DefinitionLocation::Client((parent_object_entity_name, client_pointer_name)) => {
                    let client_pointer =
                        schema.client_pointer(parent_object_entity_name, client_pointer_name);

                    let inner_reader_ast = generate_reader_ast_with_path(
                        schema,
                        client_pointer.refetch_strategy.refetch_selection_set(),
                        indentation_level + 1,
                        reader_imports,
                        root_refetched_paths,
                        path,
                        initial_variable_context,
                    );

                    linked_field_ast_node(
                        schema,
                        linked_field_selection,
                        indentation_level,
                        inner_reader_ast,
                        initial_variable_context,
                        reader_imports,
                    )
                }
                DefinitionLocation::Server((
                    parent_object_entity_name,
                    server_object_selectable_name,
                )) => {
                    let server_object_selectable = schema.server_object_selectable(
                        parent_object_entity_name,
                        server_object_selectable_name,
                    );
                    let normalization_key = match server_object_selectable.object_selectable_variant
                    {
                        ServerObjectSelectableVariant::LinkedField => NameAndArguments {
                            // TODO use alias
                            name: linked_field_selection.name.item.into(),
                            // TODO this clearly does something, but why are we able to pass
                            // the initial variable context here??
                            arguments: transform_arguments_with_child_context(
                                linked_field_selection
                                    .arguments
                                    .iter()
                                    .map(|x| x.item.into_key_and_value()),
                                // TODO why is this not the transformed context?
                                initial_variable_context,
                            ),
                        }
                        .normalization_key(),
                        ServerObjectSelectableVariant::InlineFragment => {
                            let target_object_entity =
                                schema.server_entity_data.server_object_entity(
                                    *server_object_selectable.target_object_entity.inner(),
                                );
                            NormalizationKey::InlineFragment(target_object_entity.name)
                        }
                    };

                    path.push(normalization_key);

                    let inner_reader_ast = generate_reader_ast_with_path(
                        schema,
                        &linked_field_selection.selection_set,
                        indentation_level + 1,
                        reader_imports,
                        root_refetched_paths,
                        path,
                        initial_variable_context,
                    );

                    path.pop();

                    linked_field_ast_node(
                        schema,
                        linked_field_selection,
                        indentation_level,
                        inner_reader_ast,
                        initial_variable_context,
                        reader_imports,
                    )
                }
            }
        }
    }
}

fn linked_field_ast_node<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    linked_field: &ValidatedObjectSelection,
    indentation_level: u8,
    inner_reader_ast: ReaderAst,
    initial_variable_context: &VariableContext,
    reader_imports: &mut ReaderImports,
) -> String {
    let name = linked_field.name.item;
    let alias = linked_field
        .reader_alias
        .map(|x| format!("\"{}\"", x.item))
        .unwrap_or("null".to_string());

    let arguments = get_serialized_field_arguments(
        &transform_arguments_with_child_context(
            linked_field
                .arguments
                .iter()
                .map(|x| x.item.into_key_and_value()),
            initial_variable_context,
        ),
        indentation_level + 1,
    );
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let condition = match linked_field.associated_data {
        DefinitionLocation::Client((parent_object_entity_name, client_pointer_name)) => {
            let client_pointer =
                schema.client_pointer(parent_object_entity_name, client_pointer_name);

            let reader_artifact_import_name = format!(
                "{}__resolver_reader",
                client_pointer.type_and_field.underscore_separated()
            );

            reader_imports.insert((
                client_pointer.type_and_field,
                ImportedFileCategory::ResolverReader,
            ));

            reader_artifact_import_name
        }
        DefinitionLocation::Server((parent_object_entity_name, server_object_selectable_name)) => {
            let server_field = schema
                .server_object_selectable(parent_object_entity_name, server_object_selectable_name);
            match &server_field.object_selectable_variant {
                ServerObjectSelectableVariant::InlineFragment => {
                    let object = schema
                        .server_entity_data
                        .server_object_entity(server_field.parent_object_name);

                    let type_and_field = ObjectTypeAndFieldName {
                        field_name: linked_field.name.item.into(),
                        type_name: object.name,
                    };

                    let reader_artifact_import_name =
                        format!("{}__resolver_reader", type_and_field.underscore_separated());

                    reader_imports.insert((type_and_field, ImportedFileCategory::ResolverReader));

                    reader_artifact_import_name
                }
                ServerObjectSelectableVariant::LinkedField => "null".to_string(),
            }
        }
    };

    let is_updatable = matches!(
        linked_field.object_selection_directive_set,
        ObjectSelectionDirectiveSet::Updatable(_)
    );

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Linked\",\n\
        {indent_2}fieldName: \"{name}\",\n\
        {indent_2}alias: {alias},\n\
        {indent_2}arguments: {arguments},\n\
        {indent_2}condition: {condition},\n\
        {indent_2}isUpdatable: {is_updatable},\n\
        {indent_2}selections: {inner_reader_ast},\n\
        {indent_2}refetchQueryIndex: null,\n\
        {indent_1}}},\n",
    )
}

#[allow(clippy::too_many_arguments)]
fn scalar_client_defined_field_ast_node<TNetworkProtocol: NetworkProtocol>(
    scalar_field_selection: &ValidatedScalarSelection,
    schema: &Schema<TNetworkProtocol>,
    client_field: &ClientScalarSelectable<TNetworkProtocol>,
    indentation_level: u8,
    path: &mut Vec<NormalizationKey>,
    root_refetched_paths: &RefetchedPathsMap,
    reader_imports: &mut ReaderImports,
    parent_variable_context: &VariableContext,
) -> String {
    let client_field_variable_context = parent_variable_context.child_variable_context(
        &scalar_field_selection.arguments,
        &client_field.variable_definitions,
        &scalar_field_selection.scalar_selection_directive_set,
    );

    match categorize_field_loadability(
        client_field,
        &scalar_field_selection.scalar_selection_directive_set,
    ) {
        Some(Loadability::LoadablySelectedField(loadable_directive_parameters)) => {
            loadably_selected_field_ast_node(
                schema,
                client_field,
                reader_imports,
                indentation_level,
                scalar_field_selection,
                &client_field_variable_context,
                loadable_directive_parameters,
            )
        }
        Some(Loadability::ImperativelyLoadedField(_)) => imperatively_loaded_variant_ast_node(
            client_field,
            reader_imports,
            root_refetched_paths,
            path,
            indentation_level,
            scalar_field_selection,
        ),
        None => match client_field.variant {
            ClientFieldVariant::Link => {
                link_variant_ast_node(scalar_field_selection, indentation_level)
            }
            ClientFieldVariant::UserWritten(_) | ClientFieldVariant::ImperativelyLoadedField(_) => {
                user_written_variant_ast_node(
                    scalar_field_selection,
                    indentation_level,
                    client_field,
                    schema,
                    path,
                    root_refetched_paths,
                    reader_imports,
                    &client_field_variable_context,
                    parent_variable_context,
                )
            }
        },
    }
}

fn link_variant_ast_node(
    scalar_field_selection: &ValidatedScalarSelection,
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

#[allow(clippy::too_many_arguments)]
fn user_written_variant_ast_node<TNetworkProtocol: NetworkProtocol>(
    scalar_field_selection: &ValidatedScalarSelection,
    indentation_level: u8,
    nested_client_field: &ClientScalarSelectable<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    path: &mut Vec<NormalizationKey>,
    root_refetched_paths: &RefetchedPathsMap,
    reader_imports: &mut ReaderImports,
    client_field_variable_context: &VariableContext,
    initial_variable_context: &VariableContext,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);
    // Note: this is confusing. We're using the parent context to determine the
    // arguments **to** the client field (below), and the child context (here) for
    // the refetch paths **within** the client field.
    let paths_to_refetch_field_in_client_field = refetched_paths_for_client_field(
        nested_client_field,
        schema,
        path,
        client_field_variable_context,
    );

    let nested_refetch_queries = get_nested_refetch_query_text(
        root_refetched_paths,
        &paths_to_refetch_field_in_client_field,
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
        nested_client_field.type_and_field.underscore_separated()
    );

    reader_imports.insert((
        nested_client_field.type_and_field,
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

#[allow(clippy::too_many_arguments)]
fn imperatively_loaded_variant_ast_node<TNetworkProtocol: NetworkProtocol>(
    nested_client_field: &ClientScalarSelectable<TNetworkProtocol>,
    reader_imports: &mut ReaderImports,
    root_refetched_paths: &RefetchedPathsMap,
    path: &[NormalizationKey],
    indentation_level: u8,
    scalar_field_selection: &ValidatedScalarSelection,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let refetch_reader_artifact_import_name = format!(
        "{}__refetch_reader",
        nested_client_field.type_and_field.underscore_separated()
    );

    reader_imports.insert((
        nested_client_field.type_and_field,
        ImportedFileCategory::RefetchReader,
    ));

    let refetch_query_index = find_imperatively_fetchable_query_index(
        root_refetched_paths,
        path,
        scalar_field_selection.name.item.unchecked_conversion(),
    )
    .0;

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

fn loadably_selected_field_ast_node<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    client_field: &ClientScalarSelectable<TNetworkProtocol>,
    reader_imports: &mut ReaderImports,
    indentation_level: u8,
    scalar_field_selection: &ValidatedScalarSelection,
    client_field_variable_context: &VariableContext,
    loadable_directive_parameters: &LoadableDirectiveParameters,
) -> String {
    let name = scalar_field_selection.name.item;
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let type_and_field = client_field.type_and_field.underscore_separated();
    let entrypoint_text = if !loadable_directive_parameters.lazy_load_artifact {
        reader_imports.insert((
            client_field.type_and_field,
            ImportedFileCategory::Entrypoint,
        ));
        format!("{type_and_field}__entrypoint")
    } else {
        let indent_3 = "  ".repeat((indentation_level + 2) as usize);
        let field_parent_type = client_field.type_and_field.type_name;
        format!(
            "{{ \n\
            {indent_3}kind: \"EntrypointLoader\",\n\
            {indent_3}typeAndField: \"{type_and_field}\",\n\
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
            client_field_variable_context,
        ),
        indentation_level + 1,
    );

    let (reader_ast, additional_reader_imports) = generate_reader_ast(
        schema,
        client_field
            .refetch_strategy
            .as_ref()
            .expect(
                "Expected refetch strategy. \
                    This is indicative of a bug in Isograph.",
            )
            .refetch_selection_set(),
        indentation_level + 1,
        // This is weird!
        &Default::default(),
        client_field_variable_context,
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
    scalar_field_selection: &ValidatedScalarSelection,
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

fn generate_reader_ast_with_path<'schema, TNetworkProtocol: NetworkProtocol>(
    schema: &'schema Schema<TNetworkProtocol>,
    selection_set: &'schema [WithSpan<ValidatedSelection>],
    indentation_level: u8,
    nested_client_field_imports: &mut ReaderImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable client field :(
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NormalizationKey>,
    initial_variable_context: &VariableContext,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in selection_set {
        let s = generate_reader_ast_node(
            item,
            schema,
            indentation_level + 1,
            nested_client_field_imports,
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
    paths_to_refetch_fields_in_client_field: &[PathToRefetchField],
) -> String {
    let mut s = "[".to_string();
    for nested_refetch_query in paths_to_refetch_fields_in_client_field.iter() {
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
    imperatively_fetchable_field_name: ClientScalarSelectableName,
) -> RefetchQueryIndex {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, ((path, _), root_refetch_path))| {
            if path.linked_fields == outer_path
                && root_refetch_path.field_name == imperatively_fetchable_field_name
            {
                Some(RefetchQueryIndex(index as u32))
            } else {
                None
            }
        })
        .expect(
            "Expected refetch query to be found. \
            This is indicative of a bug in Isograph.",
        )
}

pub(crate) fn generate_reader_ast<'schema, TNetworkProtocol: NetworkProtocol>(
    schema: &'schema Schema<TNetworkProtocol>,
    selection_set: &'schema [WithSpan<ValidatedSelection>],
    indentation_level: u8,
    // N.B. this is not root_refetched_paths when we're generating an entrypoint :(
    // ????
    root_refetched_paths: &RefetchedPathsMap,
    initial_variable_context: &VariableContext,
) -> (ReaderAst, ReaderImports) {
    let mut client_field_imports = BTreeSet::new();
    let reader_ast = generate_reader_ast_with_path(
        schema,
        selection_set,
        indentation_level,
        &mut client_field_imports,
        root_refetched_paths,
        // TODO we are not starting at the root when generating ASTs for reader artifacts
        // (and in theory some entrypoints).
        &mut vec![],
        initial_variable_context,
    );
    (reader_ast, client_field_imports)
}

fn refetched_paths_for_client_field<TNetworkProtocol: NetworkProtocol>(
    nested_client_field: &ClientScalarSelectable<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    path: &mut Vec<NormalizationKey>,
    client_field_variable_context: &VariableContext,
) -> Vec<PathToRefetchField> {
    // Here, path is acting as a prefix. We will receive (for example) foo.bar, and
    // the client field may have a refetch query at baz.__refetch. In this case,
    // this method would return something containing foo.bar.baz.__refetch
    // TODO return a BTreeSet
    let path_set = refetched_paths_with_path(
        nested_client_field.selection_set_for_parent_query(),
        schema,
        path,
        client_field_variable_context,
    );

    let mut paths: Vec<_> = path_set.into_iter().collect();
    paths.sort();
    paths
}

fn refetched_paths_with_path<TNetworkProtocol: NetworkProtocol>(
    selection_set: &[WithSpan<ValidatedSelection>],
    schema: &Schema<TNetworkProtocol>,
    path: &mut Vec<NormalizationKey>,
    initial_variable_context: &VariableContext,
) -> HashSet<PathToRefetchField> {
    let mut paths = HashSet::default();

    for selection in selection_set {
        match &selection.item {
            SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
                match scalar_field_selection.associated_data {
                    DefinitionLocation::Server(_) => {
                        // Do nothing, we encountered a server field
                    }
                    DefinitionLocation::Client((parent_object_entity_name, client_field_name)) => {
                        let client_field =
                            schema.client_field(parent_object_entity_name, client_field_name);
                        match categorize_field_loadability(
                            client_field,
                            &scalar_field_selection.scalar_selection_directive_set,
                        ) {
                            Some(Loadability::ImperativelyLoadedField(_)) => {
                                paths.insert(PathToRefetchField {
                                    linked_fields: path.clone(),
                                    field_name: client_field.name,
                                });
                            }
                            Some(Loadability::LoadablySelectedField(_)) => {
                                // Do not recurse into selections of loadable fields
                            }
                            None => {
                                let new_paths = refetched_paths_with_path(
                                    client_field.selection_set_for_parent_query(),
                                    schema,
                                    path,
                                    &initial_variable_context.child_variable_context(
                                        &scalar_field_selection.arguments,
                                        &client_field.variable_definitions,
                                        &ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
                                    ),
                                );

                                paths.extend(new_paths.into_iter());
                            }
                        }
                    }
                }
            }
            SelectionTypeContainingSelections::Object(linked_field_selection) => {
                match linked_field_selection.associated_data {
                    DefinitionLocation::Client(_) => {
                        // Do not recurse into selections of client pointers
                    }
                    DefinitionLocation::Server((
                        parent_object_entity_name,
                        server_object_selectable_name,
                    )) => {
                        let server_object_selectable = schema.server_object_selectable(
                            parent_object_entity_name,
                            server_object_selectable_name,
                        );
                        let normalization_key =
                            match server_object_selectable.object_selectable_variant {
                                ServerObjectSelectableVariant::LinkedField => NameAndArguments {
                                    // TODO use alias
                                    name: linked_field_selection.name.item.into(),
                                    arguments: transform_arguments_with_child_context(
                                        linked_field_selection
                                            .arguments
                                            .iter()
                                            .map(|x| x.item.into_key_and_value()),
                                        // TODO this clearly does something, but why are we able to pass
                                        // the initial variable context here??
                                        initial_variable_context,
                                    ),
                                }
                                .normalization_key(),
                                ServerObjectSelectableVariant::InlineFragment => {
                                    let target_object_entity =
                                        schema.server_entity_data.server_object_entity(
                                            *server_object_selectable.target_object_entity.inner(),
                                        );
                                    NormalizationKey::InlineFragment(target_object_entity.name)
                                }
                            };

                        path.push(normalization_key);

                        let new_paths = refetched_paths_with_path(
                            &linked_field_selection.selection_set,
                            schema,
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
