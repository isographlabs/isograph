use std::collections::{BTreeSet, HashSet};

use common_lang_types::{SelectableFieldName, WithSpan};
use isograph_lang_types::{
    ClientFieldId, IsographSelectionVariant, RefetchQueryIndex, Selection, ServerFieldSelection,
};
use isograph_schema::{
    into_name_and_arguments, ArgumentKeyAndValue, ClientFieldVariant, FieldDefinitionLocation,
    NameAndArguments, PathToRefetchField, RefetchedPathsMap, ValidatedClientField,
    ValidatedLinkedFieldSelection, ValidatedScalarFieldSelection, ValidatedSchema,
    ValidatedSelection,
};

use crate::{
    generate_artifacts::{get_serialized_field_arguments, ReaderAst},
    import_statements::{ReaderImports, ResolverReaderOrRefetchResolver},
};

// Can we do this when visiting the client field in when generating entrypoints?
fn generate_reader_ast_node(
    selection: &WithSpan<ValidatedSelection>,
    schema: &ValidatedSchema,
    indentation_level: u8,
    reader_imports: &mut ReaderImports,
    // TODO use this to generate usedRefetchQueries
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NameAndArguments>,
) -> String {
    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                match scalar_field.associated_data.location {
                    FieldDefinitionLocation::Server(_) => {
                        server_defined_scalar_field_ast_node(scalar_field, indentation_level)
                    }
                    FieldDefinitionLocation::Client(client_field_id) => {
                        scalar_client_defined_field_ast_node(
                            scalar_field,
                            schema,
                            client_field_id,
                            indentation_level,
                            path,
                            root_refetched_paths,
                            reader_imports,
                        )
                    }
                }
            }
            ServerFieldSelection::LinkedField(linked_field) => {
                path.push(into_name_and_arguments(&linked_field));

                let inner_reader_ast = generate_reader_ast_with_path(
                    schema,
                    &linked_field.selection_set,
                    indentation_level + 1,
                    reader_imports,
                    root_refetched_paths,
                    path,
                );

                path.pop();

                linked_field_ast_node(linked_field, indentation_level, inner_reader_ast)
            }
        },
    }
}

fn linked_field_ast_node(
    linked_field: &ValidatedLinkedFieldSelection,
    indentation_level: u8,
    inner_reader_ast: ReaderAst,
) -> String {
    let name = linked_field.name.item;
    let alias = linked_field
        .reader_alias
        .map(|x| format!("\"{}\"", x.item))
        .unwrap_or("null".to_string());

    let arguments = get_serialized_field_arguments(
        &linked_field
            .arguments
            .iter()
            .map(|x| x.item.clone())
            .collect::<Vec<_>>(),
        indentation_level + 1,
    );
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);
    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Linked\",\n\
        {indent_2}fieldName: \"{name}\",\n\
        {indent_2}alias: {alias},\n\
        {indent_2}arguments: {arguments},\n\
        {indent_2}selections: {inner_reader_ast},\n\
        {indent_1}}},\n",
    )
}

fn scalar_client_defined_field_ast_node(
    scalar_field_selection: &ValidatedScalarFieldSelection,
    schema: &ValidatedSchema,
    client_field_id: ClientFieldId,
    indentation_level: u8,
    path: &mut Vec<NameAndArguments>,
    root_refetched_paths: &RefetchedPathsMap,
    reader_imports: &mut ReaderImports,
) -> String {
    // This field is a client field, so we need to look up the field in the
    // schema.
    let nested_client_field = schema.client_field(client_field_id);

    // This is indicative of poor data modeling.
    match nested_client_field.variant {
        ClientFieldVariant::ImperativelyLoadedField(_) => imperatively_loaded_variant_ast_node(
            nested_client_field,
            reader_imports,
            root_refetched_paths,
            path,
            indentation_level,
            scalar_field_selection,
        ),
        ClientFieldVariant::UserWritten(_) => {
            if matches!(
                scalar_field_selection.associated_data.selection_variant,
                IsographSelectionVariant::Loadable(_)
            ) {
                imperatively_loaded_variant_ast_node(
                    nested_client_field,
                    reader_imports,
                    root_refetched_paths,
                    path,
                    indentation_level,
                    scalar_field_selection,
                )
            } else {
                user_written_variant_ast_node(
                    scalar_field_selection,
                    indentation_level,
                    nested_client_field,
                    schema,
                    path,
                    root_refetched_paths,
                    reader_imports,
                )
            }
        }
    }
}

