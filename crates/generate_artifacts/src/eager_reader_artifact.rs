use common_lang_types::{ArtifactPathAndContent, ObjectTypeAndFieldName, WithSpan};
use intern::Lookup;

use isograph_config::{CompilerConfig, GenerateFileExtensionsOption};

use isograph_lang_types::{ClientFieldDirectiveSet, SelectionType};
use isograph_schema::{
    initial_variable_context, ClientScalarOrObjectSelectable, ClientSelectable, NetworkProtocol,
    Schema, ServerObjectSelectable, ValidatedSelection,
};
use isograph_schema::{RefetchedPathsMap, UserWrittenClientTypeInfo};

use std::{borrow::Cow, collections::BTreeSet, path::PathBuf};

use crate::generate_artifacts::ClientFieldOutputType;
use crate::{
    generate_artifacts::{
        generate_client_field_parameter_type, generate_client_field_updatable_data_type,
        generate_output_type, generate_parameters, ClientFieldFunctionImportStatement,
        RESOLVER_OUTPUT_TYPE, RESOLVER_OUTPUT_TYPE_FILE_NAME, RESOLVER_PARAMETERS_TYPE_FILE_NAME,
        RESOLVER_PARAM_TYPE, RESOLVER_PARAM_TYPE_FILE_NAME, RESOLVER_READER_FILE_NAME,
    },
    import_statements::{
        param_type_imports_to_import_param_statement, param_type_imports_to_import_statement,
        reader_imports_to_import_statement,
    },
    reader_ast::generate_reader_ast,
};

pub(crate) fn generate_eager_reader_artifacts<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    client_selectable: &ClientSelectable<TNetworkProtocol>,
    config: &CompilerConfig,
    info: UserWrittenClientTypeInfo,
    refetched_paths: &RefetchedPathsMap,
    file_extensions: GenerateFileExtensionsOption,
    has_updatable: bool,
) -> Vec<ArtifactPathAndContent> {
    let ts_file_extension = file_extensions.ts();
    let user_written_component_variant = info.client_field_directive_set;
    let parent_object_entity = schema
        .server_entity_data
        .server_object_entity(client_selectable.parent_object_entity_id());

    let (reader_ast, reader_imports) = generate_reader_ast(
        schema,
        client_selectable.selection_set_for_parent_query(),
        0,
        refetched_paths,
        &initial_variable_context(client_selectable),
    );

    let function_import_statement =
        generate_function_import_statement(config, info, file_extensions);

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

    let reader_param_type = format!(
        "{}__{}__param",
        parent_object_entity.name,
        client_selectable.name()
    );

    let reader_content = if let ClientFieldDirectiveSet::None(_) = user_written_component_variant {
        let eager_reader_name =
            format!("{}.{}", parent_object_entity.name, client_selectable.name());
        let reader_output_type = format!(
            "{}__{}__output_type",
            parent_object_entity.name,
            client_selectable.name()
        );
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
            {}fieldName: \"{eager_reader_name}\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            {}hasUpdatable: {has_updatable},\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ", "  ",
        )
    } else {
        let component_name = format!("{}.{}", parent_object_entity.name, client_selectable.name());
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
            {}fieldName: \"{component_name}\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            {}hasUpdatable: {has_updatable},\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ", "  "
        )
    };

    let mut path_and_contents = vec![ArtifactPathAndContent {
        file_name: *RESOLVER_READER_FILE_NAME,
        file_content: reader_content,
        type_and_field: Some(ObjectTypeAndFieldName {
            type_name: parent_object_entity.name,
            field_name: client_selectable.name().into(),
        }),
    }];

    if !client_selectable.variable_definitions().is_empty() {
        let reader_parameters_type = format!(
            "{}__{}__parameters",
            parent_object_entity.name,
            client_selectable.name()
        );
        let parameters = client_selectable
            .variable_definitions()
            .iter()
            .map(|x| &x.item);
        let parameters_types = generate_parameters(schema, parameters);
        let parameters_content =
            format!("export type {reader_parameters_type} = {parameters_types}\n");
        path_and_contents.push(ArtifactPathAndContent {
            file_name: *RESOLVER_PARAMETERS_TYPE_FILE_NAME,
            file_content: parameters_content,
            type_and_field: Some(ObjectTypeAndFieldName {
                type_name: parent_object_entity.name,
                field_name: client_selectable.name().into(),
            }),
        });
    }

    path_and_contents
}

