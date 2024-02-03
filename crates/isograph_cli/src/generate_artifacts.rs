use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug, Display},
    io,
    path::PathBuf,
    str::FromStr,
};

use common_lang_types::{
    HasName, IsographObjectTypeName, Location, QueryOperationName, SelectableFieldName, Span,
    UnvalidatedTypeName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLInputValueDefinition, ListTypeAnnotation, NamedTypeAnnotation, NonNullTypeAnnotation,
    TypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    DefinedTypeId, NonConstantValue, ResolverFieldId, Selection, SelectionFieldArgument,
    ServerFieldSelection, VariableDefinition,
};
use isograph_schema::{
    create_merged_selection_set, into_name_and_arguments, refetched_paths_for_resolver,
    ArtifactQueueItem, DefinedField, FieldMapItem, MergedLinkedFieldSelection,
    MergedScalarFieldSelection, MergedSelectionSet, MergedServerFieldSelection,
    MutationFieldResolverInfo, NameAndArguments, PathToRefetchField, RefetchFieldResolverInfo,
    RequiresRefinement, ResolverActionKind, ResolverTypeAndField, ResolverVariant,
    RootRefetchedPath, ValidatedSchema, ValidatedSchemaObject, ValidatedSchemaResolver,
    ValidatedSelection, ValidatedVariableDefinition, ENTRYPOINT, READER,
};
use thiserror::Error;

use crate::write_artifacts::write_to_disk;

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

pub(crate) struct PathAndContent {
    pub(crate) relative_directory: PathBuf,
    // It doesn't make sense that this is a SelectableFieldName
    pub(crate) file_name_prefix: SelectableFieldName,
    pub(crate) file_content: String,
}

// TODO move to another module
pub(crate) fn generate_and_write_artifacts(
    schema: &ValidatedSchema,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> Result<usize, GenerateArtifactsError> {
    let paths_and_contents =
        get_artifact_path_and_contents(schema, project_root, artifact_directory);
    let artifact_count = write_to_disk(paths_and_contents, artifact_directory)?;

    Ok(artifact_count)
}

fn build_iso_overload_for_entrypoint<'schema>(
    resolver_field_id: ResolverFieldId,
    schema: &'schema ValidatedSchema,
) -> (String, String) {
    let resolver = schema.resolver(resolver_field_id);
    let mut s: String = "".to_string();
    let import = format!(
        "import entrypoint_{} from '../__isograph/{}/{}/entrypoint.ts'\n",
        resolver.type_and_field.underscore_separated(),
        resolver.type_and_field.type_name,
        resolver.type_and_field.field_name,
    );
    let formatted_field = format!(
        "entrypoint {}.{}",
        resolver.type_and_field.type_name, resolver.type_and_field.field_name
    );
    s.push_str(&format!(
        "export function iso<T>(
            param: T & MatchesWhitespaceAndString<'{}', T>
        ): IdentityWithParam<typeof entrypoint_{}>;\n",
        formatted_field,
        resolver.type_and_field.underscore_separated(),
    ));
    (import, s)
}

fn build_iso_overload_for_schema_resolver(resolver: &ValidatedSchemaResolver) -> (String, String) {
    let mut s: String = "".to_string();
    let import = format!(
        "import {{ ResolverParameterType as field_{} }} from './{}/{}/reader.ts'\n",
        resolver.type_and_field.underscore_separated(),
        resolver.type_and_field.type_name,
        resolver.type_and_field.field_name,
    );
    let formatted_field = format!(
        "field {}.{}",
        resolver.type_and_field.type_name, resolver.type_and_field.field_name
    );
    s.push_str(&format!(
        "export function iso<T>(
            param: T & MatchesWhitespaceAndString<'{}', T>
        ): IdentityWithParam<field_{}>;\n",
        formatted_field,
        resolver.type_and_field.underscore_separated(),
    ));
    (import, s)
}

