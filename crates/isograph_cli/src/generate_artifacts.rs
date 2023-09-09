use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug, Display},
    io,
    path::PathBuf,
};

use common_lang_types::{
    HasName, IsographObjectTypeName, QueryOperationName, SelectableFieldName, Span,
    UnvalidatedTypeName, VariableName, WithSpan,
};
use graphql_lang_types::{
    InputValueDefinition, ListTypeAnnotation, NamedTypeAnnotation, NonNullTypeAnnotation,
    TypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    InputTypeId, NonConstantValue, ObjectId, OutputTypeId, Selection, SelectionFieldArgument,
    ServerFieldSelection, VariableDefinition,
};
use isograph_schema::{
    create_merged_selection_set, into_name_and_arguments, refetched_paths_for_resolver,
    ArtifactQueueItem, DefinedField, MergedLinkedFieldSelection, MergedScalarFieldSelection,
    MergedSelectionSet, MergedServerFieldSelection, MutationFieldResolverInfo, NameAndArguments,
    PathToRefetchField, RefetchFieldResolverInfo, ResolverActionKind, ResolverArtifactKind,
    ResolverTypeAndField, ResolverVariant, SchemaObject, ValidatedEncounteredDefinedField,
    ValidatedScalarDefinedField, ValidatedSchema, ValidatedSchemaObject, ValidatedSchemaResolver,
    ValidatedSelection, ValidatedVariableDefinition,
};
use thiserror::Error;

use crate::write_artifacts::write_artifacts;

type NestedResolverImports = HashMap<ResolverTypeAndField, ResolverImport>;

macro_rules! derive_display {
    ($type:ident) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

// TODO move to another module
pub(crate) fn generate_artifacts(
    schema: &ValidatedSchema,
    project_root: &PathBuf,
) -> Result<(), GenerateArtifactsError> {
    write_artifacts(get_all_artifacts(schema), project_root)?;

    Ok(())
}

/// get all artifacts that we must generate according to the following rough plan:
/// - initially, we know we must generate artifacts for each resolver
/// - we must also generate an artifact for each refetch field we encounter while
///   generating an artifact for a fetchable resolver (TODO)
///
/// We do this by keeping a queue of artifacts to generate, and adding to the queue
/// as we process fetchable resolvers.
fn get_all_artifacts<'schema>(schema: &'schema ValidatedSchema) -> Vec<Artifact<'schema>> {
    let mut artifact_queue: Vec<_> = schema
        .resolvers
        .iter()
        .map(ArtifactQueueItem::Resolver)
        .collect();

    let mut artifacts = vec![];
    while let Some(queue_item) = artifact_queue.pop() {
        artifacts.push(generate_artifact(queue_item, schema, &mut artifact_queue));
    }

    artifacts
}

fn generate_artifact<'schema>(
    queue_item: ArtifactQueueItem<'schema>,
    schema: &'schema ValidatedSchema,
    artifact_queue: &mut Vec<ArtifactQueueItem<'schema>>,
) -> Artifact<'schema> {
    match queue_item {
        ArtifactQueueItem::Resolver(resolver) => {
            get_artifact_for_resolver(resolver, schema, artifact_queue)
        }
        ArtifactQueueItem::RefetchField(refetch_info) => {
            get_artifact_for_refetch_field(schema, refetch_info)
        }
        ArtifactQueueItem::MutationField(mutation_info) => {
            get_artifact_for_mutation_field(schema, mutation_info)
        }
    }
}

// N.B. this was originally copied from generate_fetchable_resolver_artifact,
// and it could use some de-duplicatoni
fn get_artifact_for_refetch_field<'schema>(
    schema: &'schema ValidatedSchema,
    refetch_info: RefetchFieldResolverInfo,
) -> Artifact<'schema> {
    let RefetchFieldResolverInfo {
        merged_selection_set,
        refetch_field_parent_id: parent_id,
        variable_definitions,
        root_fetchable_field,
        root_parent_object,
        refetch_query_index,
        ..
    } = refetch_info;

    let parent_object = schema.schema_data.object(parent_id);

    // --------- HACK ---------
    // Merged selection sets do not support type refinements, so for now,
    // we are hard-coding the outside of the query text (which includes
    // `... on Type`) and the outside of the normalization AST (which can
    // ignore the type refinement for now.)
    let query_text = generate_refetchable_query_text(
        parent_object,
        schema,
        &merged_selection_set,
        variable_definitions,
    );

    let normalization_ast = NormalizationAst(format!(
        "[{{ kind: \"Linked\", fieldName: \"node\", \
        alias: null, arguments: [{{ argumentName: \"id\", variableName: \"id\" }}], \
        selections: {} }}]",
        generate_normalization_ast(schema, &merged_selection_set, 0).0,
    ));
    // ------- END HACK -------

    Artifact::RefetchQuery(RefetchQueryResolver {
        normalization_ast,
        query_text,
        root_fetchable_field,
        root_fetchable_field_parent_object: root_parent_object,
        refetch_query_index,
    })
}

