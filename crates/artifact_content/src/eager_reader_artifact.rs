use common_lang_types::{
    ArtifactPath, ArtifactPathAndContent, EntityNameAndSelectableName, WithEmbeddedLocation,
};
use intern::Lookup;
use isograph_config::{CompilerConfig, GenerateFileExtensionsOption};
use isograph_lang_types::{
    ClientScalarSelectableDirectiveSet, SelectionSet, SelectionType, SelectionTypePostfix,
    VariableDeclaration,
};
use isograph_schema::{
    ClientScalarSelectable, ClientSelectable, CompilationProfile, IsographDatabase,
    LINK_FIELD_NAME, MemoRefClientSelectable, ServerSelectable,
    client_scalar_selectable_selection_set_for_parent_query, initial_variable_context,
    selectable_reader_selection_set, server_entity_named,
};
use isograph_schema::{RefetchedPathsMap, UserWrittenClientTypeInfo};
use prelude::Postfix;
use std::{borrow::Cow, collections::BTreeSet, path::PathBuf};

use crate::format_parameter_type::format_parameter_type;
use crate::generate_updatable_and_parameter_type::{
    generate_client_selectable_parameter_type, generate_client_selectable_updatable_data_type,
};
use crate::{
    generate_artifacts::{
        ClientScalarSelectableFunctionImportStatement, ClientScalarSelectableOutputType,
        RESOLVER_OUTPUT_TYPE, RESOLVER_OUTPUT_TYPE_FILE_NAME, RESOLVER_PARAM_TYPE,
        RESOLVER_PARAM_TYPE_FILE_NAME, RESOLVER_PARAMETERS_TYPE_FILE_NAME,
        RESOLVER_READER_FILE_NAME, generate_output_type,
    },
    import_statements::{
        param_type_imports_to_import_param_statement, param_type_imports_to_import_statement,
        reader_imports_to_import_statement,
    },
    reader_ast::generate_reader_ast,
};

pub(crate) fn generate_eager_reader_artifacts<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_selectable: &ClientSelectable<TCompilationProfile>,
    config: &CompilerConfig,
    info: &UserWrittenClientTypeInfo,
    refetched_paths: &RefetchedPathsMap,
    file_extensions: GenerateFileExtensionsOption,
    has_updatable: bool,
) -> Vec<ArtifactPathAndContent> {
    let ts_file_extension = file_extensions.ts();
    let user_written_component_variant = info.client_scalar_selectable_directive_set.clone();

    let parent_entity_name = match client_selectable {
        SelectionType::Scalar(s) => s.parent_entity_name,
        SelectionType::Object(o) => o.parent_entity_name,
    };

    let parent_object_entity = &server_entity_named(db, parent_entity_name)
        .as_ref()
        .expect(
            "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let (reader_ast, reader_imports) = generate_reader_ast(
        db,
        parent_entity_name,
        &match client_selectable {
            SelectionType::Scalar(scalar) => {
                client_scalar_selectable_selection_set_for_parent_query(
                    db,
                    scalar.parent_entity_name,
                    scalar.name,
                )
                .expect("Expected selection set to exist and to be valid.")
            }
            SelectionType::Object(object) => {
                selectable_reader_selection_set(db, object.parent_entity_name, object.name)
                    .expect("Expected selection set to exist and to be valid.")
                    .lookup(db)
                    .clone()
                    .note_todo("Don't clone")
            }
        },
        0,
        refetched_paths,
        &initial_variable_context(client_selectable),
    );

    let function_import_statement =
        generate_function_import_statement(config, info, file_extensions);

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

    let client_selectable_name = match client_selectable {
        SelectionType::Object(o) => o.name,
        SelectionType::Scalar(s) => s.name,
    };
    let reader_param_type = format!(
        "{}__{}__param",
        parent_object_entity.name, client_selectable_name
    );

    let reader_content = if let ClientScalarSelectableDirectiveSet::None(_) =
        user_written_component_variant.expect(
            "Expected component variant to be valid. \
            This is indicative of a bug in Isograph.",
        ) {
        let reader_output_type = format!(
            "{}__{}__output_type",
            parent_object_entity.name, client_selectable_name
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
            {}fieldName: \"{client_selectable_name}\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            {}hasUpdatable: {has_updatable},\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ", "  ",
        )
    } else {
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
            {}fieldName: \"{client_selectable_name}\",\n\
            {}resolver,\n\
            {}readerAst,\n\
            {}hasUpdatable: {has_updatable},\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ", "  "
        )
    };

    let mut path_and_contents = vec![ArtifactPathAndContent {
        file_content: reader_content.into(),
        artifact_path: ArtifactPath {
            file_name: *RESOLVER_READER_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: parent_object_entity.name.item,
                selectable_name: client_selectable_name,
            }
            .wrap_some(),
        },
    }];

    let variable_definitions = match client_selectable {
        SelectionType::Scalar(s) => s.variable_definitions.reference(),
        SelectionType::Object(o) => o.variable_definitions.reference(),
    };
    if !variable_definitions.is_empty() {
        let reader_parameters_type = format!(
            "{}__{}__parameters",
            parent_object_entity.name, client_selectable_name
        );
        let parameters = variable_definitions.iter();
        let parameters_types = generate_parameters(db, parameters);
        let parameters_content =
            format!("export type {reader_parameters_type} = {parameters_types}\n");
        path_and_contents.push(ArtifactPathAndContent {
            file_content: parameters_content.into(),
            artifact_path: ArtifactPath {
                file_name: *RESOLVER_PARAMETERS_TYPE_FILE_NAME,
                type_and_field: EntityNameAndSelectableName {
                    parent_entity_name: parent_object_entity.name.item,
                    selectable_name: client_selectable_name,
                }
                .wrap_some(),
            },
        });
    }

    path_and_contents
}