fn build_iso_overload<'schema>(schema: &'schema ValidatedSchema) -> PathAndContent {
    let mut imports = "".to_string();
    let mut content = String::from(
        "type IdentityWithParam<TParam> = <TResolverReturn>(
    x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;

type WhitespaceCharacter = ' ' | '\\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
? Whitespace<In>
: In;

type MatchesWhitespaceAndString<
TString extends string,
T
> = Whitespace<T> extends `${TString}${string}` ? T : never;",
    );
    let entrypoints_overload = schema
        .entrypoints
        .iter()
        .map(|resolver_field_id| build_iso_overload_for_entrypoint(*resolver_field_id, schema));
    let fields_overload = schema
        .resolvers
        .iter()
        .filter(|resolver| matches!(resolver.action_kind, ResolverActionKind::NamedImport(_)))
        .map(|schema_resolver| build_iso_overload_for_schema_resolver(schema_resolver));
    for (import, field_overload) in fields_overload {
        imports.push_str(&import);
        content.push_str(&field_overload);
    }
    for (import, entrypoint_overload) in entrypoints_overload {
        imports.push_str(&import);
        content.push_str(&entrypoint_overload);
    }
    content.push_str(
        "
export function iso(_queryText: string): IdentityWithParam<any> {
  return function identity<TResolverReturn>(
    x: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return x;
  };
}",
    );
    imports.push_str(&content);
    PathAndContent {
        file_content: imports,
        relative_directory: PathBuf::new(),
        file_name_prefix: "iso".intern().into(),
    }
}

fn get_artifact_path_and_contents<'schema>(
    schema: &'schema ValidatedSchema,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> impl Iterator<Item = PathAndContent> + 'schema {
    let artifact_infos = get_artifact_infos(schema, project_root, artifact_directory);
    artifact_infos
        .into_iter()
        .map(ArtifactInfo::to_path_and_content)
        .chain(std::iter::once(build_iso_overload(schema)))
}

/// get all artifacts that we must generate according to the following rough plan:
/// - initially, we know we must generate artifacts for each resolver
/// - we must also generate an artifact for each refetch field/magic mutation we
///   encounter while generating an artifact for an entrypoint
///
/// We do this by keeping a queue of artifacts to generate, and adding to the queue
/// as we process entrypoints.
fn get_artifact_infos<'schema>(
    schema: &'schema ValidatedSchema,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> Vec<ArtifactInfo<'schema>> {
    let mut artifact_queue: Vec<_> =
        schema
            .resolvers
            .iter()
            .map(ArtifactQueueItem::Reader)
            .chain(schema.entrypoints.iter().map(|resolver_field_id| {
                ArtifactQueueItem::Entrypoint {
                    top_level_resolver: schema.resolver(*resolver_field_id),
                }
            }))
            .collect();

    let mut artifacts = vec![];
    while let Some(queue_item) = artifact_queue.pop() {
        artifacts.push(generate_artifact(
            queue_item,
            schema,
            &mut artifact_queue,
            project_root,
            artifact_directory,
        ));
    }

    artifacts
}

fn generate_artifact<'schema>(
    queue_item: ArtifactQueueItem<'schema>,
    schema: &'schema ValidatedSchema,
    // As we process reader artifacts, we can also encounter refetch and mutation
    // fields. If so, we add them to the artifact queue.
    artifact_queue: &mut Vec<ArtifactQueueItem<'schema>>,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> ArtifactInfo<'schema> {
    match queue_item {
        ArtifactQueueItem::Reader(resolver) => ArtifactInfo::Reader(generate_reader_artifact(
            schema,
            resolver,
            project_root,
            artifact_directory,
        )),
        ArtifactQueueItem::Entrypoint { top_level_resolver } => ArtifactInfo::Entrypoint(
            generate_entrypoint_artifact(schema, top_level_resolver, artifact_queue),
        ),
        ArtifactQueueItem::RefetchField(refetch_info) => {
            get_artifact_for_refetch_field(schema, refetch_info)
        }
        ArtifactQueueItem::MutationField(mutation_info) => {
            get_artifact_for_mutation_field(schema, mutation_info)
        }
    }
}

