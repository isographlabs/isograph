use common_lang_types::{
    ArtifactFileType, ArtifactPathAndContent, DescriptionValue, IsographObjectTypeName, Location,
    SelectableFieldName, Span, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldId, NonConstantValue, SelectableServerFieldId, Selection,
    ServerFieldSelection, TypeAnnotation, UnionVariant, VariableDefinition,
};
use isograph_schema::{
    get_provided_arguments, selection_map_wrapped, ClientFieldTraversalResult, ClientFieldVariant,
    FieldDefinitionLocation, NameAndArguments, NormalizationKey, RequiresRefinement, SchemaObject,
    UserWrittenComponentVariant, ValidatedClientField, ValidatedIsographSelectionVariant,
    ValidatedSchema, ValidatedSelection, ValidatedVariableDefinition,
};
use lazy_static::lazy_static;
use std::path::Path;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::{self, Debug, Display},
    path::PathBuf,
};

use crate::entrypoint_artifact::generate_entrypoint_artifacts_with_client_field_traversal_result;
use crate::format_parameter_type::format_parameter_type;
use crate::{
    eager_reader_artifact::{
        generate_eager_reader_artifacts, generate_eager_reader_output_type_artifact,
        generate_eager_reader_param_type_artifact,
    },
    entrypoint_artifact::generate_entrypoint_artifacts,
    import_statements::ParamTypeImports,
    iso_overload_file::build_iso_overload_artifact,
    refetch_reader_artifact::{
        generate_refetch_output_type_artifact, generate_refetch_reader_artifact,
    },
};

lazy_static! {
    pub static ref RESOLVER_READER: ArtifactFileType = "resolver_reader".intern().into();
    pub static ref REFETCH_READER: ArtifactFileType = "refetch_reader".intern().into();
    pub static ref RESOLVER_PARAM_TYPE: ArtifactFileType = "param_type".intern().into();
    pub static ref RESOLVER_PARAMETERS_TYPE: ArtifactFileType = "parameters_type".intern().into();
    pub static ref RESOLVER_OUTPUT_TYPE: ArtifactFileType = "output_type".intern().into();
    pub static ref ENTRYPOINT: ArtifactFileType = "entrypoint".intern().into();
    pub static ref ISO_TS: ArtifactFileType = "iso".intern().into();
}