pub(crate) fn generate_eager_reader_condition_artifact<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    server_object_selectable: &ServerSelectable<TCompilationProfile>,
    inline_fragment_reader_selections: &WithEmbeddedLocation<SelectionSet>,
    refetch_paths: &RefetchedPathsMap,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let server_object_selectable_name = server_object_selectable.name;

    let parent_object_entity =
        &server_entity_named(db, server_object_selectable.parent_entity_name)
            .as_ref()
            .expect(
                "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db);

    let concrete_type = &server_entity_named(
        db,
        server_object_selectable
            .target_entity
            .as_ref()
            .expect("Expected target entity to be valid.")
            .inner()
            .0,
    )
    .as_ref()
    .expect(
        "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect(
        "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
    )
    .lookup(db)
    .name;

    let (reader_ast, reader_imports) = generate_reader_ast(
        db,
        server_object_selectable.parent_entity_name,
        inline_fragment_reader_selections,
        0,
        refetch_paths,
        &server_object_selectable.initial_variable_context(),
    );

    let reader_import_statement =
        reader_imports_to_import_statement(&reader_imports, file_extensions);

    let reader_param_type = "{ data: any, parameters: Record<PropertyKey, never> }";

    let reader_output_type = format!("Link<\"{}\"> | null", concrete_type);

    let eager_reader_name = server_object_selectable_name;

    let link_field_name = *LINK_FIELD_NAME;
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
        {}resolver: ({{ data }}) => data.__typename === \"{concrete_type}\" ? data.{link_field_name} : null,\n\
        {}readerAst,\n\
        {}hasUpdatable: false,\n\
        }};\n\n\
        export default artifact;\n",
        "  ", "  ", "  ", "  ", "  ", "  ", "  "
    );

    ArtifactPathAndContent {
        file_content: reader_content.into(),
        artifact_path: ArtifactPath {
            file_name: *RESOLVER_READER_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: parent_object_entity.name.item,
                selectable_name: server_object_selectable_name,
            }
            .wrap_some(),
        },
    }
}