fn user_written_variant_ast_node(
    scalar_field_selection: &ValidatedScalarFieldSelection,
    indentation_level: u8,
    nested_client_field: &ValidatedClientField,
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
    root_refetched_paths: &RefetchedPathsMap,
    reader_imports: &mut ReaderImports,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);
    let paths_to_refetch_field_in_client_field =
        refetched_paths_for_client_field(nested_client_field, schema, path);

    let nested_refetch_queries = get_nested_refetch_query_text(
        &root_refetched_paths,
        &paths_to_refetch_field_in_client_field,
    );

    let arguments = get_serialized_field_arguments(
        &scalar_field_selection
            .arguments
            .iter()
            // TODO we shouldn't need to clone here
            .map(|x| x.item.clone())
            .collect::<Vec<_>>(),
        indentation_level + 1,
    );

    let reader_artifact_import_name = format!(
        "{}__resolver_reader",
        nested_client_field.type_and_field.underscore_separated()
    );

    reader_imports.insert((
        nested_client_field.type_and_field,
        ResolverReaderOrRefetchResolver::ResolverReader,
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

fn imperatively_loaded_variant_ast_node(
    nested_client_field: &ValidatedClientField,
    reader_imports: &mut ReaderImports,
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NameAndArguments>,
    indentation_level: u8,
    scalar_field_selection: &ValidatedScalarFieldSelection,
) -> String {
    let alias = scalar_field_selection.name_or_alias().item;
    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    let reader_artifact_import_name = format!(
        "{}__refetch_reader",
        nested_client_field.type_and_field.underscore_separated()
    );

    {
        let artifact_file_type = ResolverReaderOrRefetchResolver::RefetchReader;
        reader_imports.insert((nested_client_field.type_and_field, artifact_file_type));
    };
    let refetch_query_index = find_imperatively_fetchable_query_index(
        root_refetched_paths,
        path,
        scalar_field_selection.name.item.into(),
    )
    .0;

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"ImperativelyLoadedField\",\n\
        {indent_2}alias: \"{alias}\",\n\
        {indent_2}readerArtifact: {reader_artifact_import_name},\n\
        {indent_2}refetchQuery: {refetch_query_index},\n\
        {indent_1}}},\n",
    )
}

fn server_defined_scalar_field_ast_node(
    scalar_field: &ValidatedScalarFieldSelection,
    indentation_level: u8,
) -> String {
    let field_name = scalar_field.name.item;
    let alias = scalar_field
        .reader_alias
        .map(|x| format!("\"{}\"", x.item))
        .unwrap_or("null".to_string());
    let arguments = get_serialized_field_arguments(
        &scalar_field
            .arguments
            .iter()
            .map(|x| x.item.clone())
            .collect::<Vec<_>>(),
        indentation_level + 1,
    );

    let indent_1 = "  ".repeat(indentation_level as usize);
    let indent_2 = "  ".repeat((indentation_level + 1) as usize);

    format!(
        "{indent_1}{{\n\
        {indent_2}kind: \"Scalar\",\n\
        {indent_2}fieldName: \"{field_name}\",\n\
        {indent_2}alias: {alias},\n\
        {indent_2}arguments: {arguments},\n\
        {indent_1}}},\n",
    )
}

fn generate_reader_ast_with_path<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    indentation_level: u8,
    nested_client_field_imports: &mut ReaderImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable client field :(
    root_refetched_paths: &RefetchedPathsMap,
    path: &mut Vec<NameAndArguments>,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in selection_set {
        let s = generate_reader_ast_node(
            item,
            schema,
            indentation_level + 1,
            nested_client_field_imports,
            &root_refetched_paths,
            path,
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
            s.push_str(&format!("{}, ", index));
        }

        assert!(
            found_at_least_one,
            "nested refetch query should be in root refetched paths. \
            This is indicative of a bug in Isograph."
        );
    }
    s.push_str("]");
    s
}

fn find_imperatively_fetchable_query_index(
    paths: &RefetchedPathsMap,
    outer_path: &[NameAndArguments],
    imperatively_fetchable_field_name: SelectableFieldName,
) -> RefetchQueryIndex {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, ((path, _), root_refetch_path))| {
            if &path.linked_fields == outer_path
                && root_refetch_path.field_name == imperatively_fetchable_field_name
            {
                Some(RefetchQueryIndex(index as u32))
            } else {
                None
            }
        })
        .expect("Expected refetch query to be found")
}