/// Get all artifacts according to the following scheme:
///
/// For each entrypoint, generate an entrypoint artifact. This involves
/// generating the merged selection map.
///
/// - While creating a client field's merged selection map, whenever we enter
///   a client field, we check a cache (`global_client_field_map`). If that
///   cache is empty, we populate it with the client field's merged selection
///   map, reachable variables, and paths to refetch fields.
/// - If that cache is full, we reuse the values in the cache.
/// - Using that value, we merge it the child field's selection map into the
///   parent's selection map, merge variables, etc.
///
/// For each field that we encounter in this way (including the entrypoint
/// itself), we must generate a resolver or refetch reader artifact. (Currently,
/// each field will have only one or the other; but once we have loadable fields,
/// the loadable field will have both a refetch and resolver reader artifact.)
///
/// Also, for each user-written resolver, we must generate a param_type artifact.
/// For each resolver that is reachable from a reader, we must also generate an
/// output_type artifact.
pub fn get_artifact_path_and_content(
    schema: &ValidatedSchema,
    project_root: &Path,
    artifact_directory: &Path,
) -> Vec<ArtifactPathAndContent> {
    let mut global_client_field_map = BTreeMap::new();
    let mut path_and_contents = vec![];
    let mut encountered_output_types = HashSet::<ClientFieldId>::new();

    // For each entrypoint, generate an entrypoint artifact and refetch artifacts
    for entrypoint_id in schema.entrypoints.iter() {
        let entrypoint_path_and_content =
            generate_entrypoint_artifacts(schema, *entrypoint_id, &mut global_client_field_map);
        path_and_contents.extend(entrypoint_path_and_content);

        // We also need to generate output types for entrypoints
        encountered_output_types.insert(*entrypoint_id);
    }

    for (
        encountered_client_field_id,
        ClientFieldTraversalResult {
            traversal_state,
            merged_selection_map,
            was_ever_selected_loadably,
            ..
        },
    ) in &global_client_field_map
    {
        let encountered_client_field = schema.client_field(*encountered_client_field_id);
        // Generate reader ASTs for all encountered client fields, which may be reader or refetch reader
        match &encountered_client_field.variant {
            ClientFieldVariant::UserWritten(info) => {
                path_and_contents.extend(generate_eager_reader_artifacts(
                    schema,
                    encountered_client_field,
                    project_root,
                    artifact_directory,
                    *info,
                    &traversal_state.refetch_paths,
                ));

                if *was_ever_selected_loadably {
                    path_and_contents.push(generate_refetch_reader_artifact(
                        schema,
                        encountered_client_field,
                        None,
                        &traversal_state.refetch_paths,
                        true,
                    ));

                    // Everything about this is quite sus
                    let id_arg = ArgumentKeyAndValue {
                        key: "id".intern().into(),
                        value: NonConstantValue::Variable("id".intern().into()),
                    };

                    let type_to_refine_to = schema
                        .server_field_data
                        .object(encountered_client_field.parent_object_id)
                        .name;

                    if schema
                        .fetchable_types
                        .contains_key(&encountered_client_field.parent_object_id)
                    {
                        panic!("Loadable fields on root objects are not yet supported");
                    }

                    let wrapped_map = selection_map_wrapped(
                        merged_selection_map.clone(),
                        "node".intern().into(),
                        vec![id_arg.clone()],
                        None,
                        RequiresRefinement::Yes(type_to_refine_to),
                    );
                    let id_var = ValidatedVariableDefinition {
                        name: WithLocation::new("id".intern().into(), Location::Generated),
                        type_: GraphQLTypeAnnotation::NonNull(Box::new(
                            GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                                WithSpan::new(
                                    SelectableServerFieldId::Scalar(schema.id_type_id),
                                    Span::todo_generated(),
                                ),
                            )),
                        )),
                        default_value: None,
                    };
                    let variable_definitions_iter = encountered_client_field
                        .variable_definitions
                        .iter()
                        .map(|variable_defition| &variable_defition.item)
                        .chain(std::iter::once(&id_var));
                    let mut traversal_state = traversal_state.clone();
                    traversal_state.refetch_paths = traversal_state
                        .refetch_paths
                        .into_iter()
                        .map(|(mut key, value)| {
                            key.0
                                .linked_fields
                                .insert(0, NormalizationKey::InlineFragment(type_to_refine_to));
                            key.0.linked_fields.insert(
                                0,
                                NormalizationKey::ServerField(NameAndArguments {
                                    name: "node".intern().into(),
                                    arguments: vec![id_arg.clone()],
                                }),
                            );
                            (key, value)
                        })
                        .collect();

                    path_and_contents.extend(
                        generate_entrypoint_artifacts_with_client_field_traversal_result(
                            schema,
                            encountered_client_field,
                            &wrapped_map,
                            &traversal_state,
                            &global_client_field_map,
                            variable_definitions_iter,
                            &schema.query_root_operation_name(),
                        ),
                    );
                }
            }
            ClientFieldVariant::ImperativelyLoadedField(variant) => {
                path_and_contents.push(generate_refetch_reader_artifact(
                    schema,
                    encountered_client_field,
                    variant.primary_field_info.as_ref(),
                    &traversal_state.refetch_paths,
                    false,
                ));
            }
        };
    }

    for user_written_client_field in
        schema
            .client_fields
            .iter()
            .filter(|field| match field.variant {
                ClientFieldVariant::UserWritten(_) => true,
                ClientFieldVariant::ImperativelyLoadedField(_) => false,
            })
    {
        // For each user-written client field, generate a param type artifact
        path_and_contents.push(generate_eager_reader_param_type_artifact(
            schema,
            user_written_client_field,
        ));

        match global_client_field_map.get(&user_written_client_field.id) {
            Some(ClientFieldTraversalResult {
                traversal_state, ..
            }) => {
                // If this user-written client field is reachable from an entrypoint,
                // we've already noted the accessible client fields
                encountered_output_types.extend(traversal_state.accessible_client_fields.iter())
            }
            None => {
                // If this field is not reachable from an entrypoint, we need to
                // encounter all the client fields
                for nested_client_field in
                    user_written_client_field.accessible_client_fields(schema)
                {
                    encountered_output_types.insert(nested_client_field.id);
                }
            }
        }
    }

    for output_type_id in encountered_output_types {
        let client_field = schema.client_field(output_type_id);
        let path_and_content = match client_field.variant {
            ClientFieldVariant::UserWritten(info) => generate_eager_reader_output_type_artifact(
                schema,
                client_field,
                project_root,
                artifact_directory,
                info,
            ),
            ClientFieldVariant::ImperativelyLoadedField(_) => {
                generate_refetch_output_type_artifact(schema, client_field)
            }
        };
        path_and_contents.push(path_and_content);
    }

    path_and_contents.push(build_iso_overload_artifact(schema));

    path_and_contents
}

