use common_lang_types::{ArtifactFileName, EntityName, EntityNameAndSelectableName, QueryText};
use intern::Lookup;
use isograph_compiler::ArtifactPath;
use prelude::Postfix;

use crate::generate_types::{OUTPUT_TYPE, PARAM_TYPE, output_type_name, param_type_name};
use crate::imports::relative_artifact_specifier;
use crate::module::{
    ArtifactImport, ArtifactModule, ArtifactObjectEntry, ArtifactValue, DeclaredExportType,
    DeclaredTypeArgument, DefaultExportType, NamedImport,
};

/// The package the artifact types and the runtime live in.
pub(crate) const RUNTIME_PACKAGE: &str = "@isograph/runtime";
const ISOGRAPH_ENTRYPOINT: &str = "IsographEntrypoint";

pub(crate) const NORMALIZATION_AST: &str = "normalization_ast.ts";
pub(crate) const QUERY_TEXT: &str = "query_text.ts";
pub(crate) const RESOLVER_READER: &str = "resolver_reader.ts";

/// The entrypoint artifact of `entrypoint`, whose operation's root entity is `root_entity`,
/// at the path the iso transforms import, declared as an `IsographEntrypoint` of its param
/// type, output type, and root entity.
pub(crate) fn entrypoint_artifact(
    entrypoint: EntityNameAndSelectableName,
    root_entity: EntityName,
) -> ArtifactModule {
    let path = entrypoint_path(entrypoint);
    let type_import = |file: &str, name: String| NamedImport {
        specifier: relative_artifact_specifier(
            path.reference(),
            field_artifact_path(entrypoint, file).reference(),
        ),
        imported: name.clone(),
        local: name,
    };
    ArtifactModule {
        default_export_type: DefaultExportType::Declared(DeclaredExportType {
            binding: "artifact".to_owned(),
            type_import: NamedImport {
                specifier: RUNTIME_PACKAGE.to_owned(),
                imported: ISOGRAPH_ENTRYPOINT.to_owned(),
                local: ISOGRAPH_ENTRYPOINT.to_owned(),
            },
            type_arguments: vec![
                DeclaredTypeArgument::Import(type_import(PARAM_TYPE, param_type_name(entrypoint))),
                DeclaredTypeArgument::Import(type_import(
                    OUTPUT_TYPE,
                    output_type_name(entrypoint),
                )),
                DeclaredTypeArgument::StringLiteral(root_entity.to_string()),
            ],
        }),
        path: path.clone(),
        default_export: ArtifactValue::Object(vec![
            ArtifactObjectEntry {
                key: "kind".to_owned(),
                value: ArtifactValue::String("Entrypoint".to_owned()),
            },
            ArtifactObjectEntry {
                key: "name".to_owned(),
                value: ArtifactValue::String(format!(
                    "{}.{}",
                    entrypoint.parent_entity_name, entrypoint.selectable_name
                )),
            },
            ArtifactObjectEntry {
                key: "queryText".to_owned(),
                value: ArtifactValue::Import(ArtifactImport(field_artifact_path(
                    entrypoint, QUERY_TEXT,
                ))),
            },
            ArtifactObjectEntry {
                key: "normalizationAst".to_owned(),
                value: ArtifactValue::Import(ArtifactImport(normalization_ast_path(entrypoint))),
            },
            ArtifactObjectEntry {
                key: "readerArtifact".to_owned(),
                value: ArtifactValue::Import(ArtifactImport(field_artifact_path(
                    entrypoint,
                    RESOLVER_READER,
                ))),
            },
        ]),
    }
}

/// The module whose default export is the text of `entrypoint`'s query.
pub(crate) fn query_text_artifact(
    entrypoint: EntityNameAndSelectableName,
    text: &QueryText,
) -> ArtifactModule {
    ArtifactModule {
        path: field_artifact_path(entrypoint, QUERY_TEXT),
        default_export: ArtifactValue::String(text.0.clone()),
        default_export_type: DefaultExportType::Inferred,
    }
}

/// The protocol's file holding exactly the text of `entrypoint`'s query, and one newline.
pub(crate) fn raw_query_text_file(
    entrypoint: EntityNameAndSelectableName,
    file_name: ArtifactFileName,
    text: &QueryText,
) -> (ArtifactPath, String) {
    (
        field_artifact_path(entrypoint, file_name.lookup()),
        format!("{}\n", text.0),
    )
}

/// `<Entity>/<selectable>/normalization_ast.ts`.
pub(crate) fn normalization_ast_path(entrypoint: EntityNameAndSelectableName) -> ArtifactPath {
    field_artifact_path(entrypoint, NORMALIZATION_AST)
}

pub(crate) fn entrypoint_path(entrypoint: EntityNameAndSelectableName) -> ArtifactPath {
    field_artifact_path(entrypoint, "entrypoint.ts")
}

/// `<parent>/<field>/<file>`.
pub(crate) fn field_artifact_path(field: EntityNameAndSelectableName, file: &str) -> ArtifactPath {
    ArtifactPath(format!(
        "{}/{}/{file}",
        field.parent_entity_name, field.selectable_name
    ))
}