pub(crate) fn generate_eager_reader_condition_artifact<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    server_object_selectable: &ServerObjectSelectable<TNetworkProtocol>,
    inline_fragment_reader_selections: &[WithSpan<ValidatedSelection>],
    refetch_paths: &RefetchedPathsMap,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let server_object_selectable_name = server_object_selectable.name.item;

    let parent_object_entity = schema
        .server_entity_data
        .server_object_entity(server_object_selectable.parent_object_entity_id);

    let concrete_type = schema
        .server_entity_data
        .server_object_entity(*server_object_selectable.target_object_entity.inner())
        .name;

    let (reader_ast, reader_imports) = generate_reader_ast(
        schema,
        inline_fragment_reader_selections,
        0,
        refetch_paths,
        &server_object_selectable.initial_variable_context(),
    );

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

    let reader_param_type = "{ data: any, parameters: Record<PropertyKey, never> }";
    let reader_output_type = "Link | null";

    let eager_reader_name = format!(
        "{}.{}",
        parent_object_entity.name, server_object_selectable_name
    );

    let reader_content = format!(
        "import type {{ EagerReaderArtifact, ReaderAst, Link }} from '@isograph/react';\n\
        {reader_import_statement}\n\
        const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
        const artifact: EagerReaderArtifact<\n\
        {}{reader_param_type},\n\
        {}{reader_output_type}\n\
        > = {{\n\
        {}kind: \"EagerReaderArtifact\",\n\
        {}fieldName: \"{eager_reader_name}\",\n\
        {}resolver: ({{ data }}) => data.__typename === \"{concrete_type}\" ? data.link : null,\n\
        {}readerAst,\n\
        {}hasUpdatable: false,\n\
        }};\n\n\
        export default artifact;\n",
        "  ", "  ", "  ", "  ", "  ", "  ", "  "
    );

    ArtifactPathAndContent {
        file_name: *RESOLVER_READER_FILE_NAME,
        file_content: reader_content,
        type_and_field: Some(ObjectTypeAndFieldName {
            type_name: parent_object_entity.name,
            field_name: server_object_selectable_name.into(),
        }),
    }
}

pub(crate) fn generate_eager_reader_param_type_artifact<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    client_scalar_selectable: &ClientSelectable<TNetworkProtocol>,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let ts_file_extension = file_extensions.ts();
    let parent_type = schema
        .server_entity_data
        .server_object_entity(client_scalar_selectable.parent_object_entity_id());

    let mut param_type_imports = BTreeSet::new();
    let mut loadable_fields = BTreeSet::new();
    let mut link_fields = false;
    let mut updatable_fields = false;
    let client_field_parameter_type = generate_client_field_parameter_type(
        schema,
        client_scalar_selectable.selection_set_for_parent_query(),
        &mut param_type_imports,
        &mut loadable_fields,
        1,
        &mut link_fields,
    );
    let updatable_data_type = generate_client_field_updatable_data_type(
        schema,
        client_scalar_selectable.selection_set_for_parent_query(),
        &mut param_type_imports,
        &mut loadable_fields,
        1,
        &mut link_fields,
        &mut updatable_fields,
    );

    let param_type_import_statement =
        param_type_imports_to_import_statement(&param_type_imports, file_extensions);
    let reader_param_type = format!(
        "{}__{}__param",
        parent_type.name,
        client_scalar_selectable.name()
    );

    let link_field_imports = if link_fields {
        "import type { Link } from '@isograph/react';\n".to_string()
    } else {
        "".to_string()
    };

    let start_update_imports = if updatable_fields {
        "import type { StartUpdate } from '@isograph/react';\n".to_string()
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

    let (parameters_import, parameters_type) = if !client_scalar_selectable
        .variable_definitions()
        .is_empty()
    {
        let reader_parameters_type = format!(
            "{}__{}__parameters",
            parent_type.name,
            client_scalar_selectable.name()
        );
        (
            format!("import type {{ {reader_parameters_type} }} from './parameters_type{ts_file_extension}';\n"),
            reader_parameters_type,
        )
    } else {
        ("".to_string(), "Record<PropertyKey, never>".to_string())
    };

    let indent = "  ";
    let start_update_type = if updatable_fields {
        format!(
            "{}readonly startUpdate: StartUpdate<{}>,\n",
            indent, updatable_data_type
        )
    } else {
        "".to_string()
    };

    let param_type_content = format!(
        "{param_type_import_statement}\
        {link_field_imports}\
        {start_update_imports}\
        {loadable_field_imports}\
        {parameters_import}\n\
        export type {reader_param_type} = {{\n\
        {indent}readonly data: {client_field_parameter_type},\n\
        {indent}readonly parameters: {parameters_type},\n\
        {start_update_type}\
        }};\n",
    );
    ArtifactPathAndContent {
        file_name: *RESOLVER_PARAM_TYPE_FILE_NAME,
        file_content: param_type_content,
        type_and_field: Some(ObjectTypeAndFieldName {
            type_name: parent_type.name,
            field_name: client_scalar_selectable.name().into(),
        }),
    }
}

pub(crate) fn generate_eager_reader_output_type_artifact<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    client_field: &ClientSelectable<TNetworkProtocol>,
    config: &CompilerConfig,
    info: UserWrittenClientTypeInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let parent_type = schema
        .server_entity_data
        .server_object_entity(client_field.parent_object_entity_id());

    let function_import_statement =
        generate_function_import_statement(config, info, file_extensions);

    let client_field_output_type = match client_field {
        SelectionType::Object(_) => ClientFieldOutputType("Link".to_string()),
        SelectionType::Scalar(client_field) => generate_output_type(client_field),
    };

    let output_type_text = format!(
        "import type React from 'react';\n\
        {function_import_statement}\n\
        export type {}__{}__output_type = {};",
        parent_type.name,
        client_field.name(),
        client_field_output_type
    );

    let final_output_type_text = if let SelectionType::Object(_) = client_field {
        format!(
            "import type {{ Link }} \
                from '@isograph/react';\n\
                {output_type_text}\n",
        )
    } else if let ClientFieldDirectiveSet::None(_) = info.client_field_directive_set {
        output_type_text
    } else {
        format!(
            "import type {{ ExtractSecondParam, CombineWithIntrinsicAttributes }} \
                from '@isograph/react';\n\
                {output_type_text}\n",
        )
    };

    ArtifactPathAndContent {
        file_name: *RESOLVER_OUTPUT_TYPE_FILE_NAME,
        file_content: final_output_type_text,
        type_and_field: Some(ObjectTypeAndFieldName {
            type_name: parent_type.name,
            field_name: client_field.name().into(),
        }),
    }
}

/// Example: import { PetUpdater as resolver } from '../../../PetUpdater';
fn generate_function_import_statement(
    config: &CompilerConfig,
    target_field_info: UserWrittenClientTypeInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> ClientFieldFunctionImportStatement {
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
    let relative_path_to_current_artifact =
        PathBuf::from(config.artifact_directory.relative_path.lookup()).join("Type/Field");
    let relative_path_to_client_field = target_field_info.file_path.lookup();

    let relative_path = pathdiff::diff_paths(
        relative_path_to_client_field,
        relative_path_to_current_artifact,
    )
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

    let const_export_name = target_field_info.const_export_name;
    ClientFieldFunctionImportStatement(format!(
        "import {{ {const_export_name} as resolver }} from '{}';",
        file_name
    ))
}