pub(crate) fn get_serialized_field_arguments(
    // TODO make this an iterator
    arguments: &[ArgumentKeyAndValue],
    indentation_level: u8,
) -> String {
    if arguments.is_empty() {
        return "null".to_string();
    }

    let mut s = "[".to_string();
    let indent_1 = "  ".repeat((indentation_level + 1) as usize);
    let indent_2 = "  ".repeat((indentation_level + 2) as usize);

    for argument in arguments {
        let argument_name = argument.key;
        let arg_value = match &argument.value {
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
                    {indent_2}{{ kind: \"Literal\", value: {bool_string} }},\n\
                    {indent_1}],\n"
                )
            }
            NonConstantValue::String(s) => {
                format!(
                    "\n\
                    {indent_1}[\n\
                    {indent_2}\"{argument_name}\",\n\
                    {indent_2}{{ kind: \"String\", value: \"{s}\" }},\n\
                    {indent_1}],\n"
                )
            }
            NonConstantValue::Float(f) => {
                let float = f.as_float();
                format!(
                    "\n\
                    {indent_1}[\n\
                    {indent_2}\"{argument_name}\",\n\
                    {indent_2}{{ kind: \"Literal\", value: {float} }},\n\
                    {indent_1}],\n"
                )
            }
            NonConstantValue::Null => {
                format!(
                    "\n\
                    {indent_1}[\n\
                    {indent_2}\"{argument_name}\",\n\
                    {indent_2}{{ kind: \"Literal\", value: null }},\n\
                    {indent_1}],\n"
                )
            }
            NonConstantValue::Enum(e) => {
                format!(
                    "\n\
                    {indent_1}[\n\
                    {indent_2}\"{argument_name}\",\n\
                    {indent_2}{{ kind: \"Enum\", value: \"{e}\" }},\n\
                    {indent_1}],\n"
                )
            }
            NonConstantValue::List(_) => panic!("Lists are not supported here"),
            NonConstantValue::Object(_) => panic!("Objects not supported here"),
        };

        s.push_str(&arg_value);
    }

    s.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    s
}

