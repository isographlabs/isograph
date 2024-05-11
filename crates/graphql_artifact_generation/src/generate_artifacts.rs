use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::{self, Debug, Display},
    path::PathBuf,
    str::FromStr,
};

use common_lang_types::{
    ConstExportName, DescriptionValue, FilePath, IsographObjectTypeName, PathAndContent,
    QueryOperationName, SelectableFieldName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{GraphQLTypeAnnotation, ListTypeAnnotation, NonNullTypeAnnotation};
use intern::Lookup;
use isograph_lang_types::{
    ClientFieldId, NonConstantValue, SelectableServerFieldId, Selection, SelectionFieldArgument,
    ServerFieldSelection,
};
use isograph_schema::{
    create_merged_selection_set, ClientFieldVariant, FieldDefinitionLocation, FieldMapItem,
    ImperativelyLoadedFieldArtifactInfo, ObjectTypeAndFieldNames, RootRefetchedPath, SchemaObject,
    ValidatedClientField, ValidatedSchema, ValidatedSelection,
};

use crate::{
    artifact_file_contents::ENTRYPOINT,
    imperatively_loaded_fields::{
        get_artifact_for_imperatively_loaded_field, ImperativelyLoadedEntrypointArtifactInfo,
    },
    iso_overload_file::build_iso_overload,
    normalization_ast_text::generate_normalization_ast_text,
    query_text::generate_query_text,
    reader_ast::generate_reader_ast,
};

pub(crate) type NestedClientFieldImports = HashMap<ObjectTypeAndFieldNames, JavaScriptImports>;

macro_rules! derive_display {
    ($type:ident) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

pub(crate) fn client_defined_fields<'a>(
    schema: &'a ValidatedSchema,
) -> impl Iterator<Item = &'a ValidatedClientField> + 'a {
    schema.client_fields.iter().filter(|client_field| {
        matches!(
            client_field.variant,
            ClientFieldVariant::Component(_) | ClientFieldVariant::Eager(_)
        )
    })
}

pub fn get_artifact_path_and_content<'schema>(
    schema: &'schema ValidatedSchema,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> impl Iterator<Item = PathAndContent> + 'schema {
    let artifact_infos = get_artifact_infos(schema, project_root, artifact_directory);
    artifact_infos
        .into_iter()
        .map(ArtifactInfo::to_path_and_content)
        .flatten()
        .chain(std::iter::once(build_iso_overload(schema)))
}

/// Get all artifacts according to the following scheme:
/// - Add all the entrypoints to the queue
/// - While generating merged selection sets for entrypoints, if we encounter:
///   - a client field, add it the queue (but only once per client field.)
///   - a refetch field/magic mutation field, add it to the queue (along with
///     the path)
/// Keep processing artifacts until the queue is empty.
///
/// We *also* need to generate all (type) artifacts for all client-defined fields,
/// (i.e. including unreachable ones), because they are referenced in iso.ts.
/// So we separately add those to the encountered_client_field_ids set and generate full
/// artifacts. In the future, we should just generate types for these client fields, not
/// readers, etc.
///
/// TODO The artifact queue abstraction doesn't make much sense here.
fn get_artifact_infos<'schema>(
    schema: &'schema ValidatedSchema,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> Vec<ArtifactInfo<'schema>> {
    let mut artifact_queue = vec![];
    let mut encountered_client_field_ids = HashSet::new();
    let mut artifact_infos = vec![];

    for client_field_id in schema.entrypoints.iter() {
        artifact_infos.push(ArtifactInfo::Entrypoint(generate_entrypoint_artifact(
            schema,
            *client_field_id,
            &mut artifact_queue,
            &mut encountered_client_field_ids,
        )));

        // We also need to generate reader artifacts for the entrypoint client fields themselves
        encountered_client_field_ids.insert(*client_field_id);
    }

    for client_defined_field in client_defined_fields(schema) {
        if encountered_client_field_ids.insert(client_defined_field.id) {
            // What are we doing here?
            // We are generating, and throwing away, an entrypoint artifact. This has the effect of
            // encountering selected __refetch fields. Refetch fields reachable from orphaned
            // client fields still need type artifacts generated.
            // We currently also generate unneeded reader artifacts.
            //
            // Anyway, this sucks and should be improved.
            let _ = generate_entrypoint_artifact(
                schema,
                client_defined_field.id,
                &mut vec![],
                &mut encountered_client_field_ids,
            );
        }
    }

    for encountered_client_field_id in encountered_client_field_ids {
        let encountered_client_field = schema.client_field(encountered_client_field_id);
        let artifact_info = match &encountered_client_field.variant {
            ClientFieldVariant::Eager(component_name_and_path) => {
                ArtifactInfo::EagerReader(generate_eager_reader_artifact(
                    schema,
                    encountered_client_field,
                    project_root,
                    artifact_directory,
                    *component_name_and_path,
                ))
            }
            ClientFieldVariant::Component(component_name_and_path) => {
                ArtifactInfo::ComponentReader(generate_component_reader_artifact(
                    schema,
                    encountered_client_field,
                    project_root,
                    artifact_directory,
                    *component_name_and_path,
                ))
            }
            ClientFieldVariant::ImperativelyLoadedField(variant) => {
                let function_import_statement = match &variant.primary_field_info {
                    Some(info) => generate_function_import_statement_for_mutation_reader(
                        &info.primary_field_field_map,
                    ),
                    None => generate_function_import_statement_for_refetch_reader(),
                };
                ArtifactInfo::RefetchReader(generate_mutation_reader_artifact(
                    schema,
                    encountered_client_field,
                    function_import_statement,
                ))
            }
        };
        artifact_infos.push(artifact_info);
    }

    for imperatively_loaded_field_artifact_info in artifact_queue {
        artifact_infos.push(ArtifactInfo::ImperativelyLoadedEntrypoint(
            get_artifact_for_imperatively_loaded_field(
                schema,
                imperatively_loaded_field_artifact_info,
            ),
        ))
    }

    artifact_infos
}

fn generate_entrypoint_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    client_field_id: ClientFieldId,
    artifact_queue: &mut Vec<ImperativelyLoadedFieldArtifactInfo>,
    encountered_client_field_ids: &mut HashSet<ClientFieldId>,
) -> EntrypointArtifactInfo<'schema> {
    let fetchable_client_field = schema.client_field(client_field_id);
    if let Some((ref selection_set, _)) = fetchable_client_field.selection_set_and_unwraps {
        let query_name = fetchable_client_field.name.into();

        let (merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            schema
                .server_field_data
                .object(fetchable_client_field.parent_object_id),
            selection_set,
            Some(artifact_queue),
            Some(encountered_client_field_ids),
            &fetchable_client_field,
        );

        // TODO when we do not call generate_entrypoint_artifact extraneously,
        // we can panic instead of using a default entrypoint type
        // TODO model this better so that the RootOperationName is somehow a
        // parameter
        let root_operation_name = schema
            .fetchable_types
            .get(&fetchable_client_field.parent_object_id)
            .unwrap_or_else(|| {
                schema
                    .fetchable_types
                    .iter()
                    .next()
                    .expect("Expected at least one fetchable type to exist")
                    .1
            });

        let parent_object = schema
            .server_field_data
            .object(fetchable_client_field.parent_object_id);
        let query_text = generate_query_text(
            query_name,
            schema,
            &merged_selection_set,
            &fetchable_client_field.variable_definitions,
            root_operation_name,
        );
        let refetch_query_artifact_imports =
            generate_refetch_query_artifact_imports(&root_refetched_paths);

        let normalization_ast_text =
            generate_normalization_ast_text(schema, &merged_selection_set, 0);

        EntrypointArtifactInfo {
            query_text,
            query_name,
            parent_type: parent_object.into(),
            normalization_ast_text,
            refetch_query_artifact_import: refetch_query_artifact_imports,
        }
    } else {
        // TODO convert to error
        todo!("Unsupported: client fields on query with no selection set")
    }
}

