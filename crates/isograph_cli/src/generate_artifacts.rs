use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Debug, Display},
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use common_lang_types::{
    DefinedField, FieldNameOrAlias, HasName, IsographObjectTypeName, QueryOperationName,
    ResolverDefinitionPath, SelectableFieldName, TypeAndField, UnvalidatedTypeName, WithSpan,
};
use graphql_lang_types::{ListTypeAnnotation, NonNullTypeAnnotation, TypeAnnotation};
use isograph_lang_types::{
    NonConstantValue, ObjectId, OutputTypeId, ScalarId, Selection, SelectionFieldArgument,
    ServerFieldSelection,
};
use isograph_schema::{
    merge_selection_set, MergedSelection, MergedSelectionSet, ResolverVariant, SchemaObject,
    ValidatedDefinedField, ValidatedSchema, ValidatedSchemaIdField, ValidatedSchemaObject,
    ValidatedSchemaResolver, ValidatedSelection, ValidatedVariableDefinition,
};
use thiserror::Error;

pub(crate) fn generate_artifacts(
    schema: &ValidatedSchema,
    project_root: &PathBuf,
) -> Result<(), GenerateArtifactsError> {
    write_artifacts(get_all_artifacts(schema), project_root)?;

    Ok(())
}

fn get_all_artifacts<'schema>(
    schema: &'schema ValidatedSchema,
) -> impl Iterator<Item = Artifact<'schema>> + 'schema {
    schema.resolvers.iter().map(|resolver| {
        if resolver.is_fetchable {
            Artifact::FetchableResolver(generate_fetchable_resolver_artifact(schema, resolver))
        } else {
            Artifact::NonFetchableResolver(generate_non_fetchable_resolver_artifact(
                schema, resolver,
            ))
        }
    })
}

#[derive(Debug)]
pub struct QueryText(pub String);

fn generate_fetchable_resolver_artifact<'schema>(
    schema: &'schema ValidatedSchema,
    fetchable_resolver: &ValidatedSchemaResolver,
) -> FetchableResolver<'schema> {
    if let Some((ref selection_set, _)) = fetchable_resolver.selection_set_and_unwraps {
        let query_name = fetchable_resolver.name.into();

        let merged_selection_set = merge_selection_set(
            schema,
            // TODO here we are assuming that the resolver is only on the Query type.
            // That restriction should be loosened.
            schema
                .schema_data
                .object(schema.query_type_id.expect("expect query type to exist"))
                .into(),
            selection_set,
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
        let mut nested_resolver_artifact_imports: HashMap<TypeAndField, ResolverImport> =
            HashMap::new();
        let resolver_parameter_type = generate_resolver_parameter_type(
            schema,
            &selection_set,
            fetchable_resolver.variant,
            query_object.into(),
            &mut nested_resolver_artifact_imports,
            0,
        );
        let resolver_import_statement = generate_resolver_import_statement(
            fetchable_resolver.name,
            fetchable_resolver.resolver_definition_path,
            fetchable_resolver.has_associated_js_function,
        );
        let resolver_return_type = generate_resolver_return_type_declaration(
            fetchable_resolver.has_associated_js_function,
        );
        let resolver_read_out_type = generate_read_out_type(fetchable_resolver);
        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            query_object.into(),
            0,
            &mut nested_resolver_artifact_imports,
        );
        let convert_function =
            generate_convert_function(&fetchable_resolver.variant, fetchable_resolver.name);

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
        let reader_ast = generate_reader_ast(
            schema,
            selection_set,
            parent_type.into(),
            0,
            &mut nested_resolver_artifact_imports,
        );

        let resolver_parameter_type = generate_resolver_parameter_type(
            schema,
            &selection_set,
            non_fetchable_resolver.variant,
            parent_type.into(),
            &mut nested_resolver_artifact_imports,
            0,
        );
        let resolver_return_type = generate_resolver_return_type_declaration(
            non_fetchable_resolver.has_associated_js_function,
        );
        let resolver_read_out_type = generate_read_out_type(non_fetchable_resolver);
        let resolver_import_statement = generate_resolver_import_statement(
            non_fetchable_resolver.name,
            non_fetchable_resolver.resolver_definition_path,
            non_fetchable_resolver.has_associated_js_function,
        );
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
pub enum Artifact<'schema> {
    FetchableResolver(FetchableResolver<'schema>),
    NonFetchableResolver(NonFetchableResolver<'schema>),
}

#[derive(Debug)]
pub struct ResolverParameterType(pub String);

impl fmt::Display for ResolverParameterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug)]
pub struct ResolverImportStatement(pub String);

