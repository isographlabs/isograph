use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Debug},
    path::PathBuf,
    str::FromStr,
};

use common_lang_types::{
    ArtifactFileType, ConstExportName, FilePath, IsographObjectTypeName, PathAndContent,
    SelectableFieldName, WithLocation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{NonConstantValue, SelectionFieldArgument};
use isograph_schema::{
    ClientFieldVariant, ObjectTypeAndFieldNames, ValidatedClientField, ValidatedSchema,
};
use lazy_static::lazy_static;

use crate::{
    component_reader_artifact_info::{
        generate_component_reader_artifact, ComponentReaderArtifactInfo,
    },
    eager_reader_artifact_info::{generate_eager_reader_artifact, EagerReaderArtifactInfo},
    entrypoint_artifact_info::{generate_entrypoint_artifact, EntrypointArtifactInfo},
    imperatively_loaded_fields::{
        get_artifact_for_imperatively_loaded_field, ImperativelyLoadedEntrypointArtifactInfo,
    },
    iso_overload_file::build_iso_overload,
    refetch_reader_artifact_info::{
        generate_refetch_reader_artifact_info, RefetchReaderArtifactInfo,
    },
};

lazy_static! {
    pub static ref READER: ArtifactFileType = "reader".intern().into();
    pub static ref READER_PARAM_TYPE: ArtifactFileType = "param_type".intern().into();
    pub static ref READER_OUTPUT_TYPE: ArtifactFileType = "output_type".intern().into();
    pub static ref ENTRYPOINT: ArtifactFileType = "entrypoint".intern().into();
    pub static ref ISO_TS: ArtifactFileType = "iso".intern().into();
}

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
            ClientFieldVariant::ImperativelyLoadedField(variant) => ArtifactInfo::RefetchReader(
                generate_refetch_reader_artifact_info(schema, encountered_client_field, variant),
            ),
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

/// A data structure that contains enough information to infallibly
/// generate the contents of the generated file (e.g. of the entrypoint
/// artifact), as well as the path to the generated file.
#[derive(Debug)]
pub(crate) enum ArtifactInfo<'schema> {
    Entrypoint(EntrypointArtifactInfo<'schema>),

    // These artifact types all generate reader.ts files, but they
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

pub(crate) fn nested_client_field_names_to_import_statement(
    nested_client_field_imports: HashMap<ObjectTypeAndFieldNames, JavaScriptImports>,
    current_file_type_name: IsographObjectTypeName,
) -> (String, String) {
    let mut client_field_import_statement = String::new();
    let mut client_field_type_import_statement = String::new();

    // TODO we should always sort outputs. We should find a nice generic way to ensure that.
    let mut nested_client_field_imports: Vec<_> = nested_client_field_imports.into_iter().collect();
    nested_client_field_imports.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (nested_client_field_name, javascript_import) in nested_client_field_imports {
        write_client_field_import(
            javascript_import,
            nested_client_field_name,
            &mut client_field_import_statement,
            &mut client_field_type_import_statement,
            current_file_type_name,
        );
    }
    (
        client_field_import_statement,
        client_field_type_import_statement,
    )
}

fn write_client_field_import(
    javascript_import: JavaScriptImports,
    nested_client_field_name: ObjectTypeAndFieldNames,
    client_field_import_statement: &mut String,
    client_field_type_import_statement: &mut String,
    current_file_type_name: IsographObjectTypeName,
) {
    if !javascript_import.default_import && javascript_import.types.is_empty() {
        panic!(
            "Client field imports should not be created in an empty state. \
            This is indicative of a bug in Isograph."
        );
    }

    let mut s_client_field_import = "".to_string();
    let mut s_client_field_type_import = "".to_string();

    if javascript_import.default_import {
        s_client_field_import.push_str(&format!(
            "import {} from '{}';\n",
            nested_client_field_name.underscore_separated(),
            nested_client_field_name.relative_path(current_file_type_name, *READER)
        ));
    }

    let mut types = javascript_import.types.iter();
    if let Some(first) = types.next() {
        s_client_field_type_import.push_str(&format!("import {{{}", first));
        for value in types {
            s_client_field_type_import.push_str(&format!(", {}", value));
        }
        s_client_field_type_import.push_str(&format!(
            "}} from '{}';\n",
            nested_client_field_name.relative_path(current_file_type_name, *READER_OUTPUT_TYPE)
        ));
    }

    client_field_import_statement.push_str(&s_client_field_import);
    client_field_type_import_statement.push_str(&s_client_field_type_import);
}

pub(crate) fn get_output_type_text(
    function_import_statement: &ClientFieldFunctionImportStatement,
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableFieldName,
    output_type: ClientFieldOutputType,
) -> String {
    let function_import_statement = &function_import_statement.0;
    format!(
        "{function_import_statement}\n\
        export type {}__{}__outputType = {};",
        parent_type_name, field_name, output_type
    )
}