fn generate_eager_reader_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    client_field: &ValidatedClientField,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
    component_name_and_path: (ConstExportName, FilePath),
) -> EagerReaderArtifactInfo<'schema> {
    if let Some((selection_set, _)) = &client_field.selection_set_and_unwraps {
        let parent_type = schema
            .server_field_data
            .object(client_field.parent_object_id);
        let mut nested_client_field_artifact_imports = HashMap::new();

        let (_merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            // TODO here we are assuming that the client field is only on the Query type.
            // That restriction should be loosened.
            schema
                .server_field_data
                .object(client_field.parent_object_id)
                .into(),
            selection_set,
            None,
            None,
            client_field,
        );

        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            0,
            &mut nested_client_field_artifact_imports,
            &root_refetched_paths,
        );

        let client_field_parameter_type = generate_client_field_parameter_type(
            schema,
            &selection_set,
            parent_type.into(),
            &mut nested_client_field_artifact_imports,
            0,
        );
        let client_field_output_type = generate_output_type(client_field);
        let function_import_statement = generate_function_import_statement_for_eager_or_component(
            project_root,
            artifact_directory,
            component_name_and_path,
        );
        EagerReaderArtifactInfo {
            parent_type: parent_type.into(),
            client_field_name: client_field.name,
            reader_ast,
            nested_client_field_artifact_imports,
            function_import_statement,
            client_field_output_type,
            client_field_parameter_type,
        }
    } else {
        panic!("Unsupported: client fields not on query with no selection set")
    }
}

