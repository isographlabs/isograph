use common_lang_types::EntityNameAndSelectableName;
use isograph_compiler::ArtifactPath;

use crate::module::{ArtifactModule, ArtifactObjectEntry, ArtifactValue};

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
        ]),
    }
}

pub(crate) fn entrypoint_path(entrypoint: EntityNameAndSelectableName) -> ArtifactPath {
    ArtifactPath(format!(
        "{}/{}/entrypoint.ts",
        entrypoint.parent_entity_name, entrypoint.selectable_name
    ))
}