pub(crate) fn generate_output_type(client_field: &ValidatedClientField) -> ClientFieldOutputType {
    let variant = &client_field.variant;
    match variant {
        ClientFieldVariant::UserWritten(info) => match info.user_written_component_variant {
            UserWrittenComponentVariant::Eager => {
                ClientFieldOutputType("ReturnType<typeof resolver>".to_string())
            }
            UserWrittenComponentVariant::Component => {
                ClientFieldOutputType("(React.FC<ExtractSecondParam<typeof resolver>>)".to_string())
            }
        },
        ClientFieldVariant::ImperativelyLoadedField(params) => {
            // N.B. the string is a stable id for deduplicating
            match params.primary_field_info {
                Some(_) => {
                    ClientFieldOutputType("(params: any) => [string, () => void]".to_string())
                }
                None => ClientFieldOutputType("() => [string, () => void]".to_string()),
            }
        }
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
    selection_map: &[WithSpan<ValidatedSelection>],
    parent_type: &SchemaObject,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) -> ClientFieldParameterType {
    // TODO use unwraps
    let mut client_field_parameter_type = "{\n".to_string();
    for selection in selection_map.iter() {
        write_param_type_from_selection(
            schema,
            &mut client_field_parameter_type,
            selection,
            parent_type,
            nested_client_field_imports,
            loadable_fields,
            indentation_level + 1,
        );
    }
    client_field_parameter_type.push_str(&format!("{}}}", "  ".repeat(indentation_level as usize)));

    ClientFieldParameterType(client_field_parameter_type)
}

fn write_param_type_from_selection(
    schema: &ValidatedSchema,
    query_type_declaration: &mut String,
    selection: &WithSpan<ValidatedSelection>,
    parent_type: &SchemaObject,
    nested_client_field_imports: &mut ParamTypeImports,
    loadable_fields: &mut ParamTypeImports,
    indentation_level: u8,
) {
    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field_selection) => {
                match scalar_field_selection.associated_data.location {
                    FieldDefinitionLocation::Server(_server_field) => {
                        let parent_field = parent_type
                            .encountered_fields
                            .get(&scalar_field_selection.name.item.into())
                            .expect("parent_field should exist 1")
                            .as_server_field()
                            .expect("parent_field should exist and be server field");
                        let field = schema.server_field(*parent_field);

                        write_optional_description(
                            field.description,
                            query_type_declaration,
                            indentation_level,
                        );

                        let name_or_alias = scalar_field_selection.name_or_alias().item;

                        // TODO there should be a clever way to print without cloning
                        let output_type =
                            field.associated_data.clone().map(&mut |output_type_id| {
                                // TODO not just scalars, enums as well. Both should have a javascript name
                                let scalar_id = if let SelectableServerFieldId::Scalar(scalar) =
                                    output_type_id
                                {
                                    scalar
                                } else {
                                    panic!("output_type_id should be a scalar");
                                };
                                schema.server_field_data.scalar(scalar_id).javascript_name
                            });

                        query_type_declaration.push_str(&format!(
                            "{}readonly {}: {},\n",
                            "  ".repeat(indentation_level as usize),
                            name_or_alias,
                            print_javascript_type_declaration(&output_type)
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
                            .push_str(&"  ".repeat(indentation_level as usize).to_string());

                        nested_client_field_imports.insert(client_field.type_and_field);
                        let inner_output_type = format!(
                            "{}__output_type",
                            client_field.type_and_field.underscore_separated()
                        );

                        let output_type =
                            match scalar_field_selection.associated_data.selection_variant {
                                ValidatedIsographSelectionVariant::Regular => inner_output_type,
                                ValidatedIsographSelectionVariant::Loadable(_) => {
                                    loadable_fields.insert(client_field.type_and_field);
                                    let provided_arguments = get_provided_arguments(
                                        client_field.variable_definitions.iter().map(|x| &x.item),
                                        &scalar_field_selection.arguments,
                                    );

                                    let indent = "  ".repeat((indentation_level + 1) as usize);
                                    let provided_args_type = if provided_arguments.is_empty() {
                                        "".to_string()
                                    } else {
                                        format!(
                                            ",\n{indent}{}",
                                            get_loadable_field_type_from_arguments(
                                                schema,
                                                provided_arguments
                                            )
                                        )
                                    };

                                    format!(
                                        "LoadableField<\n\
                                        {indent}{}__param,\n\
                                        {indent}{inner_output_type}\
                                        {provided_args_type}\n\
                                        {}>",
                                        client_field.type_and_field.underscore_separated(),
                                        "  ".repeat(indentation_level as usize),
                                    )
                                }
                            };

                        query_type_declaration.push_str(
                            &(format!(
                                "readonly {}: {},\n",
                                scalar_field_selection.name_or_alias().item,
                                output_type
                            )),
                        );
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
                    .push_str(&"  ".repeat(indentation_level as usize).to_string());
                let name_or_alias = linked_field.name_or_alias().item;

                let type_annotation = field.associated_data.clone().map(&mut |output_type_id| {
                    let object_id = output_type_id.try_into().expect(
                        "output_type_id should be an object. \
                        This is indicative of a bug in Isograph.",
                    );
                    let object = schema.server_field_data.object(object_id);
                    generate_client_field_parameter_type(
                        schema,
                        &linked_field.selection_set,
                        object,
                        nested_client_field_imports,
                        loadable_fields,
                        indentation_level,
                    )
                });
                query_type_declaration.push_str(&format!(
                    "readonly {}: {},\n",
                    name_or_alias,
                    print_javascript_type_declaration(&type_annotation),
                ));
            }
        },
    }
}

fn get_loadable_field_type_from_arguments(
    schema: &ValidatedSchema,
    arguments: Vec<ValidatedVariableDefinition>,
) -> String {
    let mut loadable_field_type = "{".to_string();
    let mut is_first = true;
    for arg in arguments.iter() {
        if !is_first {
            loadable_field_type.push_str(", ");
        }
        is_first = false;
        let is_optional = !matches!(arg.type_, GraphQLTypeAnnotation::NonNull(_));
        loadable_field_type.push_str(&format!(
            "readonly {}{}: {}",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_type_for_js(schema, arg.type_.clone())
        ));
    }
    loadable_field_type.push('}');
    loadable_field_type
}

fn format_type_for_js(
    schema: &ValidatedSchema,
    type_: GraphQLTypeAnnotation<SelectableServerFieldId>,
) -> String {
    let new_type = type_.map(
        |selectable_server_field_id| match selectable_server_field_id {
            SelectableServerFieldId::Object(_) => {
                panic!(
                    "Unexpected object. Objects are not supported as parameters, yet. \
                    This is indicative of an unimplemented feature in Isograph."
                )
            }
            SelectableServerFieldId::Scalar(scalar_id) => {
                schema.server_field_data.scalar(scalar_id).javascript_name
            }
        },
    );

    format_type_for_js_inner(new_type)
}

fn format_type_for_js_inner(
    new_type: GraphQLTypeAnnotation<common_lang_types::JavascriptName>,
) -> String {
    match new_type {
        GraphQLTypeAnnotation::Named(named_inner_type) => {
            format!("{} | null | void", named_inner_type.0.item)
        }
        GraphQLTypeAnnotation::List(list) => {
            format!("ReadonlyArray<{}> | null", format_type_for_js_inner(list.0))
        }
        GraphQLTypeAnnotation::NonNull(non_null) => match *non_null {
            GraphQLNonNullTypeAnnotation::Named(named_inner_type) => {
                named_inner_type.0.item.to_string()
            }
            GraphQLNonNullTypeAnnotation::List(list) => {
                format!("ReadonlyArray<{}>", format_type_for_js_inner(list.0))
            }
        },
    }
}

pub(crate) fn generate_parameters<'a>(
    schema: &ValidatedSchema,
    argument_definitions: impl Iterator<Item = &'a VariableDefinition<SelectableServerFieldId>>,
) -> String {
    let mut s = "{\n".to_string();
    let indent = "  ";
    for arg in argument_definitions {
        let is_optional = !matches!(arg.type_, GraphQLTypeAnnotation::NonNull(_));
        s.push_str(&format!(
            "{indent}readonly {}{}: {},\n",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_parameter_type(schema, arg.type_.clone(), 1)
        ));
    }
    s.push_str("};");
    s
}

fn write_optional_description(
    description: Option<DescriptionValue>,
    query_type_declaration: &mut String,
    indentation_level: u8,
) {
    if let Some(description) = description {
        query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
        query_type_declaration.push_str("/**\n");
        query_type_declaration.push_str(description.lookup());
        query_type_declaration.push('\n');
        query_type_declaration.push_str(&"  ".repeat(indentation_level as usize).to_string());
        query_type_declaration.push_str("*/\n");
    }
}

fn print_javascript_type_declaration<T: Display + Ord + Debug>(
    type_annotation: &TypeAnnotation<T>,
) -> String {
    let mut s = String::new();
    print_javascript_type_declaration_impl(type_annotation, &mut s);
    s
}

fn print_javascript_type_declaration_impl<T: Display + Ord + Debug>(
    type_annotation: &TypeAnnotation<T>,
    s: &mut String,
) {
    match &type_annotation {
        TypeAnnotation::Scalar(scalar) => {
            s.push_str(&scalar.to_string());
        }
        TypeAnnotation::Union(union_type_annotation) => {
            if union_type_annotation.variants.is_empty() {
                panic!("Unexpected union with not enough variants.");
            }

            if union_type_annotation.variants.len() > 1 || union_type_annotation.nullable {
                s.push('(');
                for (index, variant) in union_type_annotation.variants.iter().enumerate() {
                    if index != 0 {
                        s.push_str(" | ");
                    }

                    match variant {
                        UnionVariant::Scalar(scalar) => {
                            s.push_str(&scalar.to_string());
                        }
                        UnionVariant::Plural(type_annotation) => {
                            s.push_str("ReadonlyArray<");
                            print_javascript_type_declaration_impl(type_annotation, s);
                            s.push('>');
                        }
                    }
                }
                if union_type_annotation.nullable {
                    s.push_str(" | null");
                }
                s.push(')');
            } else {
                let variant = union_type_annotation
                    .variants
                    .first()
                    .expect("Expected variant to exist");
                match variant {
                    UnionVariant::Scalar(scalar) => {
                        s.push_str(&scalar.to_string());
                    }
                    UnionVariant::Plural(type_annotation) => {
                        s.push_str("ReadonlyArray<");
                        print_javascript_type_declaration_impl(type_annotation, s);
                        s.push('>');
                    }
                }
            }
        }
        TypeAnnotation::Plural(type_annotation) => {
            s.push_str("ReadonlyArray<");
            print_javascript_type_declaration_impl(type_annotation, s);
            s.push('>');
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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
pub(crate) struct RefetchQueryArtifactImport(pub String);
derive_display!(RefetchQueryArtifactImport);