fn generate_component_reader_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    client_field: &ValidatedClientField,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
    component_name_and_path: (ConstExportName, FilePath),
) -> ComponentReaderArtifactInfo<'schema> {
    if let Some((selection_set, _)) = &client_field.selection_set_and_unwraps {
        let parent_type = schema
            .server_field_data
            .object(client_field.parent_object_id);
        let mut nested_client_field_artifact_imports = HashMap::new();

        let (_merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            schema
                .server_field_data
                .object(client_field.parent_object_id)
                .into(),
            selection_set,
            None,
            None,
            client_field,
        );

        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            0,
            &mut nested_client_field_artifact_imports,
            &root_refetched_paths,
        );

        let client_field_parameter_type = generate_client_field_parameter_type(
            schema,
            &selection_set,
            parent_type.into(),
            &mut nested_client_field_artifact_imports,
            0,
        );
        let client_field_output_type = generate_output_type(client_field);
        let function_import_statement = generate_function_import_statement_for_eager_or_component(
            project_root,
            artifact_directory,
            component_name_and_path,
        );
        ComponentReaderArtifactInfo {
            parent_type: parent_type.into(),
            client_field_name: client_field.name,
            reader_ast,
            nested_client_field_artifact_imports,
            function_import_statement,
            client_field_output_type,
            client_field_parameter_type,
        }
    } else {
        panic!("Unsupported: client fields not on query with no selection set")
    }
}

fn generate_mutation_reader_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    client_field: &ValidatedClientField,
    function_import_statement: ClientFieldFunctionImportStatement,
) -> RefetchReaderArtifactInfo<'schema> {
    if let Some((selection_set, _)) = &client_field.selection_set_and_unwraps {
        let parent_type = schema
            .server_field_data
            .object(client_field.parent_object_id);
        let mut nested_client_field_artifact_imports = HashMap::new();

        let (_merged_selection_set, root_refetched_paths) = create_merged_selection_set(
            schema,
            schema
                .server_field_data
                .object(client_field.parent_object_id)
                .into(),
            selection_set,
            None,
            None,
            client_field,
        );

        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            0,
            &mut nested_client_field_artifact_imports,
            &root_refetched_paths,
        );

        let client_field_parameter_type = generate_client_field_parameter_type(
            schema,
            &selection_set,
            parent_type.into(),
            &mut nested_client_field_artifact_imports,
            0,
        );
        let client_field_output_type = generate_output_type(client_field);
        RefetchReaderArtifactInfo {
            parent_type: parent_type.into(),
            client_field_name: client_field.name,
            reader_ast,
            nested_client_field_artifact_imports,
            function_import_statement,
            client_field_output_type,
            client_field_parameter_type,
        }
    } else {
        panic!("Unsupported: client fields not on query with no selection set")
    }
}