pub(crate) fn generate_eager_reader_param_type_artifact<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_selectable: MemoRefClientSelectable<TCompilationProfile>,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let client_selectable = match client_selectable {
        SelectionType::Scalar(s) => s.lookup(db).scalar_selected(),
        SelectionType::Object(o) => o.lookup(db).object_selected(),
    };

    let ts_file_extension = file_extensions.ts();
    let parent_entity_name = match client_selectable {
        SelectionType::Object(o) => o.parent_entity_name,
        SelectionType::Scalar(s) => s.parent_entity_name,
    };
    let parent_object_entity = &server_entity_named(db, parent_entity_name)
        .as_ref()
        .expect(
            "Expected validation to have worked. \
            This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let mut param_type_imports = BTreeSet::new();
    let mut loadable_fields = BTreeSet::new();
    let mut updatable_fields = false;
    let selection_set_for_parent_query = match client_selectable {
        SelectionType::Scalar(scalar) => client_scalar_selectable_selection_set_for_parent_query(
            db,
            scalar.parent_entity_name,
            scalar.name,
        )
        .expect("Expected selection set to be valid."),
        SelectionType::Object(object) => {
            let parent_object_entity_name = object.parent_entity_name;
            let client_object_selectable_name = object.name;
            selectable_reader_selection_set(
                db,
                parent_object_entity_name,
                client_object_selectable_name,
            )
            .expect("Expected selection set to be valid.")
            .lookup(db)
            .clone()
            .note_todo("Don't clone")
        }
    };
    let client_scalar_selectable_parameter_type = generate_client_selectable_parameter_type(
        db,
        parent_entity_name,
        &selection_set_for_parent_query,
        &mut param_type_imports,
        &mut loadable_fields,
        1,
    );
    let updatable_data_type = generate_client_selectable_updatable_data_type(
        db,
        parent_entity_name,
        &selection_set_for_parent_query,
        &mut param_type_imports,
        &mut loadable_fields,
        1,
        &mut updatable_fields,
    );

    let param_type_import_statement =
        param_type_imports_to_import_statement(&param_type_imports, file_extensions);
    let reader_param_type = format!(
        "{}__{}__param",
        parent_object_entity.name,
        match client_selectable {
            SelectionType::Scalar(s) => s.name,
            SelectionType::Object(o) => o.name,
        }
    );

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

    let variable_definitions = match client_selectable {
        SelectionType::Scalar(s) => s.variable_definitions.reference(),
        SelectionType::Object(o) => o.variable_definitions.reference(),
    };
    let (parameters_import, parameters_type) = if !variable_definitions.is_empty() {
        let reader_parameters_type = format!(
            "{}__{}__parameters",
            parent_object_entity.name,
            match client_selectable {
                SelectionType::Scalar(s) => s.name,
                SelectionType::Object(o) => o.name,
            }
        );
        (
            format!(
                "import type {{ {reader_parameters_type} }} from './parameters_type{ts_file_extension}';\n"
            ),
            reader_parameters_type,
        )
    } else {
        ("".to_string(), "Record<PropertyKey, never>".to_string())
    };

    let indent = "  ";
    let start_update_type = if updatable_fields {
        format!("{indent}readonly startUpdate: StartUpdate<{updatable_data_type}>,\n")
    } else {
        "".to_string()
    };

    let param_type_content = format!(
        "{param_type_import_statement}\
        {start_update_imports}\
        {loadable_field_imports}\
        {parameters_import}\n\
        export type {reader_param_type} = {{\n\
        {indent}readonly data: {client_scalar_selectable_parameter_type},\n\
        {indent}readonly parameters: {parameters_type},\n\
        {start_update_type}\
        }};\n",
    );
    let client_selectable_name = match client_selectable {
        SelectionType::Object(o) => o.name,
        SelectionType::Scalar(s) => s.name,
    };

    ArtifactPathAndContent {
        file_content: param_type_content.into(),
        artifact_path: ArtifactPath {
            file_name: *RESOLVER_PARAM_TYPE_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: parent_object_entity.name.item,
                selectable_name: client_selectable_name,
            }
            .wrap_some(),
        },
    }
}