// N.B. this was originally copied from generate_entrypoint_artifact,
// and it could use some de-duplication
fn get_artifact_for_refetch_field<'schema>(
    schema: &'schema ValidatedSchema,
    refetch_info: RefetchFieldResolverInfo,
) -> ArtifactInfo<'schema> {
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

    ArtifactInfo::RefetchQuery(RefetchArtifactInfo {
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
) -> ArtifactInfo<'schema> {
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
        requires_refinement,
        ..
    } = refetch_info;

    let arguments = get_serialized_field_arguments(
        &mutation_field_arguments
            .iter()
            .map(|input_value_definition| {
                input_value_definition
                    .clone()
                    .map(|input_value_definition| SelectionFieldArgument {
                        name: input_value_definition
                            .name
                            .map(|x| x.into())
                            .hack_to_with_span(),
                        value: input_value_definition
                            .name
                            .map(|x| NonConstantValue::Variable(x.into()))
                            .hack_to_with_span(),
                    })
            })
            .collect::<Vec<_>>(),
        1,
    );

    let parent_object = schema.schema_data.object(parent_id);

    // HACK
    // TODO pass the non-magical name
    let mutation_field_name = magic_mutation_field_name.lookup()[2..].to_string();
    // END HACK

    let query_text = generate_mutation_query_text(
        parent_object,
        schema,
        &merged_selection_set,
        variable_definitions,
        magic_mutation_field_name,
        &mutation_field_name,
        mutation_primary_field_name,
        mutation_field_arguments,
        requires_refinement,
    );

    let selections = generate_normalization_ast(schema, &merged_selection_set, 2);
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

    ArtifactInfo::RefetchQuery(RefetchArtifactInfo {
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
            name: WithLocation::new("id".intern().into(), Location::generated()),
            type_: TypeAnnotation::NonNull(Box::new(NonNullTypeAnnotation::Named(
                NamedTypeAnnotation(WithSpan {
                    item: DefinedTypeId::Scalar(schema.id_type_id),
                    span: Span::todo_generated(),
                }),
            ))),
        },
        span: Span::todo_generated(),
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
    magic_mutation_field_name: SelectableFieldName,
    mutation_field_name: &str,
    mutation_primary_field_name: SelectableFieldName,
    mutation_field_arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    requires_refinement: RequiresRefinement,
) -> QueryText {
    let mut query_text = String::new();

    let mutation_parameters: Vec<_> = mutation_field_arguments
        .iter()
        .map(|argument| {
            let variable_name = argument.item.name.map(|value_name| value_name.into());
            variable_definitions.push(WithSpan {
                item: VariableDefinition {
                    name: variable_name,
                    type_: argument.item.type_.clone().map(|type_name| {
                        *schema
                            .schema_data
                            .defined_types
                            .get(&type_name.into())
                            .expect("Expected type to be found, this indicates a bug in Isograph")
                    }),
                },
                span: Span::todo_generated(),
            });
            WithLocation::new(
                SelectionFieldArgument {
                    name: argument.item.name.map(|x| x.into()).hack_to_with_span(),
                    value: variable_name
                        .map(|variable_name| NonConstantValue::Variable(variable_name))
                        .hack_to_with_span(),
                },
                Location::generated(),
            )
        })
        .collect();

    let variable_text = write_variables_to_string(schema, &mut variable_definitions.iter());
    let mutation_field_arguments = get_serialized_arguments_for_query_text(&mutation_parameters);

    let aliased_mutation_field_name =
        get_aliased_mutation_field_name(&mutation_field_name, &mutation_parameters);

    let parent_object_name = parent_object_type.name;
    query_text.push_str(&format!(
        "mutation {parent_object_name}{magic_mutation_field_name} {variable_text} {{\\\n\
        {aliased_mutation_field_name}: {mutation_field_name}{mutation_field_arguments} {{\\\n\
        {mutation_primary_field_name} {{ \\\n",
    ));

    if let RequiresRefinement::Yes(refine_to) = requires_refinement {
        query_text.push_str(&format!("... on {} {{\\\n", refine_to));
        write_selections_for_query_text(&mut query_text, schema, &merged_selection_set, 1);
        query_text.push_str("}\\\n");
    } else {
        write_selections_for_query_text(&mut query_text, schema, &merged_selection_set, 1);
    }

    query_text.push_str("}}}");
    QueryText(query_text)
}

fn get_aliased_mutation_field_name(
    name: &str,
    parameters: &[WithLocation<SelectionFieldArgument>],
) -> String {
    let mut s = name.to_string();

    for param in parameters.iter() {
        // TODO NonConstantValue will format to a string like "$name", but we want just "name".
        // There is probably a better way to do this.
        s.push_str("____");
        s.push_str(&param.item.to_alias_str_chunk());
    }
    s
}

