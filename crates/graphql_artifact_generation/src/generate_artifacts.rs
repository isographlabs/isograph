use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Display},
    path::PathBuf,
};

use common_lang_types::{
    ArtifactFileType, ArtifactPathAndContent, DescriptionValue, IsographObjectTypeName,
    SelectableFieldName, WithSpan,
};
use graphql_lang_types::{GraphQLTypeAnnotation, ListTypeAnnotation, NonNullTypeAnnotation};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    NonConstantValue, SelectableServerFieldId, Selection, SelectionFieldArgument,
    ServerFieldSelection,
};
use isograph_schema::{
    ClientFieldVariant, FieldDefinitionLocation, SchemaObject, UserWrittenComponentVariant,
    ValidatedClientField, ValidatedSchema, ValidatedSelection,
};
use lazy_static::lazy_static;

use crate::{
    eager_reader_artifact::generate_eager_reader_artifact,
    entrypoint_artifact::generate_entrypoint_artifacts, import_statements::ParamTypeImports,
    iso_overload_file::build_iso_overload,
    refetch_reader_artifact::generate_refetch_reader_artifact,
};

lazy_static! {
    pub static ref RESOLVER_READER: ArtifactFileType = "resolver_reader".intern().into();
    pub static ref REFETCH_READER: ArtifactFileType = "refetch_reader".intern().into();
    pub static ref RESOLVER_PARAM_TYPE: ArtifactFileType = "param_type".intern().into();
    pub static ref RESOLVER_OUTPUT_TYPE: ArtifactFileType = "output_type".intern().into();
    pub static ref ENTRYPOINT: ArtifactFileType = "entrypoint".intern().into();
    pub static ref ISO_TS: ArtifactFileType = "iso".intern().into();
}

/// Get all artifacts according to the following scheme:
///
/// For each entrypoint, generate an entrypoint artifact.
/// - While creating the artifact's merged selection set:
///   - note imperative client fields (e.g. __refetch, exposeAs and
///     @loadable fields.) and their path, and queue up imperative
///     field artifacts
///
/// For each hand-written client field, generate
/// - parameter in type/field/parameter_type.ts
/// - output type artifacts
///   - note that we only really need to do this for client fields
///     reachable from other client fields and those that serve
///     as entrypoints
/// - reader artifacts in type/field/reader.ts
///   - note that we only need readers if an entrypoint shows up as part
///     of an entrypoint, but it doesn't seem to hurt to have readers for
///     all hand-written fields, since one may want to debug a reader.
///
/// For each imperative field artifact, generate:
/// - reader (i.e. to select id field), in type/field/imperative_reader.ts
/// - entrypoint + output type in root_type/field/imperative_field_N.ts (if slim)
/// - entrypoint + output type in type/field/entrypoint.ts (if not slim)
/// - if readable, reader in type/field/reader.ts
pub fn get_artifact_path_and_content<'schema>(
    schema: &'schema ValidatedSchema,
    project_root: &PathBuf,
    artifact_directory: &PathBuf,
) -> Vec<ArtifactPathAndContent> {
    let mut global_client_field_map = BTreeMap::new();
    let mut path_and_contents = vec![];

    // For each entrypoint, generate an entrypoint artifact and refetch artifacts
    for entrypoint_id in schema.entrypoints.iter() {
        let entrypoint_path_and_content =
            generate_entrypoint_artifacts(schema, *entrypoint_id, &mut global_client_field_map);
        path_and_contents.extend(entrypoint_path_and_content);
    }

    for (encountered_client_field_id, (scalar_client_field_traversal_state, _selection_map)) in
        &global_client_field_map
    {
        let encountered_client_field = schema.client_field(*encountered_client_field_id);

        // Generate reader ASTs for all encountered client fields, which may be reader or refetch reader
        path_and_contents.extend(match &encountered_client_field.variant {
            ClientFieldVariant::UserWritten(info) => generate_eager_reader_artifact(
                schema,
                encountered_client_field,
                project_root,
                artifact_directory,
                *info,
                scalar_client_field_traversal_state,
            ),
            ClientFieldVariant::ImperativelyLoadedField(variant) => {
                generate_refetch_reader_artifact(
                    schema,
                    encountered_client_field,
                    variant,
                    scalar_client_field_traversal_state,
                )
            }
        });
    }

    path_and_contents.push(build_iso_overload(schema));

    path_and_contents
}

pub(crate) fn get_serialized_field_arguments(
    // TODO make this an iterator
    arguments: &[SelectionFieldArgument],
    indentation_level: u8,
) -> String {
    if arguments.is_empty() {
        return "null".to_string();
    }

    let mut s = "[".to_string();
    let indent_1 = "  ".repeat((indentation_level + 1) as usize);
    let indent_2 = "  ".repeat((indentation_level + 2) as usize);

    for argument in arguments {
        let argument_name = argument.name.item;
        let arg_value = match argument.value.item {
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
                    {indent_2}\"{{ kind: \"Literal\", value: {bool_string} }},\n\
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
            ClientFieldVariant::UserWritten(info) => match info.user_written_component_variant {
                UserWrittenComponentVariant::Eager => {
                    ClientFieldOutputType("ReturnType<typeof resolver>".to_string())
                }
                UserWrittenComponentVariant::Component => ClientFieldOutputType(
                    "(React.FC<ExtractSecondParam<typeof resolver>>)".to_string(),
                ),
            },
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

pub(crate) fn generate_client_field_parameter_type(
    schema: &ValidatedSchema,
    selection_set: &[WithSpan<ValidatedSelection>],
    parent_type: &SchemaObject,
    nested_client_field_imports: &mut ParamTypeImports,
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
    nested_client_field_imports: &mut ParamTypeImports,
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

                        nested_client_field_imports.insert(client_field.type_and_field);
                        query_type_declaration.push_str(&format!(
                            "{}: {}__output_type,\n",
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
                    let object_id = output_type_id.try_into().expect(
                        "output_type_id should be an object. \
                        This is indicative of a bug in Isograph.",
                    );
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

macro_rules! derive_display {
    ($type:ident) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
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
