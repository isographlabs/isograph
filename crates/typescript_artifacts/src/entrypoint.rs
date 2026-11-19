use common_lang_types::EntityNameAndSelectableName;
use isograph_compiler::ArtifactPath;

use crate::module::{ArtifactImport, ArtifactModule, ArtifactObjectEntry, ArtifactValue};

pub(crate) const NORMALIZATION_AST: &str = "normalization_ast.ts";
pub(crate) const RESOLVER_READER: &str = "resolver_reader.ts";
pub(crate) const REFETCH_READER: &str = "refetch_reader.ts";

/// The entrypoint artifact of `entrypoint`, at the path the iso transforms import.
pub(crate) fn entrypoint_artifact(entrypoint: EntityNameAndSelectableName) -> ArtifactModule {
    ArtifactModule {
        path: entrypoint_path(entrypoint),
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
                key: "normalizationAst".to_owned(),
                value: ArtifactValue::Import(ArtifactImport(field_artifact_path(
                    entrypoint,
                    NORMALIZATION_AST,
                ))),
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

/// A module whose default export is an empty object, at `P/f/<file>`.
pub(crate) fn stub_artifact(field: EntityNameAndSelectableName, file: &str) -> ArtifactModule {
    ArtifactModule {
        path: field_artifact_path(field, file),
        default_export: ArtifactValue::Object(Vec::new()),
    }
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