fn get_artifact_for_mutation_field<'schema>(
    schema: &'schema ValidatedSchema,
    refetch_info: MutationFieldResolverInfo,
) -> Artifact<'schema> {
    let MutationFieldResolverInfo {
        merged_selection_set,
        refetch_field_parent_id: parent_id,
        variable_definitions,
        root_fetchable_field,
        root_parent_object,
        refetch_query_index,
        mutation_field_name: magic_mutation_field_name,
        mutation_primary_field_name,
        mutation_field_arguments,
        ..
    } = refetch_info;

    let parent_object = schema.schema_data.object(parent_id);

    let query_text = generate_mutation_query_text(
        parent_object,
        schema,
        &merged_selection_set,
        variable_definitions,
        magic_mutation_field_name,
        mutation_primary_field_name,
        mutation_field_arguments,
    );

    // HACK
    // TODO pass the non-magical name
    let mutation_field_name = magic_mutation_field_name.lookup()[2..].to_string();
    // END HACK

    // TODO this is incorrect, fieldName and arguments are wrong, and this
    // needs to be two-layered
    let selections = generate_normalization_ast(schema, &merged_selection_set, 2);
    let arguments = "[{ argumentName: \"id\", variableName: \"id\" }]";
    let space_2 = "  ";
    let space_4 = "    ";
    let space_6 = "      ";
    let normalization_ast = NormalizationAst(format!(
        "[{{\n\
        {space_2}kind: \"Linked\",\n\
        {space_2}fieldName: \"{mutation_field_name}\",\n\
        {space_2}arguments: {arguments},\n\
        {space_2}selections: [\n\
        {space_4}{{\n\
        {space_6}kind: \"Linked\",\n\
        {space_6}fieldName: \"{mutation_primary_field_name}\",\n\
        {space_6}arguments: null,\n\
        {space_6}selections: {selections},\n\
        {space_4}}},\n\
        {space_2}],\n\
        }}]",
    ));

    Artifact::RefetchQuery(RefetchQueryResolver {
        normalization_ast,
        query_text,
        root_fetchable_field,
        root_fetchable_field_parent_object: root_parent_object,
        refetch_query_index,
    })
}

fn generate_refetchable_query_text<'schema>(
    parent_object_type: &'schema ValidatedSchemaObject,
    schema: &'schema ValidatedSchema,
    merged_selection_set: &MergedSelectionSet,
    mut variable_definitions: Vec<WithSpan<ValidatedVariableDefinition>>,
) -> QueryText {
    let mut query_text = String::new();

    variable_definitions.push(WithSpan {
        item: VariableDefinition {
            name: WithSpan::new("id".intern().into(), Span::new(0, 0)),
            type_: TypeAnnotation::NonNull(Box::new(NonNullTypeAnnotation::Named(
                NamedTypeAnnotation(WithSpan {
                    item: InputTypeId::Scalar(schema.id_type_id),
                    span: Span::new(0, 0),
                }),
            ))),
        },
        span: Span::new(0, 0),
    });
    let variable_text = write_variables_to_string(schema, variable_definitions.iter());

    query_text.push_str(&format!(
        "query {}_refetch {} {{ node____id___id: node(id: $id) {{ ... on {} {{ \\\n",
        parent_object_type.name, variable_text, parent_object_type.name,
    ));
    write_selections_for_query_text(&mut query_text, schema, &merged_selection_set, 1);
    query_text.push_str("}}}");
    QueryText(query_text)
}

fn generate_mutation_query_text<'schema>(
    parent_object_type: &'schema ValidatedSchemaObject,
    schema: &'schema ValidatedSchema,
    merged_selection_set: &MergedSelectionSet,
    mut variable_definitions: Vec<WithSpan<ValidatedVariableDefinition>>,
    mutation_field_name: SelectableFieldName,
    mutation_primary_field_name: SelectableFieldName,
    mutation_field_arguments: Vec<WithSpan<InputValueDefinition>>,
) -> QueryText {
    let mut query_text = String::new();

    variable_definitions.push(WithSpan {
        item: VariableDefinition {
            name: WithSpan::new("id".intern().into(), Span::new(0, 0)),
            type_: TypeAnnotation::NonNull(Box::new(NonNullTypeAnnotation::Named(
                NamedTypeAnnotation(WithSpan {
                    item: InputTypeId::Scalar(schema.id_type_id),
                    span: Span::new(0, 0),
                }),
            ))),
        },
        span: Span::new(0, 0),
    });

    let mutation_parameters: Vec<_> = mutation_field_arguments.iter().map(|argument| {
        let variable_name = argument.item.name.map(|x| x.into());
        variable_definitions.push(WithSpan {
            item: VariableDefinition {
                name: variable_name,
                type_: argument.item.type_.clone().map(|x| {
                    schema
                        .schema_data
                        .defined_types
                        .get(&x.into())
                        .expect("Expected type to be found, this indicates a bug in Isograph")
                        .as_input_type_id()
                        .expect("Expected a valid input type. Objects are not yet supported as parameters here.")
                }),
            },
            span: Span::new(0, 0),
        });
        WithSpan::new(SelectionFieldArgument {
            name: argument.item.name.map(|x| x.into()),
            value: variable_name.map(|x| NonConstantValue::Variable(x)),
        }, Span::new(0,0))
    }).collect();

    let variable_text = write_variables_to_string(schema, &mut variable_definitions.iter());
    let mutation_field_arguments = get_serialized_arguments_for_query_text(&mutation_parameters);

    query_text.push_str(&format!(
        "mutation {}{} {} {{ {}{} {{ {} {{ \\\n",
        parent_object_type.name,
        mutation_field_name,
        variable_text,
        mutation_field_name,
        mutation_field_arguments,
        mutation_primary_field_name,
    ));
    write_selections_for_query_text(&mut query_text, schema, &merged_selection_set, 1);
    query_text.push_str("}}}");
    QueryText(query_text)
}

