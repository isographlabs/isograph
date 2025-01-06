use common_lang_types::ArtifactPathAndContent;
use intern::Lookup;

use isograph_config::GenerateFileExtensionsOption;
use isograph_schema::{
    RefetchedPathsMap, ServerFieldTypeAssociatedDataInlineFragment, UserWrittenClientFieldInfo,
    UserWrittenComponentVariant, ValidatedClientField, ValidatedSchema, ValidatedSchemaServerField,
};
use std::{
    borrow::Cow,
    collections::BTreeSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    generate_artifacts::{
        generate_client_field_parameter_type, generate_output_type, generate_parameters,
        generate_path, ClientFieldFunctionImportStatement, RESOLVER_OUTPUT_TYPE,
        RESOLVER_PARAMETERS_TYPE, RESOLVER_PARAM_TYPE, RESOLVER_READER,
    },
    import_statements::{
        param_type_imports_to_import_param_statement, param_type_imports_to_import_statement,
        reader_imports_to_import_statement,
    },
    reader_ast::generate_reader_ast,
};

pub(crate) fn generate_eager_reader_artifacts(
    schema: &ValidatedSchema,
    client_field: &ValidatedClientField,
    project_root: &Path,
    artifact_directory: &Path,
    info: UserWrittenClientFieldInfo,
    refetched_paths: &RefetchedPathsMap,
    file_extensions: GenerateFileExtensionsOption,
) -> Vec<ArtifactPathAndContent> {
    let ts_file_extension = file_extensions.ts();
    let user_written_component_variant = info.user_written_component_variant;
    let parent_type = schema
        .server_field_data
        .object(client_field.parent_object_id);

    let (reader_ast, reader_imports) = generate_reader_ast(
        schema,
        client_field.selection_set_for_parent_query(),
        0,
        refetched_paths,
        &client_field.initial_variable_context(),
    );

    let function_import_statement =
        generate_function_import_statement(project_root, artifact_directory, info, file_extensions);

    let relative_directory = generate_path(parent_type.name, client_field.name);

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

    let reader_param_type = format!("{}__{}__param", parent_type.name, client_field.name);

    let reader_content = if let UserWrittenComponentVariant::Eager = user_written_component_variant
    {
        let reader_output_type =
            format!("{}__{}__output_type", parent_type.name, client_field.name);
        let param_type_file_name = *RESOLVER_PARAM_TYPE;
        let output_type_file_name = *RESOLVER_OUTPUT_TYPE;
        format!(
            "import type {{ EagerReaderArtifact, ReaderAst }} from '@isograph/react';\n\
            import {{ {reader_param_type} }} from './{param_type_file_name}{ts_file_extension}';\n\
            import {{ {reader_output_type} }} from './{output_type_file_name}{ts_file_extension}';\n\
            {function_import_statement}\n\
            {reader_import_statement}\n\
            const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
            const artifact: EagerReaderArtifact<\n\
            {}{reader_param_type},\n\
            {}{reader_output_type}\n\
            > = {{\n\
            {}kind: \"EagerReaderArtifact\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ",
        )
    } else {
        let component_name = format!("{}.{}", parent_type.name, client_field.name);
        let param_type_file_name = *RESOLVER_PARAM_TYPE;
        format!(
            "import type {{ComponentReaderArtifact, ExtractSecondParam, \
            ReaderAst }} from '@isograph/react';\n\
            import {{ {reader_param_type} }} from './{param_type_file_name}{ts_file_extension}';\n\
            {function_import_statement}\n\
            {reader_import_statement}\n\
            const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
            const artifact: ComponentReaderArtifact<\n\
            {}{reader_param_type},\n\
            {}ExtractSecondParam<typeof resolver>\n\
            > = {{\n\
            {}kind: \"ComponentReaderArtifact\",\n\
            {}componentName: \"{component_name}\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ",
        )
    };

    let mut path_and_contents = vec![ArtifactPathAndContent {
        relative_directory: relative_directory.clone(),
        file_name_prefix: *RESOLVER_READER,
        file_content: reader_content,
    }];

    if !client_field.variable_definitions.is_empty() {
        let reader_parameters_type =
            format!("{}__{}__parameters", parent_type.name, client_field.name);
        let parameters = client_field.variable_definitions.iter().map(|x| &x.item);
        let parameters_types = generate_parameters(schema, parameters);
        let parameters_content =
            format!("export type {reader_parameters_type} = {parameters_types}\n");
        path_and_contents.push(ArtifactPathAndContent {
            relative_directory,
            file_name_prefix: *RESOLVER_PARAMETERS_TYPE,
            file_content: parameters_content,
        });
    }

    path_and_contents
}

pub(crate) fn generate_eager_reader_condition_artifact(
    schema: &ValidatedSchema,
    encountered_server_field: &ValidatedSchemaServerField,
    inline_fragment: &ServerFieldTypeAssociatedDataInlineFragment,
    refetch_paths: &RefetchedPathsMap,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let field_name = encountered_server_field.name.item;

    let parent_type = schema
        .server_field_data
        .object(encountered_server_field.parent_type_id);
    let concrete_type = inline_fragment.concrete_type;

    let (reader_ast, reader_imports) = generate_reader_ast(
        schema,
        &inline_fragment.condition_selection_set,
        0,
        refetch_paths,
        &encountered_server_field.initial_variable_context(),
    );

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

    let reader_param_type = "{ data: any, parameters: Record<PropertyKey, never> }";
    let reader_output_type = "Link | null";

    let reader_content = format!(
        "import type {{ EagerReaderArtifact, ReaderAst, Link }} from '@isograph/react';\n\
        {reader_import_statement}\n\
        const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
        const artifact: EagerReaderArtifact<\n\
        {}{reader_param_type},\n\
        {}{reader_output_type}\n\
        > = {{\n\
        {}kind: \"EagerReaderArtifact\",\n\
        {}resolver: ({{ data }}) => data.__typename === \"{concrete_type}\" ? data.link : null,\n\
        {}readerAst,\n\
        }};\n\n\
        export default artifact;\n",
        "  ", "  ", "  ", "  ", "  ",
    );

    let relative_directory = generate_path(parent_type.name, field_name);

    ArtifactPathAndContent {
        relative_directory: relative_directory.clone(),
        file_name_prefix: *RESOLVER_READER,
        file_content: reader_content,
    }
}

pub(crate) fn generate_eager_reader_param_type_artifact(
    schema: &ValidatedSchema,
    client_field: &ValidatedClientField,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let ts_file_extension = file_extensions.ts();
    let parent_type = schema
        .server_field_data
        .object(client_field.parent_object_id);
    let relative_directory = generate_path(parent_type.name, client_field.name);

    let mut param_type_imports = BTreeSet::new();
    let mut loadable_fields = BTreeSet::new();
    let mut link_fields = false;
    let client_field_parameter_type = generate_client_field_parameter_type(
        schema,
        client_field.selection_set_for_parent_query(),
        parent_type,
        &mut param_type_imports,
        &mut loadable_fields,
        1,
        &mut link_fields,
    );

    let param_type_import_statement =
        param_type_imports_to_import_statement(&param_type_imports, file_extensions);
    let reader_param_type = format!("{}__{}__param", parent_type.name, client_field.name);

    let link_field_imports = if link_fields {
        "import type { Link } from '@isograph/react';\n".to_string()
    } else {
        "".to_string()
    };

    let loadable_field_imports = if !loadable_fields.is_empty() {
        let param_imports =
            param_type_imports_to_import_param_statement(&loadable_fields, file_extensions);
        format!(
            "import {{ type LoadableField, type ExtractParameters }} from '@isograph/react';\n\
            {param_imports}"
        )
    } else {
        "".to_string()
    };

    let (parameters_import, parameters_type) = if !client_field.variable_definitions.is_empty() {
        let reader_parameters_type =
            format!("{}__{}__parameters", parent_type.name, client_field.name);
        (
            format!("import type {{ {reader_parameters_type} }} from './parameters_type{ts_file_extension}';\n"),
            reader_parameters_type,
        )
    } else {
        ("".to_string(), "Record<PropertyKey, never>".to_string())
    };

    let indent = "  ";
    let param_type_content = format!(
        "{param_type_import_statement}\
        {link_field_imports}\
        {loadable_field_imports}\
        {parameters_import}\n\
        export type {reader_param_type} = {{\n\
        {indent}readonly data: {client_field_parameter_type},\n\
        {indent}readonly parameters: {parameters_type},\n\
        }};\n",
    );
    ArtifactPathAndContent {
        relative_directory: relative_directory.clone(),
        file_name_prefix: *RESOLVER_PARAM_TYPE,
        file_content: param_type_content,
    }
}

pub(crate) fn generate_eager_reader_output_type_artifact(
    schema: &ValidatedSchema,
    client_field: &ValidatedClientField,
    project_root: &Path,
    artifact_directory: &Path,
    info: UserWrittenClientFieldInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let parent_type = schema
        .server_field_data
        .object(client_field.parent_object_id);
    let relative_directory = generate_path(parent_type.name, client_field.name);

    let function_import_statement =
        generate_function_import_statement(project_root, artifact_directory, info, file_extensions);

    let client_field_output_type = generate_output_type(client_field);

    let output_type_text = format!(
        "import type React from 'react';\n\
        {function_import_statement}\n\
        export type {}__{}__output_type = {};",
        parent_type.name, client_field.name, client_field_output_type
    );

    let final_output_type_text =
        if let UserWrittenComponentVariant::Eager = info.user_written_component_variant {
            output_type_text
        } else {
            format!(
                "import type {{ ExtractSecondParam, CombineWithIntrinsicAttributes }} \
                from '@isograph/react';\n\
                {output_type_text}\n",
            )
        };

    ArtifactPathAndContent {
        relative_directory: relative_directory.clone(),
        file_name_prefix: *RESOLVER_OUTPUT_TYPE,
        file_content: final_output_type_text,
    }
}

/// Example: import { PetUpdater as resolver } from '../../../PetUpdater';
fn generate_function_import_statement(
    project_root: &Path,
    artifact_directory: &Path,
    user_written_client_field_info: UserWrittenClientFieldInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> ClientFieldFunctionImportStatement {
    let const_export_name = user_written_client_field_info.const_export_name;
    let path_to_client_field = project_root.join(
        PathBuf::from_str(user_written_client_field_info.file_path.lookup())
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
    let complete_file_name = relative_path.to_str().expect(
        "This path should be stringifiable. This probably is indicative of a bug in Isograph.",
    );

    let normalized_file_name = if cfg!(windows) {
        Cow::Owned(complete_file_name.replace("\\", "/"))
    } else {
        Cow::Borrowed(complete_file_name)
    };

    let file_name = match file_extensions {
        GenerateFileExtensionsOption::ExcludeExtensionsInFileImports => {
            let extension_char_count_including_dot =
                relative_path.extension().map(|x| x.len() + 1).unwrap_or(0);
            &normalized_file_name
                [0..(normalized_file_name.len() - extension_char_count_including_dot)]
        }
        GenerateFileExtensionsOption::IncludeExtensionsInFileImports => &normalized_file_name,
    };

    ClientFieldFunctionImportStatement(format!(
        "import {{ {const_export_name} as resolver }} from '{}';",
        file_name
    ))
}