fn generate_entrypoint_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    top_level_resolver: &ValidatedSchemaResolver,
    artifact_queue: &mut Vec<ArtifactQueueItem<'schema>>,
) -> EntrypointArtifactInfo<'schema> {
    if let Some((ref selection_set, _)) = top_level_resolver.selection_set_and_unwraps {
        let query_name = top_level_resolver.name.into();

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
            &top_level_resolver,
        );

        let query_object = schema
            .query_object()
            .expect("Expected query object to exist");
        let query_text = generate_query_text(
            query_name,
            schema,
            &merged_selection_set,
            &top_level_resolver.variable_definitions,
        );
        let refetch_query_artifact_imports =
            generate_refetch_query_artifact_imports(&root_refetched_paths);

        let normalization_ast = generate_normalization_ast(schema, &merged_selection_set, 0);

        EntrypointArtifactInfo {
            query_text,
            query_name,
            parent_type: query_object.into(),
            normalization_ast,
            refetch_query_artifact_import: refetch_query_artifact_imports,
        }
    } else {
        // TODO convert to error
        todo!("Unsupported: resolvers on query with no selection set")
    }
}

fn generate_reader_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    resolver: &ValidatedSchemaResolver,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> ReaderArtifactInfo<'schema> {
    if let Some((selection_set, _)) = &resolver.selection_set_and_unwraps {
        let parent_type = schema.schema_data.object(resolver.parent_object_id);
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
            // TODO this is obviously a smell
            &mut vec![],
            &resolver,
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
            &resolver.variant,
            parent_type.into(),
            &mut nested_resolver_artifact_imports,
            0,
        );
        let resolver_return_type = generate_resolver_return_type_declaration(&resolver.action_kind);
        let resolver_read_out_type = generate_read_out_type(resolver);
        let resolver_import_statement = generate_resolver_import_statement(
            &resolver.action_kind,
            project_root,
            artifact_directory,
        );
        ReaderArtifactInfo {
            parent_type: parent_type.into(),
            resolver_field_name: resolver.name,
            reader_ast,
            nested_resolver_artifact_imports,
            resolver_import_statement,
            resolver_read_out_type,
            resolver_parameter_type,
            resolver_return_type,
            resolver_variant: resolver.variant.clone(),
        }
    } else {
        panic!("Unsupported: resolvers not on query with no selection set")
    }
}