fn get_artifact_for_resolver<'schema>(
    resolver: &'schema ValidatedSchemaResolver,
    schema: &'schema ValidatedSchema,
    artifact_queue: &mut Vec<ArtifactQueueItem<'schema>>,
) -> Artifact<'schema> {
    match resolver.artifact_kind {
        ResolverArtifactKind::FetchableOnQuery => Artifact::FetchableResolver(
            generate_fetchable_resolver_artifact(schema, resolver, artifact_queue),
        ),
        ResolverArtifactKind::NonFetchable => Artifact::NonFetchableResolver(
            generate_non_fetchable_resolver_artifact(schema, resolver),
        ),
    }
}

fn generate_fetchable_resolver_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    fetchable_resolver: &ValidatedSchemaResolver,
    artifact_queue: &mut Vec<ArtifactQueueItem<'schema>>,
) -> FetchableResolver<'schema> {
    if let Some((ref selection_set, _)) = fetchable_resolver.selection_set_and_unwraps {
        let query_name = fetchable_resolver.name.into();

        let (merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            // TODO here we are assuming that the resolver is only on the Query type.
            // That restriction should be loosened.
            schema
                .schema_data
                .object(schema.query_type_id.expect("expect query type to exist"))
                .into(),
            selection_set,
            artifact_queue,
            &fetchable_resolver,
        );

        let query_object = schema
            .query_object()
            .expect("Expected query object to exist");
        let query_text = generate_query_text(
            query_name,
            schema,
            &merged_selection_set,
            &fetchable_resolver.variable_definitions,
        );
        let mut nested_resolver_artifact_imports = HashMap::new();
        let resolver_parameter_type = generate_resolver_parameter_type(
            schema,
            &selection_set,
            fetchable_resolver.variant.as_ref(),
            query_object.into(),
            &mut nested_resolver_artifact_imports,
            0,
        );
        let refetch_query_artifact_imports =
            generate_refetch_query_artifact_imports(&root_refetched_paths, fetchable_resolver.name);
        let resolver_import_statement =
            generate_resolver_import_statement(fetchable_resolver.action_kind);
        let resolver_return_type =
            generate_resolver_return_type_declaration(fetchable_resolver.action_kind);
        let resolver_read_out_type = generate_read_out_type(fetchable_resolver);
        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            0,
            &mut nested_resolver_artifact_imports,
            &root_refetched_paths,
        );
        let convert_function =
            generate_convert_function(&fetchable_resolver.variant, fetchable_resolver.name);

        let normalization_ast = generate_normalization_ast(schema, &merged_selection_set, 0);

        FetchableResolver {
            query_text,
            query_name,
            parent_type: query_object.into(),
            resolver_parameter_type,
            resolver_import_statement,
            resolver_return_type,
            resolver_read_out_type,
            reader_ast,
            nested_resolver_artifact_imports,
            convert_function,
            normalization_ast,
            refetch_query_artifact_import: refetch_query_artifact_imports,
        }
    } else {
        // TODO convert to error
        todo!("Unsupported: resolvers on query with no selection set")
    }
}

fn generate_non_fetchable_resolver_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    non_fetchable_resolver: &ValidatedSchemaResolver,
) -> NonFetchableResolver<'schema> {
    if let Some((selection_set, _)) = &non_fetchable_resolver.selection_set_and_unwraps {
        let parent_type = schema
            .schema_data
            .object(non_fetchable_resolver.parent_object_id);
        let mut nested_resolver_artifact_imports = HashMap::new();

        let (_merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            // TODO here we are assuming that the resolver is only on the Query type.
            // That restriction should be loosened.
            schema
                .schema_data
                .object(schema.query_type_id.expect("expect query type to exist"))
                .into(),
            selection_set,
            // TODO obviously a smell
            &mut vec![],
            &non_fetchable_resolver,
        );

        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            0,
            &mut nested_resolver_artifact_imports,
            &root_refetched_paths,
        );

        let resolver_parameter_type = generate_resolver_parameter_type(
            schema,
            &selection_set,
            non_fetchable_resolver.variant.as_ref(),
            parent_type.into(),
            &mut nested_resolver_artifact_imports,
            0,
        );
        let resolver_return_type =
            generate_resolver_return_type_declaration(non_fetchable_resolver.action_kind);
        let resolver_read_out_type = generate_read_out_type(non_fetchable_resolver);
        let resolver_import_statement =
            generate_resolver_import_statement(non_fetchable_resolver.action_kind);
        NonFetchableResolver {
            parent_type: parent_type.into(),
            resolver_field_name: non_fetchable_resolver.name,
            reader_ast,
            nested_resolver_artifact_imports,
            resolver_import_statement,
            resolver_read_out_type,
            resolver_parameter_type,
            resolver_return_type,
        }
    } else {
        panic!("Unsupported: resolvers not on query with no selection set")
    }
}

