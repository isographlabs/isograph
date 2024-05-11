use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Debug},
    path::PathBuf,
    str::FromStr,
};

use common_lang_types::{
    ConstExportName, FilePath, IsographObjectTypeName, PathAndContent, SelectableFieldName,
    WithLocation,
};
use intern::Lookup;
use isograph_lang_types::{NonConstantValue, SelectionFieldArgument};
use isograph_schema::{
    create_merged_selection_set, ClientFieldVariant, FieldMapItem, ObjectTypeAndFieldNames,
    SchemaObject, ValidatedClientField, ValidatedSchema,
};

use crate::{
    eager_reader_artifact_info::{
        generate_client_field_parameter_type, generate_eager_reader_artifact,
        EagerReaderArtifactInfo,
    },
    entrypoint_artifact_info::{generate_entrypoint_artifact, EntrypointArtifactInfo},
    imperatively_loaded_fields::{
        get_artifact_for_imperatively_loaded_field, ImperativelyLoadedEntrypointArtifactInfo,
    },
    iso_overload_file::build_iso_overload,
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

pub(crate) fn generate_function_import_statement_for_eager_or_component(
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

pub(crate) fn generate_output_type(client_field: &ValidatedClientField) -> ClientFieldOutputType {
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