/// A data structure that contains enough information to infallibly
/// generate the contents of the generated file (e.g. of the entrypoint
/// artifact), as well as the path to the generated file.
#[derive(Debug)]
pub(crate) enum ArtifactInfo<'schema> {
    Entrypoint(EntrypointArtifactInfo<'schema>),
    Reader(ReaderArtifactInfo<'schema>),
    RefetchQuery(RefetchArtifactInfo),
}

impl<'schema> ArtifactInfo<'schema> {
    pub fn to_path_and_content(self) -> PathAndContent {
        match self {
            ArtifactInfo::Entrypoint(entrypoint_artifact) => entrypoint_artifact.path_and_content(),
            ArtifactInfo::Reader(reader_artifact) => reader_artifact.path_and_content(),
            ArtifactInfo::RefetchQuery(refetch_query) => refetch_query.path_and_content(),
        }
    }
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
pub(crate) struct EntrypointArtifactInfo<'schema> {
    pub(crate) query_name: QueryOperationName,
    pub parent_type: &'schema ValidatedSchemaObject,
    pub query_text: QueryText,
    pub normalization_ast: NormalizationAst,
    pub refetch_query_artifact_import: RefetchQueryArtifactImport,
}

impl<'schema> EntrypointArtifactInfo<'schema> {
    pub fn path_and_content(self) -> PathAndContent {
        let EntrypointArtifactInfo {
            query_name,
            parent_type,
            ..
        } = &self;

        let directory = generate_path(parent_type.name, (*query_name).into());

        PathAndContent {
            relative_directory: directory,
            file_content: self.file_contents(),
            file_name_prefix: *ENTRYPOINT,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReaderArtifactInfo<'schema> {
    pub parent_type: &'schema ValidatedSchemaObject,
    pub(crate) resolver_field_name: SelectableFieldName,
    pub nested_resolver_artifact_imports: NestedResolverImports,
    pub resolver_read_out_type: ResolverReadOutType,
    pub reader_ast: ReaderAst,
    pub resolver_parameter_type: ResolverParameterType,
    pub resolver_return_type: ResolverReturnType,
    pub resolver_import_statement: ResolverImportStatement,
    pub resolver_variant: ResolverVariant,
}

impl<'schema> ReaderArtifactInfo<'schema> {
    pub fn path_and_content(self) -> PathAndContent {
        let ReaderArtifactInfo {
            parent_type,
            resolver_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *resolver_field_name);

        PathAndContent {
            file_content: self.file_contents(),
            relative_directory,
            file_name_prefix: *READER,
        }
    }
}

#[derive(Debug)]
pub(crate) struct RefetchArtifactInfo {
    pub normalization_ast: NormalizationAst,
    pub query_text: QueryText,
    pub root_fetchable_field: SelectableFieldName,
    pub root_fetchable_field_parent_object: IsographObjectTypeName,
    // TODO wrap in a newtype
    pub refetch_query_index: usize,
}

impl RefetchArtifactInfo {
    pub fn path_and_content(self) -> PathAndContent {
        let RefetchArtifactInfo {
            root_fetchable_field,
            root_fetchable_field_parent_object,
            refetch_query_index,
            ..
        } = &self;

        let relative_directory =
            generate_path(*root_fetchable_field_parent_object, *root_fetchable_field);
        let file_name_prefix = format!("__refetch__{}", refetch_query_index)
            .intern()
            .into();

        PathAndContent {
            file_content: self.file_contents(),
            relative_directory,
            file_name_prefix,
        }
    }
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
    root_refetched_paths: &[RootRefetchedPath],
) -> RefetchQueryArtifactImport {
    // TODO name the refetch queries with the path, or something, instead of
    // with indexes.
    let mut output = String::new();
    let mut array_syntax = String::new();
    for (query_index, RootRefetchedPath { variables, .. }) in
        root_refetched_paths.iter().enumerate()
    {
        output.push_str(&format!(
            "import refetchQuery{} from './__refetch__{}';\n",
            query_index, query_index,
        ));
        let variable_names_str = variable_names_to_string(&variables);
        array_syntax.push_str(&format!(
            "{{ artifact: refetchQuery{}, allowedVariables: {} }}, ",
            query_index, variable_names_str
        ));
    }
    output.push_str(&format!(
        "const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [{}];",
        array_syntax
    ));
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
                let schema_input_type = schema.schema_data.lookup_unvalidated_type(input_type_id);
                schema_input_type.name().into()
            });
        // TODO this is dangerous, since variable.item.name is a WithLocation, which impl's Display.
        // We should find a way to make WithLocation not impl display, without making error's hard
        // to work with.
        variable_text.push_str(&format!("${}: {}", variable.item.name.item, x));
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
    #[error("Unable to write to artifact file at path {path:?}.\nReason: {message:?}")]
    UnableToWriteToArtifactFile { path: PathBuf, message: io::Error },

    #[error("Unable to create directory at path {path:?}.\nReason: {message:?}")]
    UnableToCreateDirectory { path: PathBuf, message: io::Error },

    #[error("Unable to delete directory at path {path:?}.\nReason: {message:?}")]
    UnableToDeleteDirectory { path: PathBuf, message: io::Error },
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
                    // This is bad, alias is WithLocation
                    query_text.push_str(&format!("{}: ", alias.item));
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
    selection_set: &[WithSpan<ValidatedSelection>],
    variant: &ResolverVariant,
    parent_type: &ValidatedSchemaObject,
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
            // TODO this works, but should be cleaned up
            &ResolverVariant::Eager,
            parent_type,
            nested_resolver_imports,
            indentation_level + 1,
        );
    }
    resolver_parameter_type.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    if variant == &ResolverVariant::Component {
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
    variant: &ResolverVariant,
    parent_type: &ValidatedSchemaObject,
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
                            let scalar_id = if let DefinedTypeId::Scalar(scalar) = output_type_id {
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
                    let object_id = if let DefinedTypeId::Object(object) = output_type_id {
                        object
                    } else {
                        panic!("output_type_id should be a object");
                    };
                    let object = schema.schema_data.object(object_id);
                    let inner = generate_resolver_parameter_type(
                        schema,
                        &linked_field.selection_set,
                        &variant,
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
    resolver_action_kind: &ResolverActionKind,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> ResolverImportStatement {
    match resolver_action_kind {
        ResolverActionKind::NamedImport((name, path)) => {
            let path_to_artifact = project_root
                .join(PathBuf::from_str(path.lookup()).expect(
                    "paths should be legal here. This is indicative of a bug in Isograph.",
                ));
            let relative_path =
                pathdiff::diff_paths(path_to_artifact, artifact_directory.join("a/b/c"))
                    .expect("Relative path should work");
            ResolverImportStatement(format!(
                "import {{ {name} as resolver }} from '{}';",
                relative_path.to_str().expect("This path should be stringifiable. This probably is indicative of a bug in Relay.")
            ))
        }
        ResolverActionKind::RefetchField => ResolverImportStatement(
            "import { makeNetworkRequest } from '@isograph/react';\n\
            const resolver = (environment, artifact, variables) => () => \
            makeNetworkRequest(environment, artifact, variables);"
                .to_string(),
        ),
        ResolverActionKind::MutationField(ref m) => {
            let spaces = "  ";
            let include_read_out_data = get_read_out_data(&m.field_map);
            ResolverImportStatement(format!(
                "{include_read_out_data}\n\
                import {{ makeNetworkRequest }} from '@isograph/react';\n\
                const resolver = (environment, artifact, readOutData, filteredVariables) => (mutationParams) => {{\n\
                {spaces}const variables = includeReadOutData({{...filteredVariables, \
                ...mutationParams}}, readOutData);\n\
                {spaces}makeNetworkRequest(environment, artifact, variables);\n\
            }};\n\
            "
            ))
        }
    }
}

fn get_read_out_data(field_map: &[FieldMapItem]) -> String {
    let spaces = "  ";
    let mut s = "const includeReadOutData = (variables, readOutData) => {\n".to_string();

    for item in field_map.iter() {
        // This is super hacky and due to the fact that argument names and field names are
        // treated differently, because that's how it is in the GraphQL spec.
        let mut path_segments = Vec::with_capacity(1 + item.to_field_names.len());
        path_segments.push(&item.to_argument_name);
        path_segments.extend(item.to_field_names.iter());

        let last_index = path_segments.len() - 1;
        let mut path_so_far = "".to_string();
        for (index, path_segment) in path_segments.into_iter().enumerate() {
            let is_last = last_index == index;
            let path_segment_item = path_segment.item;

            if is_last {
                let from_value = item.from;
                s.push_str(&format!(
                    "{spaces}variables.{path_so_far}{path_segment_item} = \
                    readOutData.{from_value};\n"
                ));
            } else {
                s.push_str(&format!(
                    "{spaces}variables.{path_so_far}{path_segment_item} = \
                    variables.{path_so_far}{path_segment_item} ?? {{}};\n"
                ));
                path_so_far.push_str(&format!("{path_segment_item}."));
            }
        }
    }

    s.push_str(&format!("{spaces}return variables;\n}};\n"));
    s
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
    // N.B. this is not root_refetched_paths when we're generating an entrypoint :(
    root_refetched_paths: &[RootRefetchedPath],
) -> ReaderAst {
    generate_reader_ast_with_path(
        schema,
        selection_set,
        indentation_level,
        nested_resolver_imports,
        root_refetched_paths,
        // TODO we are not starting at the root when generating ASTs for reader artifacts
        // (and in theory some entrypoints).
        &mut vec![],
    )
}

fn generate_reader_ast_with_path<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    indentation_level: u8,
    nested_resolver_imports: &mut NestedResolverImports,
    // N.B. this is not root_refetched_paths when we're generating a non-fetchable resolver :(
    root_refetched_paths: &[RootRefetchedPath],
    path: &mut Vec<NameAndArguments>,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in selection_set {
        let s = generate_reader_ast_node(
            item,
            schema,
            indentation_level + 1,
            nested_resolver_imports,
            &root_refetched_paths,
            path,
        );
        reader_ast.push_str(&s);
    }
    reader_ast.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    ReaderAst(reader_ast)
}

fn generate_reader_ast_node(
    selection: &WithSpan<ValidatedSelection>,
    schema: &ValidatedSchema,
    indentation_level: u8,
    nested_resolver_imports: &mut NestedResolverImports,
    // TODO use this to generate usedRefetchQueries
    root_refetched_paths: &[RootRefetchedPath],
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

                        let resolver_refetched_paths =
                            refetched_paths_for_resolver(resolver_field, schema, path);

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
                            ResolverVariant::RefetchField => {
                                let refetch_query_index =
                                    find_refetch_query_index(root_refetched_paths, path);
                                format!(
                                    "{indent_1}{{\n\
                                    {indent_2}kind: \"RefetchField\",\n\
                                    {indent_2}alias: \"{alias}\",\n\
                                    {indent_2}readerArtifact: {resolver_field_string},\n\
                                    {indent_2}refetchQuery: {refetch_query_index},\n\
                                    {indent_1}}},\n",
                                )
                            }
                            ResolverVariant::MutationField(ref s) => {
                                let refetch_query_index = find_mutation_query_index(
                                    root_refetched_paths,
                                    path,
                                    s.mutation_field_name,
                                );
                                format!(
                                    "{indent_1}{{\n\
                                    {indent_2}kind: \"MutationField\",\n\
                                    {indent_2}alias: \"{alias}\",\n\
                                    {indent_2}readerArtifact: {resolver_field_string},\n\
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
                                    {indent_2}readerArtifact: {resolver_field_string},\n\
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
    selection_set: &[WithSpan<MergedServerFieldSelection>],
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
            // TODO this is bad, name is a WithLocation and impl's Display, we should fix
            let name = name.item;

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

            // TODO this is bad, name is a WithLocation which impl's Display
            let name = name.item;

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
    arguments: &[WithLocation<SelectionFieldArgument>],
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
    arguments: &[WithLocation<SelectionFieldArgument>],
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
        let arg_value = match argument.item.value.item {
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
                    {indent_2}{{ kind: \"Literal\", value: \"{int_value}\" }},\n\
                    {indent_1}],\n"
                )
            }
        };

        s.push_str(&arg_value);
    }

    s.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    s
}

fn serialize_non_constant_value_for_graphql(value: &NonConstantValue) -> String {
    match value {
        NonConstantValue::Variable(variable_name) => format!("${}", variable_name),
        NonConstantValue::Integer(int_value) => int_value.to_string(),
    }
}

fn get_nested_refetch_query_text(
    root_refetched_paths: &[RootRefetchedPath],
    nested_refetch_queries: &[PathToRefetchField],
) -> String {
    let mut s = "[".to_string();
    for nested_refetch_query in nested_refetch_queries.iter() {
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

fn generate_read_out_type(resolver_definition: &ValidatedSchemaResolver) -> ResolverReadOutType {
    match &resolver_definition.variant {
        variant => match variant {
            ResolverVariant::Component => ResolverReadOutType("(React.FC<any>)".to_string()),
            ResolverVariant::Eager => ResolverReadOutType("ResolverReturnType".to_string()),
            ResolverVariant::RefetchField => ResolverReadOutType("any".to_string()),
            ResolverVariant::MutationField(_) => ResolverReadOutType("any".to_string()),
        },
    }
}

fn generate_resolver_return_type_declaration(
    action_kind: &ResolverActionKind,
) -> ResolverReturnType {
    match action_kind {
        ResolverActionKind::NamedImport(_) | ResolverActionKind::RefetchField => {
            ResolverReturnType("ReturnType<typeof resolver>".to_string())
        }
        // TODO what should this be
        ResolverActionKind::MutationField(_) => ResolverReturnType("any".to_string()),
    }
}

fn find_refetch_query_index(paths: &[RootRefetchedPath], path: &[NameAndArguments]) -> usize {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, path_to_field)| {
            if &path_to_field.path.linked_fields == path
                && path_to_field.field_name == "__refetch".intern().into()
            {
                Some(index)
            } else {
                None
            }
        })
        .expect("Expected refetch query to be found")
}

fn find_mutation_query_index(
    paths: &[RootRefetchedPath],
    path: &[NameAndArguments],
    mutation_name: SelectableFieldName,
) -> usize {
    paths
        .iter()
        .enumerate()
        .find_map(|(index, path_to_field)| {
            if &path_to_field.path.linked_fields == path
                && path_to_field.field_name == mutation_name
            {
                Some(index)
            } else {
                None
            }
        })
        .expect("Expected refetch query to be found")
}

fn generate_path(object_name: IsographObjectTypeName, field_name: SelectableFieldName) -> PathBuf {
    PathBuf::from(object_name.lookup()).join(field_name.lookup())
}