#[derive(Debug)]
pub(crate) enum Artifact<'schema> {
    FetchableResolver(FetchableResolver<'schema>),
    NonFetchableResolver(NonFetchableResolver<'schema>),
    RefetchQuery(RefetchQueryResolver),
}

#[derive(Debug)]
pub(crate) struct ResolverParameterType(pub String);
derive_display!(ResolverParameterType);

#[derive(Debug)]
pub(crate) struct QueryText(pub String);
derive_display!(QueryText);

#[derive(Debug)]
pub(crate) struct ResolverImportStatement(pub String);
derive_display!(ResolverImportStatement);

#[derive(Debug)]
pub(crate) struct ResolverReturnType(pub String);
derive_display!(ResolverReturnType);

#[derive(Debug)]
pub(crate) struct ResolverReadOutType(pub String);
derive_display!(ResolverReadOutType);

#[derive(Debug)]
pub(crate) struct ReaderAst(pub String);
derive_display!(ReaderAst);

#[derive(Debug)]
pub(crate) struct NormalizationAst(pub String);
derive_display!(NormalizationAst);

#[derive(Debug)]
pub(crate) struct ConvertFunction(pub String);
derive_display!(ConvertFunction);

#[derive(Debug)]
pub(crate) struct RefetchQueryArtifactImport(pub String);
derive_display!(RefetchQueryArtifactImport);

#[derive(Debug)]
pub(crate) struct FetchableResolver<'schema> {
    pub(crate) query_name: QueryOperationName,
    pub parent_type: &'schema SchemaObject<ValidatedEncounteredDefinedField>,
    pub query_text: QueryText,
    pub resolver_import_statement: ResolverImportStatement,
    pub resolver_parameter_type: ResolverParameterType,
    pub resolver_return_type: ResolverReturnType,
    pub resolver_read_out_type: ResolverReadOutType,
    pub reader_ast: ReaderAst,
    pub nested_resolver_artifact_imports: NestedResolverImports,
    pub convert_function: ConvertFunction,
    pub normalization_ast: NormalizationAst,
    pub refetch_query_artifact_import: RefetchQueryArtifactImport,
}

#[derive(Debug)]
pub(crate) struct NonFetchableResolver<'schema> {
    pub parent_type: &'schema SchemaObject<ValidatedEncounteredDefinedField>,
    pub(crate) resolver_field_name: SelectableFieldName,
    pub nested_resolver_artifact_imports: NestedResolverImports,
    pub resolver_read_out_type: ResolverReadOutType,
    pub reader_ast: ReaderAst,
    pub resolver_parameter_type: ResolverParameterType,
    pub resolver_return_type: ResolverReturnType,
    pub resolver_import_statement: ResolverImportStatement,
}

#[derive(Debug)]
pub(crate) struct RefetchQueryResolver {
    pub normalization_ast: NormalizationAst,
    pub query_text: QueryText,
    pub root_fetchable_field: SelectableFieldName,
    pub root_fetchable_field_parent_object: IsographObjectTypeName,
    // TODO wrap in a newtype
    pub refetch_query_index: usize,
}

fn generate_query_text(
    query_name: QueryOperationName,
    schema: &ValidatedSchema,
    merged_selection_set: &MergedSelectionSet,
    query_variables: &[WithSpan<ValidatedVariableDefinition>],
) -> QueryText {
    let mut query_text = String::new();

    let variable_text = write_variables_to_string(schema, query_variables.iter());

    query_text.push_str(&format!("query {} {} {{\\\n", query_name, variable_text));
    write_selections_for_query_text(&mut query_text, schema, &merged_selection_set, 1);
    query_text.push_str("}");
    QueryText(query_text)
}

fn generate_refetch_query_artifact_imports(
    root_refetched_paths: &[(PathToRefetchField, Vec<VariableName>)],
    root_resolver_field_name: SelectableFieldName,
) -> RefetchQueryArtifactImport {
    // TODO name the refetch queries with the path, or something, instead of
    // with indexes.
    let mut output = String::new();
    let mut array_syntax = String::new();
    for (query_index, (_, variable_names)) in root_refetched_paths.iter().enumerate() {
        output.push_str(&format!(
            "import refetchQuery{} from './{}/__refetch__{}.isograph';\n",
            query_index, root_resolver_field_name, query_index,
        ));
        let variable_names_str = variable_names_to_string(&variable_names);
        array_syntax.push_str(&format!(
            "{{ artifact: refetchQuery{}, allowedVariables: {} }}, ",
            query_index, variable_names_str
        ));
    }
    output.push_str(&format!("const nestedRefetchQueries = [{}];", array_syntax));
    RefetchQueryArtifactImport(output)
}

fn variable_names_to_string(variable_names: &[VariableName]) -> String {
    let mut s = "[".to_string();

    for variable in variable_names {
        s.push_str(&format!("\"{}\", ", variable));
    }

    s.push(']');

    s
}

fn write_variables_to_string<'a>(
    schema: &ValidatedSchema,
    mut variables: impl Iterator<Item = &'a WithSpan<ValidatedVariableDefinition>> + 'a,
) -> String {
    let mut empty = true;
    let mut first = true;
    let mut variable_text = String::new();
    variable_text.push('(');
    while let Some(variable) = variables.next() {
        empty = false;
        if !first {
            variable_text.push_str(", ");
        } else {
            first = false;
        }
        // TODO can we consume the variables here?
        let x: TypeAnnotation<UnvalidatedTypeName> =
            variable.item.type_.clone().map(|input_type_id| {
                // schema.
                let schema_input_type = schema.schema_data.lookup_input_type(input_type_id);
                schema_input_type.name().into()
            });
        variable_text.push_str(&format!("${}: {}", variable.item.name, x));
    }

    if empty {
        String::new()
    } else {
        variable_text.push(')');
        variable_text
    }
}

#[derive(Debug, Error)]
pub enum GenerateArtifactsError {
    #[error("Unable to write to artifact file at path {path:?}.\nMessage: {message:?}")]
    UnableToWriteToArtifactFile { path: PathBuf, message: io::Error },

    #[error("Unable to create directory at path {path:?}.\nMessage: {message:?}")]
    UnableToCreateDirectory { path: PathBuf, message: io::Error },

