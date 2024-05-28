use std::collections::{hash_map::Entry, HashSet};

use common_lang_types::{ArtifactFileType, JavascriptVariableName, SelectableFieldName, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{RefetchQueryIndex, Selection, ServerFieldSelection};
use isograph_schema::{
    into_name_and_arguments, ArgumentKeyAndValue, ClientFieldVariant, FieldDefinitionLocation,
    NameAndArguments, PathToRefetchField, RootRefetchedPath, ValidatedClientField, ValidatedSchema,
    ValidatedSelection,
};

use crate::generate_artifacts::{
    get_serialized_field_arguments, JavaScriptImports, NestedClientFieldImportKey,
    NestedClientFieldImports, ReaderAst, SourceArtifact, REFETCH_READER, RESOLVER_READER,
};

fn generate_reader_ast_node(
    selection: &WithSpan<ValidatedSelection>,
    schema: &ValidatedSchema,
    indentation_level: u8,
    nested_client_field_imports: &mut NestedClientFieldImports,
    // TODO use this to generate usedRefetchQueries
    root_refetched_paths: &[RootRefetchedPath],
    path: &mut Vec<NameAndArguments>,
) -> String {
    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                let field_name = scalar_field.name.item;

                match scalar_field.associated_data.location {
                    FieldDefinitionLocation::Server(_) => {
                        let alias = scalar_field
                            .reader_alias
                            .map(|x| format!("\"{}\"", x.item))
                            .unwrap_or("null".to_string());
                        let arguments = get_serialized_field_arguments(
                            &scalar_field.arguments,
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
                    FieldDefinitionLocation::Client(client_field_id) => {
                        // This field is a client field, so we need to look up the field in the
                        // schema.
                        let alias = scalar_field.name_or_alias().item;
                        let nested_client_field = schema.client_field(client_field_id);
                        let arguments = get_serialized_field_arguments(
                            &scalar_field.arguments,
                            indentation_level + 1,
                        );
                        let indent_1 = "  ".repeat(indentation_level as usize);
                        let indent_2 = "  ".repeat((indentation_level + 1) as usize);

                        let paths_to_refetch_field_in_client_field =
                            refetched_paths_for_client_field(nested_client_field, schema, path);

                        let nested_refetch_queries = get_nested_refetch_query_text(
                            &root_refetched_paths,
                            &paths_to_refetch_field_in_client_field,
                        );

                        // This is indicative of poor data modeling.
                        match nested_client_field.variant {
                            ClientFieldVariant::ImperativelyLoadedField(ref s) => {
                                let reader_artifact_import_name = format!(
                                    "{}__refetch_reader",
                                    nested_client_field.type_and_field.underscore_separated()
                                )
                                .intern()
                                .into();
                                insert_default_import_into_nested_client_field_imports(
                                    nested_client_field_imports,
                                    nested_client_field,
                                    *REFETCH_READER,
                                    reader_artifact_import_name,
                                );
                                let refetch_query_index = find_imperatively_fetchable_query_index(
                                    root_refetched_paths,
                                    path,
                                    s.client_field_scalar_selection_name.into(),
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
                            ClientFieldVariant::UserWritten(_) => {
                                let reader_artifact_import_name = format!(
                                    "{}__resolver_reader",
                                    nested_client_field.type_and_field.underscore_separated()
                                )
                                .intern()
                                .into();
                                insert_default_import_into_nested_client_field_imports(
                                    nested_client_field_imports,
                                    nested_client_field,
                                    *RESOLVER_READER,
                                    reader_artifact_import_name,
                                );
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
                        }
                    }
                }
            }
            ServerFieldSelection::LinkedField(linked_field) => {
                let name = linked_field.name.item;
                let alias = linked_field
                    .reader_alias
                    .map(|x| format!("\"{}\"", x.item))
                    .unwrap_or("null".to_string());

                path.push(into_name_and_arguments(&linked_field));

                let inner_reader_ast = generate_reader_ast_with_path(
                    schema,
                    &linked_field.selection_set,
                    indentation_level + 1,
                    nested_client_field_imports,
                    root_refetched_paths,
                    path,
                );

                path.pop();

                let arguments =
                    get_serialized_field_arguments(&linked_field.arguments, indentation_level + 1);
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
        },
    }
}

fn insert_default_import_into_nested_client_field_imports(
    nested_client_field_imports: &mut NestedClientFieldImports,
    client_field: &ValidatedClientField,
    artifact_file_type: ArtifactFileType,
    reader_artifact_import_name: JavascriptVariableName,
) {
    match nested_client_field_imports.entry(NestedClientFieldImportKey {
        object_type_and_field: client_field.type_and_field,
        source_artifact: SourceArtifact::ResolverOrRefetchReader,
        artifact_file_type,
    }) {
        Entry::Occupied(mut occupied) => {
            occupied.get_mut().default_import = Some(reader_artifact_import_name);
        }
        Entry::Vacant(vacant) => {
            vacant.insert(JavaScriptImports {
                default_import: Some(reader_artifact_import_name),
                types: vec![],
            });
        }
    }
}

fn generate_reader_ast_with_path<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    indentation_level: u8,
    nested_client_field_imports: &mut NestedClientFieldImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable client field :(
    root_refetched_paths: &[RootRefetchedPath],
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
    root_refetched_paths: &[RootRefetchedPath],
    paths_to_refetch_fields_in_client_field: &[PathToRefetchField],
) -> String {
    let mut s = "[".to_string();
    for nested_refetch_query in paths_to_refetch_fields_in_client_field.iter() {
        let mut found_at_least_one = false;
        for index in root_refetched_paths
            .iter()
            .enumerate()
            .filter_map(|(index, root_path)| {
                if root_path.path == *nested_refetch_query {
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
    paths: &[RootRefetchedPath],
    path: &[NameAndArguments],
    imperatively_fetchable_field_name: SelectableFieldName,
) -> RefetchQueryIndex {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, path_to_field)| {
            if &path_to_field.path.linked_fields == path
                && path_to_field.field_name == imperatively_fetchable_field_name
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
    nested_client_field_imports: &mut NestedClientFieldImports,
    // N.B. this is not root_refetched_paths when we're generating an entrypoint :(
    root_refetched_paths: &[RootRefetchedPath],
) -> ReaderAst {
    generate_reader_ast_with_path(
        schema,
        selection_set,
        indentation_level,
        nested_client_field_imports,
        root_refetched_paths,
        // TODO we are not starting at the root when generating ASTs for reader artifacts
        // (and in theory some entrypoints).
        &mut vec![],
    )
}

fn refetched_paths_for_client_field(
    nested_client_field: &ValidatedClientField,
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
) -> Vec<PathToRefetchField> {
    let path_set = match &nested_client_field.selection_set_and_unwraps {
        Some((selection_set, _)) => {
            // Here, path is acting as a prefix. We will recieve (for example) foo.bar, and
            // the client field may have a refetch query at baz.__refetch. In this case,
            // this method would return something containing foo.bar.baz.__refetch
            refetched_paths_with_path(&selection_set, schema, path)
        }
        None => panic!("unexpected non-existent selection set on client field"),
    };
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
                                    match &client_field.selection_set_and_unwraps {
                                        Some((selection_set, _unwraps)) => {
                                            let new_paths = refetched_paths_with_path(
                                                selection_set,
                                                schema,
                                                path,
                                            );

                                            paths.extend(new_paths.into_iter());
                                        }
                                        None => panic!("Client field has no selection set"),
                                    };
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