/// A data structure that contains enough information to infallibly
/// generate the contents of the generated file (e.g. of the entrypoint
/// artifact), as well as the path to the generated file.
#[derive(Debug)]
pub(crate) enum ArtifactInfo<'schema> {
    Entrypoint(EntrypointArtifactInfo<'schema>),

    // These four artifact types all generate reader.ts files, but they
    // are different. Namely, they have different types of resolvers and
    // different types of exported artifacts.
    EagerReader(EagerReaderArtifactInfo<'schema>),
    ComponentReader(ComponentReaderArtifactInfo<'schema>),
    RefetchReader(RefetchReaderArtifactInfo<'schema>),

    ImperativelyLoadedEntrypoint(ImperativelyLoadedEntrypointArtifactInfo),
}

impl<'schema> ArtifactInfo<'schema> {
    pub fn to_path_and_content(self) -> Vec<PathAndContent> {
        match self {
            ArtifactInfo::Entrypoint(entrypoint_artifact) => {
                vec![entrypoint_artifact.path_and_content()]
            }
            ArtifactInfo::ImperativelyLoadedEntrypoint(refetch_query) => {
                vec![refetch_query.path_and_content()]
            }
            ArtifactInfo::EagerReader(eager_reader_artifact) => {
                eager_reader_artifact.path_and_content()
            }
            ArtifactInfo::ComponentReader(component_reader_artifact) => {
                component_reader_artifact.path_and_content()
            }
            ArtifactInfo::RefetchReader(refetch_reader_artifact) => {
                refetch_reader_artifact.path_and_content()
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct ClientFieldParameterType(pub String);
derive_display!(ClientFieldParameterType);

#[derive(Debug)]
pub(crate) struct QueryText(pub String);
derive_display!(QueryText);

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
pub(crate) struct ConvertFunction(pub String);
derive_display!(ConvertFunction);

#[derive(Debug)]
pub(crate) struct RefetchQueryArtifactImport(pub String);
derive_display!(RefetchQueryArtifactImport);

#[derive(Debug)]
pub(crate) struct EntrypointArtifactInfo<'schema> {
    pub(crate) query_name: QueryOperationName,
    pub parent_type: &'schema SchemaObject,
    pub query_text: QueryText,
    pub normalization_ast_text: NormalizationAstText,
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
pub(crate) struct EagerReaderArtifactInfo<'schema> {
    pub parent_type: &'schema SchemaObject,
    pub(crate) client_field_name: SelectableFieldName,
    pub nested_client_field_artifact_imports: NestedClientFieldImports,
    pub client_field_output_type: ClientFieldOutputType,
    pub reader_ast: ReaderAst,
    pub client_field_parameter_type: ClientFieldParameterType,
    pub function_import_statement: ClientFieldFunctionImportStatement,
}

impl<'schema> EagerReaderArtifactInfo<'schema> {
    pub fn path_and_content(self) -> Vec<PathAndContent> {
        let EagerReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }
}

#[derive(Debug)]
pub(crate) struct ComponentReaderArtifactInfo<'schema> {
    pub parent_type: &'schema SchemaObject,
    pub(crate) client_field_name: SelectableFieldName,
    pub nested_client_field_artifact_imports: NestedClientFieldImports,
    pub client_field_output_type: ClientFieldOutputType,
    pub reader_ast: ReaderAst,
    pub client_field_parameter_type: ClientFieldParameterType,
    pub function_import_statement: ClientFieldFunctionImportStatement,
}

impl<'schema> ComponentReaderArtifactInfo<'schema> {
    pub fn path_and_content(self) -> Vec<PathAndContent> {
        let ComponentReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }
}

#[derive(Debug)]
pub(crate) struct RefetchReaderArtifactInfo<'schema> {
    pub parent_type: &'schema SchemaObject,
    pub(crate) client_field_name: SelectableFieldName,
    pub nested_client_field_artifact_imports: NestedClientFieldImports,
    pub client_field_output_type: ClientFieldOutputType,
    pub reader_ast: ReaderAst,
    pub client_field_parameter_type: ClientFieldParameterType,
    pub function_import_statement: ClientFieldFunctionImportStatement,
}

impl<'schema> RefetchReaderArtifactInfo<'schema> {
    pub fn path_and_content(self) -> Vec<PathAndContent> {
        let RefetchReaderArtifactInfo {
            parent_type,
            client_field_name,
            ..
        } = &self;

        let relative_directory = generate_path(parent_type.name, *client_field_name);

        self.file_contents(&relative_directory)
    }
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
        "const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [{}];",
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

fn generate_client_field_parameter_type(
    schema: &ValidatedSchema,
    selection_set: &[WithSpan<ValidatedSelection>],
    parent_type: &SchemaObject,
    nested_client_field_imports: &mut NestedClientFieldImports,
    indentation_level: u8,
) -> ClientFieldParameterType {
    // TODO use unwraps
    let mut client_field_parameter_type = "{\n".to_string();
    for selection in selection_set.iter() {
        write_query_types_from_selection(
            schema,
            &mut client_field_parameter_type,
            selection,
            parent_type,
            nested_client_field_imports,
            indentation_level + 1,
        );
    }
    client_field_parameter_type.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientFieldParameterType(client_field_parameter_type)
}

fn write_query_types_from_selection(
    schema: &ValidatedSchema,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    parent_type: &SchemaObject,
    nested_client_field_imports: &mut NestedClientFieldImports,
    indentation_level: u8,
) {
    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                match scalar_field.associated_data.location {
                    FieldDefinitionLocation::Server(_server_field) => {
                        query_type_declaration
                            .push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
                        let parent_field = parent_type
                            .encountered_fields
                            .get(&scalar_field.name.item.into())
                            .expect("parent_field should exist 1")
                            .as_server_field()
                            .expect("parent_field should exist and be server field");
                        let field = schema.server_field(*parent_field);

                        write_optional_description(
                            field.description,
                            query_type_declaration,
                            indentation_level,
                        );

                        let name_or_alias = scalar_field.name_or_alias().item;

                        // TODO there should be a clever way to print without cloning
                        let output_type = field.associated_data.clone().map(|output_type_id| {
                            // TODO not just scalars, enums as well. Both should have a javascript name
                            let scalar_id =
                                if let SelectableServerFieldId::Scalar(scalar) = output_type_id {
                                    scalar
                                } else {
                                    panic!("output_type_id should be a scalar");
                                };
                            schema.server_field_data.scalar(scalar_id).javascript_name
                        });
                        query_type_declaration.push_str(&format!(
                            "{}: {},\n",
                            name_or_alias,
                            print_type_annotation(&output_type)
                        ));
                    }
                    FieldDefinitionLocation::Client(client_field_id) => {
                        let client_field = schema.client_field(client_field_id);
                        write_optional_description(
                            client_field.description,
                            query_type_declaration,
                            indentation_level,
                        );
                        query_type_declaration
                            .push_str(&format!("{}", "  ".repeat(indentation_level as usize)));

                        match nested_client_field_imports.entry(client_field.type_and_field) {
                            Entry::Occupied(mut occupied) => {
                                occupied.get_mut().types.push(TypeImportName(format!(
                                    "{}__outputType",
                                    client_field.type_and_field.underscore_separated()
                                )));
                            }
                            Entry::Vacant(vacant) => {
                                vacant.insert(JavaScriptImports {
                                    default_import: false,
                                    types: vec![TypeImportName(format!(
                                        "{}__outputType",
                                        client_field.type_and_field.underscore_separated()
                                    ))],
                                });
                            }
                        }

                        query_type_declaration.push_str(&format!(
                            "{}: {}__outputType,\n",
                            scalar_field.name_or_alias().item,
                            client_field.type_and_field.underscore_separated()
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
                let field = schema.server_field(*parent_field);
                write_optional_description(
                    field.description,
                    query_type_declaration,
                    indentation_level,
                );
                query_type_declaration
                    .push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
                let name_or_alias = linked_field.name_or_alias().item;
                let type_annotation = field.associated_data.clone().map(|output_type_id| {
                    // TODO Or interface or union type
                    let object_id = if let SelectableServerFieldId::Object(object) = output_type_id
                    {
                        object
                    } else {
                        panic!("output_type_id should be a object");
                    };
                    let object = schema.server_field_data.object(object_id);
                    let inner = generate_client_field_parameter_type(
                        schema,
                        &linked_field.selection_set,
                        object.into(),
                        nested_client_field_imports,
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

fn write_optional_description(
    description: Option<DescriptionValue>,
    query_type_declaration: &mut String,
    indentation_level: u8,
) {
    if let Some(description) = description {
        query_type_declaration.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
        query_type_declaration.push_str("/**\n");
        query_type_declaration.push_str(description.lookup());
        query_type_declaration.push_str("\n");
        query_type_declaration.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
        query_type_declaration.push_str("*/\n");
    }
}

fn print_type_annotation<T: Display>(type_annotation: &GraphQLTypeAnnotation<T>) -> String {
    let mut s = String::new();
    print_type_annotation_impl(type_annotation, &mut s);
    s
}

fn print_type_annotation_impl<T: Display>(
    type_annotation: &GraphQLTypeAnnotation<T>,
    s: &mut String,
) {
    match &type_annotation {
        GraphQLTypeAnnotation::Named(named) => {
            s.push_str("(");
            s.push_str(&named.item.to_string());
            s.push_str(" | null)");
        }
        GraphQLTypeAnnotation::List(list) => {
            print_list_type_annotation(list, s);
        }
        GraphQLTypeAnnotation::NonNull(non_null) => {
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

fn generate_function_import_statement_for_refetch_reader() -> ClientFieldFunctionImportStatement {
    let content = format!(
        "import {{ makeNetworkRequest, type IsographEnvironment }} \
        from '@isograph/react';\n\
        const resolver = (\n\
        {}environment: IsographEnvironment,\n\
        {}artifact: RefetchQueryNormalizationArtifact,\n\
        {}variables: any\n\
        ) => () => \
        makeNetworkRequest(environment, artifact, variables);",
        "  ", "  ", "  "
    );
    ClientFieldFunctionImportStatement(content)
}

fn generate_function_import_statement_for_mutation_reader(
    field_map: &[FieldMapItem],
) -> ClientFieldFunctionImportStatement {
    let include_read_out_data = get_read_out_data(&field_map);
    ClientFieldFunctionImportStatement(format!(
        "{include_read_out_data}\n\
        import {{ makeNetworkRequest, type IsographEnvironment }} from '@isograph/react';\n\
        const resolver = (\n\
        {}environment: IsographEnvironment,\n\
        {}artifact: RefetchQueryNormalizationArtifact,\n\
        {}readOutData: any,\n\
        {}filteredVariables: any\n\
        ) => (mutationParams: any) => {{\n\
        {}const variables = includeReadOutData({{...filteredVariables, \
        ...mutationParams}}, readOutData);\n\
        {}makeNetworkRequest(environment, artifact, variables);\n\
        }};\n\
        ",
        "  ", "  ", "  ", "  ", "  ", "  "
    ))
}

fn generate_function_import_statement_for_eager_or_component(
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
    (file_name, path): (ConstExportName, FilePath),
) -> ClientFieldFunctionImportStatement {
    let path_to_client_field = project_root.join(
        PathBuf::from_str(path.lookup())
            .expect("paths should be legal here. This is indicative of a bug in Isograph."),
    );
    let relative_path =
        // artifact directory includes __isograph, so artifact_directory.join("Type/Field")
        // is a directory "two levels deep" within the artifact_directory.
        //
        // So diff_paths(path_to_client_field, artifact_directory.join("Type/Field"))
        // is a lazy way of saying "make a relative path from two levels deep in the artifact
        // dir to the client field".
        //
        // Since we will always go ../../../ the Type/Field part will never show up
        // in the output.
        //
        // Anyway, TODO do better.
        pathdiff::diff_paths(path_to_client_field, artifact_directory.join("Type/Field"))
            .expect("Relative path should work");
    ClientFieldFunctionImportStatement(format!(
        "import {{ {file_name} as resolver }} from '{}';",
        relative_path.to_str().expect(
            "This path should be stringifiable. This probably is indicative of a bug in Relay."
        )
    ))
}

fn get_read_out_data(field_map: &[FieldMapItem]) -> String {
    let spaces = "  ";
    let mut s = "const includeReadOutData = (variables: any, readOutData: any) => {\n".to_string();

    for item in field_map.iter() {
        // This is super hacky and due to the fact that argument names and field names are
        // treated differently, because that's how it is in the GraphQL spec.
        let split_to_arg = item.split_to_arg();
        let mut path_segments = Vec::with_capacity(1 + split_to_arg.to_field_names.len());
        path_segments.push(split_to_arg.to_argument_name);
        path_segments.extend(split_to_arg.to_field_names.into_iter());

        let last_index = path_segments.len() - 1;
        let mut path_so_far = "".to_string();
        for (index, path_segment) in path_segments.into_iter().enumerate() {
            let is_last = last_index == index;
            let path_segment_item = path_segment;

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
pub(crate) struct TypeImportName(pub String);
derive_display!(TypeImportName);

#[derive(Debug)]
pub struct JavaScriptImports {
    pub(crate) default_import: bool,
    pub(crate) types: Vec<TypeImportName>,
}

pub(crate) fn get_serialized_field_arguments(
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
                    {indent_2}{{ kind: \"Literal\", value: {int_value} }},\n\
                    {indent_1}],\n"
                )
            }
        };

        s.push_str(&arg_value);
    }

    s.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    s
}

fn generate_output_type(client_field: &ValidatedClientField) -> ClientFieldOutputType {
    match &client_field.variant {
        variant => match variant {
            ClientFieldVariant::Component(_) => {
                ClientFieldOutputType("(React.FC<ExtractSecondParam<typeof resolver>>)".to_string())
            }
            ClientFieldVariant::Eager(_) => {
                ClientFieldOutputType("ReturnType<typeof resolver>".to_string())
            }
            ClientFieldVariant::ImperativelyLoadedField(params) => {
                match params.primary_field_info {
                    Some(_) => ClientFieldOutputType("(params: any) => void".to_string()),
                    None => ClientFieldOutputType("() => void".to_string()),
                }
            }
        },
    }
}

pub fn generate_path(
    object_name: IsographObjectTypeName,
    field_name: SelectableFieldName,
) -> PathBuf {
    PathBuf::from(object_name.lookup()).join(field_name.lookup())
}