    #[error("Unable to delete directory at path {path:?}.\nMessage: {message:?}")]
    UnableToDeleteDirectory { path: PathBuf, message: io::Error },

    #[error("Unable to canonicalize path: {path:?}.\nMessage: {message:?}")]
    UnableToCanonicalizePath { path: PathBuf, message: io::Error },
}

fn write_selections_for_query_text(
    query_text: &mut String,
    schema: &ValidatedSchema,
    items: &[WithSpan<MergedServerFieldSelection>],
    indentation_level: u8,
) {
    for item in items.iter() {
        match &item.item {
            MergedServerFieldSelection::ScalarField(scalar_field) => {
                query_text.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
                if let Some(alias) = scalar_field.normalization_alias {
                    query_text.push_str(&format!("{}: ", alias));
                }
                let name = scalar_field.name.item;
                let arguments = get_serialized_arguments_for_query_text(&scalar_field.arguments);
                query_text.push_str(&format!("{}{},\\\n", name, arguments));
            }
            MergedServerFieldSelection::LinkedField(linked_field) => {
                query_text.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
                if let Some(alias) = linked_field.normalization_alias {
                    query_text.push_str(&format!("{}: ", alias));
                }
                let name = linked_field.name.item;
                let arguments = get_serialized_arguments_for_query_text(&linked_field.arguments);
                query_text.push_str(&format!("{}{} {{\\\n", name, arguments));
                write_selections_for_query_text(
                    query_text,
                    schema,
                    &linked_field.selection_set,
                    indentation_level + 1,
                );
                query_text.push_str(&format!(
                    "{}}},\\\n",
                    "  ".repeat(indentation_level as usize)
                ));
            }
        }
    }
}

fn generate_resolver_parameter_type(
    schema: &ValidatedSchema,
    selection_set: &Vec<WithSpan<ValidatedSelection>>,
    variant: Option<&WithSpan<ResolverVariant>>,
    parent_type: &SchemaObject<ValidatedEncounteredDefinedField>,
    nested_resolver_imports: &mut NestedResolverImports,
    indentation_level: u8,
) -> ResolverParameterType {
    // TODO use unwraps
    let mut resolver_parameter_type = "{\n".to_string();
    for selection in selection_set.iter() {
        write_query_types_from_selection(
            schema,
            &mut resolver_parameter_type,
            selection,
            // Variant "unwrapping" only matters for the top-level parameter type,
            // doing it for nested selections is leads to situations where linked fields
            // show up as linkedField: { data: /* actualLinkedFields */ }
            None,
            parent_type,
            nested_resolver_imports,
            indentation_level + 1,
        );
    }
    resolver_parameter_type.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    if let Some(&ResolverVariant::Component) = variant.as_ref().map(|v| &v.item) {
        resolver_parameter_type = format!(
            "{{ data:\n{}{},\n{}[index: string]: any }}",
            "  ".repeat(indentation_level as usize),
            resolver_parameter_type,
            "  ".repeat(indentation_level as usize)
        );
    }

    ResolverParameterType(resolver_parameter_type)
}

fn write_query_types_from_selection(
    schema: &ValidatedSchema,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    variant: Option<&WithSpan<ResolverVariant>>,
    parent_type: &SchemaObject<ValidatedEncounteredDefinedField>,
    nested_resolver_imports: &mut NestedResolverImports,
    indentation_level: u8,
) {
    query_type_declaration.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));

    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                match scalar_field.associated_data {
                    DefinedField::ServerField(_server_field) => {
                        let parent_field = parent_type
                            .encountered_fields
                            .get(&scalar_field.name.item.into())
                            .expect("parent_field should exist 1")
                            .as_server_field()
                            .expect("parent_field should exist and be server field");
                        let field = schema.field(*parent_field);
                        let name_or_alias = scalar_field.name_or_alias().item;

                        // TODO there should be a clever way to print without cloning
                        let output_type = field.associated_data.clone().map(|output_type_id| {
                            // TODO not just scalars, enums as well. Both should have a javascript name
                            let scalar_id = if let OutputTypeId::Scalar(scalar) = output_type_id {
                                scalar
                            } else {
                                panic!("output_type_id should be a scalar");
                            };
                            schema.schema_data.scalar(scalar_id).javascript_name
                        });
                        query_type_declaration.push_str(&format!(
                            "{}: {},\n",
                            name_or_alias,
                            print_type_annotation(&output_type)
                        ));
                    }
                    DefinedField::ResolverField(resolver_field_id) => {
                        let resolver = schema.resolver(resolver_field_id);

                        match nested_resolver_imports.entry(resolver.type_and_field) {
                            Entry::Occupied(mut occupied) => {
                                occupied.get_mut().types.push(ResolverImportType {
                                    original: ResolverImportName("ReadOutType".to_string()),
                                    alias: ResolverImportAlias(format!(
                                        "{}__outputType",
                                        resolver.type_and_field.underscore_separated()
                                    )),
                                });
                            }
                            Entry::Vacant(vacant) => {
                                vacant.insert(ResolverImport {
                                    default_import: false,
                                    types: vec![ResolverImportType {
                                        original: ResolverImportName("ReadOutType".to_string()),
                                        alias: ResolverImportAlias(format!(
                                            "{}__outputType",
                                            resolver.type_and_field.underscore_separated()
                                        )),
                                    }],
                                });
                            }
                        }

                        query_type_declaration.push_str(&format!(
                            "{}: {}__outputType,\n",
                            scalar_field.name_or_alias().item,
                            resolver.type_and_field.underscore_separated()
                        ));
                    }
                }
            }
            ServerFieldSelection::LinkedField(linked_field) => {
                let parent_field = parent_type
                    .encountered_fields
                    .get(&linked_field.name.item.into())
                    .expect("parent_field should exist 2")
                    .as_server_field()
                    .expect("Parent field should exist and be server field");
                let field = schema.field(*parent_field);
                let name_or_alias = linked_field.name_or_alias().item;
                let type_annotation = field.associated_data.clone().map(|output_type_id| {
                    // TODO Or interface or union type
                    let object_id = if let OutputTypeId::Object(object) = output_type_id {
                        object
                    } else {
                        panic!("output_type_id should be a object");
                    };
                    let object = schema.schema_data.object(object_id);
                    let inner = generate_resolver_parameter_type(
                        schema,
                        &linked_field.selection_set,
                        variant,
                        object.into(),
                        nested_resolver_imports,
                        indentation_level,
                    );
                    inner
                });
                query_type_declaration.push_str(&format!(
                    "{}: {},\n",
                    name_or_alias,
                    print_type_annotation(&type_annotation),
                ));
            }
        },
    }
}