pub(crate) fn generate_eager_reader_output_type_artifact<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    client_selectable: &ClientSelectable<TCompilationProfile>,
    config: &CompilerConfig,
    info: &UserWrittenClientTypeInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> ArtifactPathAndContent {
    let parent_entity_name = match client_selectable {
        SelectionType::Scalar(s) => s.parent_entity_name,
        SelectionType::Object(o) => o.parent_entity_name,
    };
    let parent_object_entity = &server_entity_named(db, parent_entity_name)
        .as_ref()
        .expect(
            "Expected validation to have worked. \
            This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let function_import_statement =
        generate_function_import_statement(config, info, file_extensions);

    let client_scalar_selectable_output_type = match client_selectable {
        SelectionType::Object(_) => {
            ClientScalarSelectableOutputType("ReturnType<typeof resolver>".to_string())
        }
        SelectionType::Scalar(client_scalar_selectable) => {
            generate_output_type(db, client_scalar_selectable)
        }
    };

    let client_selectable_name = match client_selectable {
        SelectionType::Object(o) => o.name,
        SelectionType::Scalar(s) => s.name,
    };

    let output_type_text = format!(
        "import type React from 'react';\n\
        {function_import_statement}\n\
        export type {}__{}__output_type = {};",
        parent_object_entity.name, client_selectable_name, client_scalar_selectable_output_type
    );

    let final_output_type_text = if let ClientScalarSelectableDirectiveSet::None(_) =
        info.client_scalar_selectable_directive_set.clone().expect(
            "Expected directive set to have been validated. \
            This is indicative of a bug in Isograph.",
        ) {
        output_type_text
    } else {
        format!(
            "import type {{ ExtractSecondParam, CombineWithIntrinsicAttributes }} \
                from '@isograph/react';\n\
                {output_type_text}\n",
        )
    };

    ArtifactPathAndContent {
        file_content: final_output_type_text.into(),
        artifact_path: ArtifactPath {
            file_name: *RESOLVER_OUTPUT_TYPE_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: parent_object_entity.name.item,
                selectable_name: client_selectable_name,
            }
            .wrap_some(),
        },
    }
}

pub(crate) fn generate_link_output_type_artifact<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
) -> ArtifactPathAndContent {
    let parent_object_entity =
        &server_entity_named(db, client_scalar_selectable.parent_entity_name)
            .as_ref()
            .expect(
                "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db);

    let client_scalar_selectable_output_type = generate_output_type(db, client_scalar_selectable);

    let output_type_text = format!(
        "import type {{ Link }} from '@isograph/react';\n\
        export type {}__{}__output_type = {};",
        parent_object_entity.name,
        client_scalar_selectable.name,
        client_scalar_selectable_output_type
    );

    ArtifactPathAndContent {
        file_content: output_type_text.into(),
        artifact_path: ArtifactPath {
            file_name: *RESOLVER_OUTPUT_TYPE_FILE_NAME,
            type_and_field: EntityNameAndSelectableName {
                parent_entity_name: parent_object_entity.name.item,
                selectable_name: client_scalar_selectable.name,
            }
            .wrap_some(),
        },
    }
}

/// Example: import { PetUpdater as resolver } from '../../../PetUpdater';
fn generate_function_import_statement(
    config: &CompilerConfig,
    target_field_info: &UserWrittenClientTypeInfo,
    file_extensions: GenerateFileExtensionsOption,
) -> ClientScalarSelectableFunctionImportStatement {
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
    let relative_path_to_client_scalar_selectable = target_field_info.file_path.lookup();

    let relative_path = pathdiff::diff_paths(
        relative_path_to_client_scalar_selectable,
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
    ClientScalarSelectableFunctionImportStatement(format!(
        "import {{ {const_export_name} as resolver }} from '{file_name}';"
    ))
}

fn generate_parameters<'a, TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    argument_definitions: impl Iterator<Item = &'a VariableDeclaration>,
) -> String {
    let mut s = "{\n".to_string();
    let indent = "  ";
    for arg in argument_definitions {
        let is_optional = arg.type_.item.is_nullable();
        s.push_str(&format!(
            "{indent}readonly {}{}: {},\n",
            arg.name.item,
            if is_optional { "?" } else { "" },
            format_parameter_type(db, arg.type_.item.reference(), 1)
        ));
    }
    s.push_str("};");
    s
}