#[derive(Debug)]
pub struct ResolverReturnType(pub String);

#[derive(Debug)]
pub struct ResolverReadOutType(pub String);

#[derive(Debug)]
pub struct ReaderAst(pub String);

#[derive(Debug)]
pub struct ConvertFunction(pub String);

#[derive(Debug)]
pub struct FetchableResolver<'schema> {
    pub query_text: QueryText,
    pub query_name: QueryOperationName,
    pub parent_type: &'schema SchemaObject<ValidatedDefinedField>,
    pub resolver_import_statement: ResolverImportStatement,
    pub resolver_parameter_type: ResolverParameterType,
    pub resolver_return_type: ResolverReturnType,
    pub resolver_read_out_type: ResolverReadOutType,
    pub reader_ast: ReaderAst,
    pub nested_resolver_artifact_imports: HashMap<TypeAndField, ResolverImport>,
    pub convert_function: ConvertFunction,
}

impl<'schema> FetchableResolver<'schema> {
    fn file_contents(self) -> String {
        // TODO don't use merged, use regular selection set when generating fragment type
        // (i.e. we are not data masking)
        format!(
            "import type {{IsographFetchableResolver, ReaderAst, FragmentReference}} from '@isograph/react';\n\
            import {{ getRefRendererForName }} from '@isograph/react';\n\
            {}\n\
            {}\n\
            const queryText = '{}';\n\n\
            // TODO support changing this,\n\
            export type ReadFromStoreType = ResolverParameterType;\n\n\
            const normalizationAst = {{notNeededForDemo: true}};\n\
            const readerAst: ReaderAst<ReadFromStoreType> = {};\n\n\
            export type ResolverParameterType = {};\n\n\
            // The type, when returned from the resolver\n\
            export type ResolverReturnType = {};\n\n\
            {}\n\n\
            const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'FetchableResolver',\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}readerAst,\n\
            {}resolver: resolver as any,\n\
            {}convert: {},\n\
            }};\n\n\
            export default artifact;\n",
            self.resolver_import_statement.0,
            nested_resolver_names_to_import_statement(self.nested_resolver_artifact_imports),
            self.query_text.0,
            self.reader_ast.0,
            self.resolver_parameter_type.0,
            self.resolver_return_type.0,
            get_read_out_type_text(self.resolver_read_out_type),
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            self.convert_function.0,
        )
    }
}

fn get_read_out_type_text(read_out_type: ResolverReadOutType) -> String {
    format!("// the type, when read out (either via useLazyReference or via graph)\nexport type ReadOutType = {};", read_out_type.0)
}

#[derive(Debug)]
pub struct NonFetchableResolver<'schema> {
    pub parent_type: &'schema SchemaObject<ValidatedDefinedField>,
    pub resolver_field_name: SelectableFieldName,
    pub nested_resolver_artifact_imports: HashMap<TypeAndField, ResolverImport>,
    pub resolver_read_out_type: ResolverReadOutType,
    pub reader_ast: ReaderAst,
    pub resolver_parameter_type: ResolverParameterType,
    pub resolver_return_type: ResolverReturnType,
    pub resolver_import_statement: ResolverImportStatement,
}

impl<'schema> NonFetchableResolver<'schema> {
    pub fn file_contents(self) -> String {
        format!(
            "import type {{IsographNonFetchableResolver, ReaderAst}} from '@isograph/react';\n\
            {}\n\
            {}\n\
            {}\n\n\
            // TODO support changing this\n\
            export type ReadFromStoreType = ResolverParameterType;\n\n\
            const readerAst: ReaderAst<ReadFromStoreType> = {};\n\n\
            export type ResolverParameterType = {};\n\n\
            // The type, when returned from the resolver\n\
            export type ResolverReturnType = {};\n\n\
            const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'NonFetchableResolver',\n\
            {}resolver: resolver as any,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            self.resolver_import_statement.0,
            nested_resolver_names_to_import_statement(self.nested_resolver_artifact_imports),
            get_read_out_type_text(self.resolver_read_out_type),
            self.reader_ast.0,
            self.resolver_parameter_type.0,
            self.resolver_return_type.0,
            "  ",
            "  ",
            "  ",
        )
    }
}