fn print_type_annotation<T: Display>(type_annotation: &TypeAnnotation<T>) -> String {
    let mut s = String::new();
    print_type_annotation_impl(type_annotation, &mut s);
    s
}

fn print_type_annotation_impl<T: Display>(type_annotation: &TypeAnnotation<T>, s: &mut String) {
    match &type_annotation {
        TypeAnnotation::Named(named) => {
            s.push_str("(");
            s.push_str(&named.item.to_string());
            s.push_str(" | null)");
        }
        TypeAnnotation::List(list) => {
            print_list_type_annotation(list, s);
        }
        TypeAnnotation::NonNull(non_null) => {
            print_non_null_type_annotation(non_null, s);
        }
    }
}

fn print_list_type_annotation<T: Display>(list: &ListTypeAnnotation<T>, s: &mut String) {
    s.push_str("(");
    print_type_annotation_impl(&list.0, s);
    s.push_str(")[]");
}

fn print_non_null_type_annotation<T: Display>(non_null: &NonNullTypeAnnotation<T>, s: &mut String) {
    match non_null {
        NonNullTypeAnnotation::Named(named) => {
            s.push_str(&named.item.to_string());
        }
        NonNullTypeAnnotation::List(list) => {
            print_list_type_annotation(list, s);
        }
    }
}

fn generate_resolver_import_statement(
    resolver_action_kind: ResolverActionKind,
) -> ResolverImportStatement {
    match resolver_action_kind {
        ResolverActionKind::Identity => {
            ResolverImportStatement("const resolver = x => x;".to_string())
        }
        ResolverActionKind::NamedImport((name, path)) => ResolverImportStatement(format!(
            "import {{ {name} as resolver }} from '../../{path}';",
        )),
        ResolverActionKind::RefetchField => ResolverImportStatement(
            "import { makeNetworkRequest } from '@isograph/react';\n\
            const resolver = (artifact, variables) => () => \
            makeNetworkRequest(artifact, variables);"
                .to_string(),
        ),
        ResolverActionKind::MutationField => ResolverImportStatement(
            "import { makeNetworkRequest } from '@isograph/react';\n\
            const resolver = (artifact, variables) => (mutationParams) => \
            makeNetworkRequest(artifact, \
            {...variables, ...mutationParams});"
                .to_string(),
        ),
    }
}

#[derive(Debug)]
pub(crate) struct ResolverImportName(pub String);
derive_display!(ResolverImportName);

#[derive(Debug)]
pub(crate) struct ResolverImportAlias(pub String);
derive_display!(ResolverImportAlias);

#[derive(Debug)]
pub struct ResolverImportType {
    pub(crate) original: ResolverImportName,
    pub(crate) alias: ResolverImportAlias,
}
#[derive(Debug)]
pub struct ResolverImport {
    pub(crate) default_import: bool,
    pub(crate) types: Vec<ResolverImportType>,
}

fn generate_reader_ast<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    indentation_level: u8,
    nested_resolver_imports: &mut NestedResolverImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable resolver :(
    root_refetched_paths: &[(PathToRefetchField, Vec<VariableName>)],
) -> ReaderAst {
    generate_reader_ast_with_path(
        schema,
        selection_set,
        indentation_level,
        nested_resolver_imports,
        root_refetched_paths,
        // TODO we are not starting at the root when generating ASTs for non-fetchable resolvers
        // (and in theory some fetchable resolvers).
        &mut vec![],
    )
}

fn generate_reader_ast_with_path<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    indentation_level: u8,
    nested_resolver_imports: &mut NestedResolverImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable resolver :(
    root_refetched_paths: &[(PathToRefetchField, Vec<VariableName>)],
    path: &mut Vec<NameAndArguments>,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in selection_set {
        let s = generate_reader_ast_node(
            item,
            schema,
            indentation_level + 1,
            nested_resolver_imports,
            root_refetched_paths,
            path,
        );
        reader_ast.push_str(&s);
    }
    reader_ast.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    ReaderAst(reader_ast)
}