pub(crate) fn generate_reader_ast<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    indentation_level: u8,
    // N.B. this is not root_refetched_paths when we're generating an entrypoint :(
    // ????
    root_refetched_paths: &RefetchedPathsMap,
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
    );
    (reader_ast, client_field_imports)
}

fn refetched_paths_for_client_field(
    nested_client_field: &ValidatedClientField,
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
) -> Vec<PathToRefetchField> {
    // Here, path is acting as a prefix. We will receive (for example) foo.bar, and
    // the client field may have a refetch query at baz.__refetch. In this case,
    // this method would return something containing foo.bar.baz.__refetch
    let path_set = refetched_paths_with_path(
        &*nested_client_field.selection_set_for_parent_query(),
        schema,
        path,
    );

    let mut paths: Vec<_> = path_set.into_iter().collect();
    paths.sort();
    paths
}

fn refetched_paths_with_path(
    selection_set: &[WithSpan<ValidatedSelection>],
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
) -> HashSet<PathToRefetchField> {
    let mut paths = HashSet::default();

    for selection in selection_set {
        match &selection.item {
            Selection::ServerField(field) => match field {
                ServerFieldSelection::ScalarField(scalar) => {
                    match scalar.associated_data.location {
                        FieldDefinitionLocation::Server(_) => {
                            // Do nothing, we encountered a server field
                        }
                        FieldDefinitionLocation::Client(client_field_id) => {
                            let client_field = schema.client_field(client_field_id);
                            match client_field.variant {
                                ClientFieldVariant::ImperativelyLoadedField(_) => {
                                    paths.insert(PathToRefetchField {
                                        linked_fields: path.clone(),
                                        field_name: client_field.name,
                                    });
                                }
                                _ => {
                                    // For non-refetch fields, we need to recurse into the selection set
                                    // (if there is one)
                                    let new_paths = refetched_paths_with_path(
                                        &*client_field.selection_set_for_parent_query(),
                                        schema,
                                        path,
                                    );

                                    paths.extend(new_paths.into_iter());
                                }
                            }
                        }
                    }
                }
                ServerFieldSelection::LinkedField(linked_field_selection) => {
                    path.push(NameAndArguments {
                        name: linked_field_selection.name.item.into(),
                        arguments: linked_field_selection
                            .arguments
                            .iter()
                            .map(|x| ArgumentKeyAndValue {
                                key: x.item.name.item,
                                value: x.item.value.item.clone(),
                            })
                            .collect::<Vec<_>>(),
                    });

                    let new_paths = refetched_paths_with_path(
                        &linked_field_selection.selection_set,
                        schema,
                        path,
                    );

                    paths.extend(new_paths.into_iter());

                    path.pop();
                }
            },
        };
    }

    paths
}