fn generate_query_text(
    query_name: QueryOperationName,
    schema: &ValidatedSchema,
    merged_selection_set: &MergedSelectionSet,
    query_variables: &[WithSpan<ValidatedVariableDefinition>],
) -> QueryText {
    let mut query_text = String::new();

    let variable_text = write_variables_to_string(schema, query_variables);

    query_text.push_str(&format!("query {} {} {{\\\n", query_name, variable_text));
    write_selections_for_query_text(
        &mut query_text,
        schema,
        schema
            .query_object()
            .expect("Expected query object to exist"),
        // TODO do not do this here, instead do it during validation, and topologically sort first
        &merged_selection_set,
        1,
    );
    query_text.push_str("}");
    QueryText(query_text)
}

fn write_variables_to_string(
    schema: &ValidatedSchema,
    variables: &[WithSpan<ValidatedVariableDefinition>],
) -> String {
    if variables.is_empty() {
        String::new()
    } else {
        let mut variable_text = String::new();
        variable_text.push('(');
        for (i, variable) in variables.iter().enumerate() {
            if i != 0 {
                variable_text.push_str(", ");
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

fn generated_file_name(
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableFieldName,
) -> PathBuf {
    PathBuf::from(format!("{}__{}.isograph.tsx", parent_type_name, field_name))
}

fn generated_file_path(project_root: &PathBuf, file_name: &PathBuf) -> PathBuf {
    project_root.join(file_name)
}

fn write_selections_for_query_text(
    query_text: &mut String,
    schema: &ValidatedSchema,
    root_object: &ValidatedSchemaObject,
    items: &[WithSpan<MergedSelection>],
    indentation_level: u8,
) {
    let id_field: Option<ValidatedSchemaIdField> = root_object
        .id_field
        .map(|id_field_id| schema.id_field(id_field_id));

    // If the type has an id field, we must select it.
    if let Some(id_field) = id_field {
        query_text.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
        query_text.push_str(&format!("{},\\\n", id_field.name));
    }

    'item: for item in items.iter() {
        match &item.item {
            Selection::ServerField(field) => {
                match field {
                    ServerFieldSelection::ScalarField(scalar_field) => {
                        // ------ HACK ------
                        // Here, we are avoiding selecting the id field twice.
                        if let Some(id_field) = id_field {
                            // Note: we aren't checking for reader alias because reader aliases
                            // don't exist in generated query texts! We can check for the presence
                            // of a normalization alias, but we know that that won't exist for
                            // a field with no arguments (as we are assuming is the case with ID).

                            // THIS IS BLATANTLY WRONG!!
                            // This causes us to skip fields with type ID, in addition to the "ID"
                            // field.
                            if scalar_field.field == id_field.field_type.0.item {
                                continue 'item;
                            }
                        }
                        // ---- END HACK ----
                        query_text
                            .push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
                        if let Some(alias) = scalar_field.normalization_alias {
                            query_text.push_str(&format!("{}: ", alias));
                        }
                        let name = scalar_field.name.item;
                        let arguments = get_serialized_arguments(&scalar_field.arguments);
                        query_text.push_str(&format!("{}{},\\\n", name, arguments));
                    }
                    ServerFieldSelection::LinkedField(linked_field) => {
                        query_text
                            .push_str(&format!("{}", "  ".repeat(indentation_level as usize)));
                        if let Some(alias) = linked_field.normalization_alias {
                            query_text.push_str(&format!("{}: ", alias));
                        }
                        let name = linked_field.name.item;
                        let arguments = get_serialized_arguments(&linked_field.arguments);
                        query_text.push_str(&format!("{}{} {{\\\n", name, arguments));
                        write_selections_for_query_text(
                            query_text,
                            schema,
                            schema.schema_data.object(linked_field.field),
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
    }
}

fn write_artifacts<'schema>(
    artifacts: impl Iterator<Item = Artifact<'schema>> + 'schema,
    project_root: &PathBuf,
) -> Result<(), GenerateArtifactsError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let project_root = current_dir.join(project_root).canonicalize().map_err(|e| {
        GenerateArtifactsError::UnableToCanonicalizePath {
            path: project_root.clone(),
            message: e,
        }
    })?;

    let generated_folder_root = project_root.join("__isograph");

    if generated_folder_root.exists() {
        fs::remove_dir_all(&generated_folder_root).map_err(|e| {
            GenerateArtifactsError::UnableToDeleteDirectory {
                path: project_root.clone(),
                message: e,
            }
        })?;
    }
    fs::create_dir_all(&generated_folder_root).map_err(|e| {
        GenerateArtifactsError::UnableToCreateDirectory {
            path: project_root.clone(),
            message: e,
        }
    })?;
    for artifact in artifacts {
        match artifact {
            Artifact::FetchableResolver(fetchable_resolver) => {
                let FetchableResolver {
                    query_name,
                    parent_type,
                    ..
                } = &fetchable_resolver;

                let generated_file_name =
                    generated_file_name(parent_type.name, (*query_name).into());
                let generated_file_path =
                    generated_file_path(&generated_folder_root, &generated_file_name);

                let mut file = File::create(&generated_file_path).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;

                let file_contents = fetchable_resolver.file_contents();

                file.write(file_contents.as_bytes()).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;
            }
            Artifact::NonFetchableResolver(non_fetchable_resolver) => {
                let NonFetchableResolver {
                    parent_type,
                    resolver_field_name,
                    ..
                } = &non_fetchable_resolver;

                let generated_file_name =
                    generated_file_name(parent_type.name, *resolver_field_name);
                let generated_file_path =
                    generated_file_path(&generated_folder_root, &generated_file_name);

                let mut file = File::create(&generated_file_path).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;

                let file_contents = non_fetchable_resolver.file_contents();

                file.write(file_contents.as_bytes()).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;
            }
        }
    }
    Ok(())
}

fn generate_resolver_parameter_type(
    schema: &ValidatedSchema,
    selection_set: &Vec<WithSpan<ValidatedSelection>>,
    variant: Option<WithSpan<ResolverVariant>>,
    parent_type: &SchemaObject<ValidatedDefinedField>,
    nested_resolver_imports: &mut HashMap<TypeAndField, ResolverImport>,
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

    if let Some(ResolverVariant::Component) = variant.map(|v| v.item) {
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
    variant: Option<WithSpan<ResolverVariant>>,
    parent_type: &SchemaObject<ValidatedDefinedField>,
    nested_resolver_imports: &mut HashMap<TypeAndField, ResolverImport>,
    indentation_level: u8,
) {
    query_type_declaration.push_str(&format!("{}", "  ".repeat(indentation_level as usize)));

    match &selection.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                match scalar_field.field {
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
                        let output_type = field.field_type.clone().map(|output_type_id| {
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
                    DefinedField::ResolverField((name_or_alias, type_and_field)) => {
                        let resolver = schema
                            .resolvers
                            .iter()
                            .find(|x| x.type_and_field == type_and_field)
                            .expect("Expected resolver to exist. Probably indicates a bug in Isograph at this point.");

                        match nested_resolver_imports.entry(resolver.type_and_field) {
                            Entry::Occupied(mut occupied) => {
                                occupied.get_mut().types.push(ResolverImportType {
                                    original: ResolverImportName("ReadOutType".to_string()),
                                    alias: ResolverImportAlias(format!(
                                        "{}__outputType",
                                        resolver.type_and_field
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
                                            resolver.type_and_field
                                        )),
                                    }],
                                });
                            }
                        }

                        query_type_declaration.push_str(&format!(
                            "{}: {}__outputType,\n",
                            name_or_alias, resolver.type_and_field
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
                let type_annotation = field.field_type.clone().map(|output_type_id| {
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
    resolver_name: SelectableFieldName,
    resolver_path: ResolverDefinitionPath,
    has_associated_js_function: bool,
) -> ResolverImportStatement {
    // TODO make this an enum/option instead of three variables
    if has_associated_js_function {
        // ../ gets us to the project root from the __isograph folder
        ResolverImportStatement(format!(
            "import {{ {} as resolver }} from '../{}';",
            resolver_name, resolver_path
        ))
    } else {
        ResolverImportStatement("const resolver = x => x;".to_string())
    }
}

#[derive(Debug)]
struct ResolverImportName(pub String);
#[derive(Debug)]
struct ResolverImportAlias(pub String);

#[derive(Debug)]
pub struct ResolverImportType {
    original: ResolverImportName,
    alias: ResolverImportAlias,
}
#[derive(Debug)]
pub struct ResolverImport {
    default_import: bool,
    types: Vec<ResolverImportType>,
}

fn generate_reader_ast<'schema>(
    schema: &'schema ValidatedSchema,
    selection_set: &'schema Vec<WithSpan<ValidatedSelection>>,
    parent_type: &SchemaObject<ValidatedDefinedField>,
    indentation_level: u8,
    nested_resolver_imports: &mut HashMap<TypeAndField, ResolverImport>,
) -> ReaderAst {
    let mut reader_ast = "[\n".to_string();
    for item in selection_set {
        let s = generate_reader_ast_node(
            item,
            parent_type,
            schema,
            indentation_level + 1,
            nested_resolver_imports,
        );
        reader_ast.push_str(&s);
    }
    reader_ast.push_str(&format!("{}]", "  ".repeat(indentation_level as usize)));
    ReaderAst(reader_ast)
}

fn generate_reader_ast_node(
    item: &WithSpan<Selection<DefinedField<ScalarId, (FieldNameOrAlias, TypeAndField)>, ObjectId>>,
    parent_type: &SchemaObject<ValidatedDefinedField>,
    schema: &ValidatedSchema,
    indentation_level: u8,
    nested_resolver_imports: &mut HashMap<TypeAndField, ResolverImport>,
) -> String {
    match &item.item {
        Selection::ServerField(field) => match field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                let field_name = scalar_field.name.item;

                match scalar_field.field {
                    DefinedField::ServerField(_server_field) => {
                        let alias = scalar_field
                            .reader_alias
                            .map(|x| format!("\"{}\"", x.item))
                            .unwrap_or("null".to_string());
                        let arguments =
                            format_arguments(&scalar_field.arguments, indentation_level + 1);
                        format!(
                            "{}{{\n{}kind: \"Scalar\",\n{}response_name: \"{}\",\n{}alias: {},\n{}arguments: {},\n{}}},\n",
                            "  ".repeat(indentation_level as usize),
                            "  ".repeat((indentation_level + 1) as usize),
                            "  ".repeat((indentation_level + 1) as usize),
                            field_name,
                            "  ".repeat((indentation_level + 1) as usize),
                            alias,
                            "  ".repeat((indentation_level + 1) as usize),
                            arguments,
                            "  ".repeat((indentation_level) as usize),
                        )
                    }
                    DefinedField::ResolverField(_) => {
                        let alias = scalar_field.name_or_alias().item;
                        // This field is a resolver, so we need to look up the field in the
                        // schema.
                        let resolver_field_name = scalar_field.name.item;
                        let resolver_field_id = parent_type
                            .resolvers
                            .iter()
                            .find(|parent_field_id| {
                                let field = schema.resolver(**parent_field_id);
                                field.name == resolver_field_name.into()
                            })
                            .expect("expect field to exist");
                        let resolver_field = schema.resolver(*resolver_field_id);
                        let arguments =
                            format_arguments(&scalar_field.arguments, indentation_level + 1);
                        let res = format!(
                                    "{}{{\n{}kind: \"Resolver\",\n{}alias: \"{}\",\n{}arguments: {},\n{}resolver: {},\n{}variant: {},\n{}}},\n",
                                    "  ".repeat(indentation_level as usize),
                                    "  ".repeat((indentation_level + 1) as usize),
                                    "  ".repeat((indentation_level + 1) as usize),
                                    alias,
                                    "  ".repeat((indentation_level + 1) as usize),
                                    arguments,
                                    "  ".repeat((indentation_level + 1) as usize),
                                    resolver_field.type_and_field,
                                    "  ".repeat((indentation_level + 1) as usize),
                                    resolver_field.variant.map(|x| format!("\"{}\"", x)).unwrap_or_else(|| "null".to_string()),
                                    "  ".repeat(indentation_level as usize),
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
                        res
                    }
                }
            }
            ServerFieldSelection::LinkedField(linked_field) => {
                let name = linked_field.name.item;
                let alias = linked_field
                    .reader_alias
                    .map(|x| format!("\"{}\"", x.item))
                    .unwrap_or("null".to_string());
                let linked_field_type = schema.schema_data.object(linked_field.field);
                let inner_reader_ast = generate_reader_ast(
                    schema,
                    &linked_field.selection_set,
                    linked_field_type,
                    indentation_level + 1,
                    nested_resolver_imports,
                );
                let arguments = format_arguments(&linked_field.arguments, indentation_level + 1);
                format!(
                    "{}{{\n{}kind: \"Linked\",\n{}response_name: \"{}\",\n{}alias: {},\n{}arguments: {},\n{}selections: {},\n{}}},\n",
                    "  ".repeat(indentation_level as usize),
                    "  ".repeat((indentation_level + 1) as usize),
                    "  ".repeat((indentation_level + 1) as usize),
                    name,
                    "  ".repeat((indentation_level + 1) as usize),
                    alias,
                    "  ".repeat((indentation_level + 1) as usize),
                    arguments,
                    "  ".repeat((indentation_level + 1) as usize),
                    inner_reader_ast.0, "  ".repeat(indentation_level as usize),
                )
            }
        },
    }
}

fn nested_resolver_names_to_import_statement(
    nested_resolver_imports: HashMap<TypeAndField, ResolverImport>,
) -> String {
    let mut overall = String::new();

    // TODO we should always sort outputs. We should find a nice generic way to ensure that.
    let mut nested_resolver_imports: Vec<_> = nested_resolver_imports.into_iter().collect();
    nested_resolver_imports.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (nested_resolver_name, resolver_import) in nested_resolver_imports {
        if !resolver_import.default_import && resolver_import.types.is_empty() {
            continue;
        }

        let mut s = "import ".to_string();
        if resolver_import.default_import {
            s.push_str(&format!("{}", nested_resolver_name));
        }
        let mut types = resolver_import.types.iter();
        if let Some(first) = types.next() {
            if resolver_import.default_import {
                s.push_str(",");
            }
            s.push_str(" { ");
            s.push_str(&format!("{} as {} ", first.original.0, first.alias.0));
            for value in types {
                s.push_str(&format!(", {} as {} ", value.original.0, value.alias.0));
            }
            s.push_str("}");
        }
        s.push_str(&format!(" from './{}.isograph';\n", nested_resolver_name));
        overall.push_str(&s);
    }
    overall
}

fn get_serialized_arguments(arguments: &[WithSpan<SelectionFieldArgument>]) -> String {
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

fn format_arguments(
    arguments: &[WithSpan<SelectionFieldArgument>],
    indentation_level: u8,
) -> String {
    if arguments.is_empty() {
        return "null".to_string();
    } else {
        let mut out_str = "{".to_string();
        for argument in arguments {
            out_str.push_str(&format!(
                "\n{}\"{}\": {},",
                "  ".repeat((indentation_level + 1) as usize),
                argument.item.name.item,
                serialize_non_constant_value_for_js(&argument.item.value.item)
            ));
        }
        out_str.push_str(&format!("\n{}}}", "  ".repeat(indentation_level as usize)));
        out_str
    }
}

fn generate_read_out_type(resolver_definition: &ValidatedSchemaResolver) -> ResolverReadOutType {
    match resolver_definition.variant {
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
    has_associated_js_function: bool,
) -> ResolverReturnType {
    if has_associated_js_function {
        ResolverReturnType("ReturnType<typeof resolver>".to_string())
    } else {
        ResolverReturnType("ResolverParameterType".to_string())
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
                {}const RefRendererForName = getRefRendererForName('{}');\n\
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
                    field_name,
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