fn generate_reader_ast_node(
    selection: &WithSpan<Selection<ValidatedScalarDefinedField, ObjectId>>,
    schema: &ValidatedSchema,
    indentation_level: u8,
    nested_resolver_imports: &mut NestedResolverImports,
    // TODO use this to generate usedRefetchQueries
    root_refetched_paths: &[(PathToRefetchField, Vec<VariableName>)],
    path: &mut Vec<NameAndArguments>,
) -> String {
    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                let field_name = scalar_field.name.item;

                match scalar_field.associated_data {
                    DefinedField::ServerField(_) => {
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
                    DefinedField::ResolverField(resolver_field_id) => {
                        // This field is a resolver, so we need to look up the field in the
                        // schema.
                        let alias = scalar_field.name_or_alias().item;
                        let resolver_field = schema.resolver(resolver_field_id);
                        let arguments = get_serialized_field_arguments(
                            &scalar_field.arguments,
                            indentation_level + 1,
                        );
                        let indent_1 = "  ".repeat(indentation_level as usize);
                        let indent_2 = "  ".repeat((indentation_level + 1) as usize);
                        let resolver_field_string =
                            resolver_field.type_and_field.underscore_separated();
                        let variant = resolver_field
                            .variant
                            .as_ref()
                            .map(|x| format!("\"{}\"", x))
                            .unwrap_or_else(|| "null".to_string());

                        let resolver_refetched_paths =
                            refetched_paths_for_resolver(resolver_field, schema);

                        let nested_refetch_queries = get_nested_refetch_query_text(
                            &root_refetched_paths,
                            &resolver_refetched_paths,
                        );

                        match nested_resolver_imports.entry(resolver_field.type_and_field) {
                            Entry::Occupied(mut occupied) => {
                                occupied.get_mut().default_import = true;
                            }
                            Entry::Vacant(vacant) => {
                                vacant.insert(ResolverImport {
                                    default_import: true,
                                    types: vec![],
                                });
                            }
                        }

                        // This is indicative of poor data modeling.
                        match resolver_field.variant {
                            Some(WithSpan {
                                item: ResolverVariant::RefetchField,
                                ..
                            }) => {
                                let refetch_query_index =
                                    find_refetch_query_index(root_refetched_paths, path);
                                format!(
                                    "{indent_1}{{\n\
                                    {indent_2}kind: \"RefetchField\",\n\
                                    {indent_2}alias: \"{alias}\",\n\
                                    {indent_2}resolver: {resolver_field_string},\n\
                                    {indent_2}refetchQuery: {refetch_query_index},\n\
                                    {indent_1}}},\n",
                                )
                            }
                            Some(WithSpan {
                                item: ResolverVariant::MutationField(_),
                                ..
                            }) => {
                                let refetch_query_index =
                                    find_refetch_query_index(root_refetched_paths, path);
                                format!(
                                    "{indent_1}{{\n\
                                    {indent_2}kind: \"MutationField\",\n\
                                    {indent_2}alias: \"{alias}\",\n\
                                    {indent_2}resolver: {resolver_field_string},\n\
                                    {indent_2}refetchQuery: {refetch_query_index},\n\
                                    {indent_1}}},\n",
                                )
                            }
                            _ => {
                                format!(
                                    "{indent_1}{{\n\
                                    {indent_2}kind: \"Resolver\",\n\
                                    {indent_2}alias: \"{alias}\",\n\
                                    {indent_2}arguments: {arguments},\n\
                                    {indent_2}resolver: {resolver_field_string},\n\
                                    {indent_2}variant: {variant},\n\
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
                    nested_resolver_imports,
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

fn generate_normalization_ast<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &Vec<WithSpan<MergedServerFieldSelection>>,
    indentation_level: u8,
) -> NormalizationAst {
    let mut normalization_ast = "[\n".to_string();
    for item in selection_set.iter() {
        let s = generate_normalization_ast_node(item, schema, indentation_level + 1);
        normalization_ast.push_str(&s);
    }
    normalization_ast.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    NormalizationAst(normalization_ast)
}

fn generate_normalization_ast_node(
    item: &WithSpan<MergedServerFieldSelection>,
    schema: &ValidatedSchema,
    indentation_level: u8,
) -> String {
    match &item.item {
        MergedServerFieldSelection::ScalarField(scalar_field) => {
            let MergedScalarFieldSelection {
                name, arguments, ..
            } = scalar_field;
            let indent = "  ".repeat(indentation_level as usize);
            let indent_2 = "  ".repeat((indentation_level + 1) as usize);
            let serialized_arguments =
                get_serialized_field_arguments(arguments, indentation_level + 1);

            format!(
                "{indent}{{\n\
                {indent_2}kind: \"Scalar\",\n\
                {indent_2}fieldName: \"{name}\",\n\
                {indent_2}arguments: {serialized_arguments},\n\
                {indent}}},\n"
            )
        }
        MergedServerFieldSelection::LinkedField(linked_field) => {
            let MergedLinkedFieldSelection {
                name,
                selection_set,
                arguments,
                ..
            } = linked_field;
            let indent = "  ".repeat(indentation_level as usize);
            let indent_2 = "  ".repeat((indentation_level + 1) as usize);
            let serialized_arguments =
                get_serialized_field_arguments(arguments, indentation_level + 1);

            let selections =
                generate_normalization_ast(schema, selection_set, indentation_level + 1);

            format!(
                "{indent}{{\n\
                {indent_2}kind: \"Linked\",\n\
                {indent_2}fieldName: \"{name}\",\n\
                {indent_2}arguments: {serialized_arguments},\n\
                {indent_2}selections: {selections},\n\
                {indent}}},\n"
            )
        }
    }
}

fn get_serialized_arguments_for_query_text(
    arguments: &[WithSpan<SelectionFieldArgument>],
) -> String {
    if arguments.is_empty() {
        return "".to_string();
    } else {
        let mut arguments = arguments.iter();
        let first = arguments.next().unwrap();
        let mut s = format!(
            "({}: {}",
            first.item.name.item,
            serialize_non_constant_value_for_graphql(&first.item.value.item)
        );
        for argument in arguments {
            s.push_str(&format!(
                ", {}: {}",
                argument.item.name.item,
                serialize_non_constant_value_for_graphql(&argument.item.value.item)
            ));
        }
        s.push_str(")");
        s
    }
}

fn get_serialized_field_arguments(
    arguments: &[WithSpan<SelectionFieldArgument>],
    indentation_level: u8,
) -> String {
    if arguments.is_empty() {
        return "null".to_string();
    }

    let mut s = "[".to_string();
    let indent_1 = "  ".repeat((indentation_level + 1) as usize);
    let indent_2 = "  ".repeat((indentation_level + 2) as usize);

    for argument in arguments {
        let argument_name = argument.item.name.item;
        let non_constant_value_for_js =
            serialize_non_constant_value_for_js(&argument.item.value.item);
        s.push_str(&format!(
            "\n\
            {indent_1}{{\n\
            {indent_2}argumentName: \"{argument_name}\",\n\
            {indent_2}variableName: {non_constant_value_for_js},\n\
            {indent_1}}},\n",
        ));
    }

    s.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    s
}

fn serialize_non_constant_value_for_graphql(value: &NonConstantValue) -> String {
    match value {
        NonConstantValue::Variable(variable_name) => format!("${}", variable_name),
    }
}

// TODO strings and variables are indistinguishable
fn serialize_non_constant_value_for_js(value: &NonConstantValue) -> String {
    match value {
        NonConstantValue::Variable(variable_name) => format!("\"{}\"", variable_name),
    }
}

fn get_nested_refetch_query_text(
    root_refetched_paths: &[(PathToRefetchField, Vec<VariableName>)],
    nested_refetch_queries: &[PathToRefetchField],
) -> String {
    // Assuming count is 2... TODO fix
    let mut s = "[".to_string();
    for nested_refetch_query in nested_refetch_queries.iter() {
        let index = root_refetched_paths
            .iter()
            .enumerate()
            .find_map(|(index, root_path)| {
                if root_path.0 == *nested_refetch_query {
                    Some(index)
                } else {
                    None
                }
            })
            .expect("nested refetch query should be in root refetched paths");
        s.push_str(&format!("{}, ", index));
    }
    s.push_str("]");
    s
}

fn generate_read_out_type(resolver_definition: &ValidatedSchemaResolver) -> ResolverReadOutType {
    match &resolver_definition.variant {
        Some(variant) => match variant.item {
            ResolverVariant::Component => {
                // The read out type of a component is a function that accepts additional
                // (currently untyped) runtime props, and returns a component.
                ResolverReadOutType(
                    "(additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null)"
                        .to_string(),
                )
            }
            ResolverVariant::Eager => ResolverReadOutType("ResolverReturnType".to_string()),
            ResolverVariant::RefetchField => ResolverReadOutType("any".to_string()),
            ResolverVariant::MutationField(_) => ResolverReadOutType("any".to_string()),
        },
        None => ResolverReadOutType(
            // This is correct:
            // "FragmentReference<ReadFromStoreType, ResolverParameterType, ResolverReturnType>"
            //     .to_string(),
            // This is not correct, but has the correct behavior for now:
            "ResolverReturnType".to_string(),
        ),
    }
}

fn generate_resolver_return_type_declaration(
    action_kind: ResolverActionKind,
) -> ResolverReturnType {
    match action_kind {
        ResolverActionKind::Identity => ResolverReturnType("ResolverParameterType".to_string()),
        ResolverActionKind::NamedImport(_) | ResolverActionKind::RefetchField => {
            ResolverReturnType("ReturnType<typeof resolver>".to_string())
        }
        // TODO what should this be
        ResolverActionKind::MutationField => ResolverReturnType("any".to_string()),
    }
}

fn generate_convert_function(
    variant: &Option<WithSpan<ResolverVariant>>,
    field_name: SelectableFieldName,
) -> ConvertFunction {
    match variant {
        Some(variant) => {
            if let ResolverVariant::Component = variant.item {
                return ConvertFunction(format!(
                    "(() => {{\n\
                    {}const RefRendererForName = getRefRendererForName('{field_name}');\n\
                    {}return ((resolver, data) => additionalRuntimeProps => \n\
                    {}{{\n\
                    {}return <RefRendererForName \n\
                    {}resolver={{resolver}}\n\
                    {}data={{data}}\n\
                    {}additionalRuntimeProps={{additionalRuntimeProps}}\n\
                    {}/>;\n\
                    {}}})\n\
                    {}}})()",
                    "  ".repeat(2),
                    "  ".repeat(2),
                    "  ".repeat(3),
                    "  ".repeat(4),
                    "  ".repeat(5),
                    "  ".repeat(5),
                    "  ".repeat(5),
                    "  ".repeat(4),
                    "  ".repeat(3),
                    "  ".repeat(2),
                ));
            }
        }
        None => {}
    }
    ConvertFunction("((resolver, data) => resolver(data))".to_string())
}

fn find_refetch_query_index(
    paths: &[(PathToRefetchField, Vec<VariableName>)],
    path: &[NameAndArguments],
) -> usize {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, path_to_field)| {
            if &path_to_field.0.linked_fields == path {
                Some(index)
            } else {
                None
            }
        })
        .expect("Expected refetch query to be found")
}
