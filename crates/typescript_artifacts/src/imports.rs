use std::collections::BTreeSet;

use isograph_compiler::ArtifactPath;
use prelude::Postfix;

use crate::module::{ArtifactImport, ArtifactValue, NamedImport};

/// Every `Import` inside `value`, at any depth.
pub(crate) fn imports_of(value: &ArtifactValue) -> BTreeSet<&ArtifactImport> {
    match value {
        ArtifactValue::Null
        | ArtifactValue::Boolean(_)
        | ArtifactValue::Number(_)
        | ArtifactValue::String(_)
        | ArtifactValue::NamedImport(_) => BTreeSet::new(),
        ArtifactValue::Array(items) => items.iter().flat_map(imports_of).collect(),
        ArtifactValue::Object(entries) => entries
            .iter()
            .flat_map(|entry| imports_of(&entry.value))
            .collect(),
        ArtifactValue::Import(import) => std::iter::once(import).collect(),
    }
}

/// Every `NamedImport` inside `value`, at any depth.
pub(crate) fn named_imports_of(value: &ArtifactValue) -> BTreeSet<&NamedImport> {
    match value {
        ArtifactValue::Null
        | ArtifactValue::Boolean(_)
        | ArtifactValue::Number(_)
        | ArtifactValue::String(_)
        | ArtifactValue::Import(_) => BTreeSet::new(),
        ArtifactValue::Array(items) => items.iter().flat_map(named_imports_of).collect(),
        ArtifactValue::Object(entries) => entries
            .iter()
            .flat_map(|entry| named_imports_of(&entry.value))
            .collect(),
        ArtifactValue::NamedImport(import) => std::iter::once(import).collect(),
    }
}

/// The local name of `import`: its path's segments joined by `__`, `.ts` dropped
/// (`Query/HomeRoute/entrypoint.ts` is `Query__HomeRoute__entrypoint`).
pub(crate) fn artifact_import_identifier(import: &ArtifactImport) -> String {
    without_extension(import.0.reference())
        .split('/')
        .collect::<Vec<_>>()
        .join("__")
}

/// The specifier that imports `import` from the module at `from`: the import's path relative
/// to `from`'s folder, `./`-prefixed, `.ts` dropped.
pub(crate) fn artifact_import_specifier(from: &ArtifactPath, import: &ArtifactImport) -> String {
    relative_artifact_specifier(from, import.0.reference())
}

/// The specifier that imports the artifact at `to` from the artifact at `from`: `to` relative
/// to `from`'s folder, `./`-prefixed, `.ts` dropped.
pub(crate) fn relative_artifact_specifier(from: &ArtifactPath, to: &ArtifactPath) -> String {
    let from_folder: Vec<&str> = folder_segments(from);
    let target: Vec<&str> = without_extension(to).split('/').collect();
    let shared = from_folder
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = std::iter::repeat_n("..", from_folder.len() - shared);
    let rest = target[shared..].iter().copied();
    std::iter::once(".")
        .chain(ups)
        .chain(rest)
        .collect::<Vec<_>>()
        .join("/")
}

fn folder_segments(path: &ArtifactPath) -> Vec<&str> {
    let mut segments: Vec<&str> = path.0.split('/').collect();
    segments.pop();
    segments
}

fn without_extension(path: &ArtifactPath) -> &str {
    path.0.strip_suffix(".ts").unwrap_or(path.0.as_str())
}

#[cfg(test)]
mod tests {
    use isograph_compiler::ArtifactPath;

    use super::{artifact_import_identifier, artifact_import_specifier};
    use crate::module::ArtifactImport;

    fn path(text: &str) -> ArtifactPath {
        ArtifactPath(text.to_owned())
    }

    fn specifier(from: &str, to: &str) -> String {
        artifact_import_specifier(&path(from), &ArtifactImport(path(to)))
    }

    #[test]
    fn an_import_in_the_same_folder_is_dot_slash_its_name() {
        assert_eq!(
            specifier("Query/A/reader.ts", "Query/A/entrypoint.ts"),
            "./entrypoint"
        );
    }

    #[test]
    fn an_import_from_the_top_names_the_folders() {
        assert_eq!(
            specifier("iso.ts", "Query/A/entrypoint.ts"),
            "./Query/A/entrypoint"
        );
    }

    #[test]
    fn an_import_from_another_folder_climbs_to_the_shared_folder() {
        assert_eq!(specifier("Query/A/x.ts", "Pet/B/y.ts"), "./../../Pet/B/y");
        assert_eq!(specifier("Query/A/x.ts", "Query/B/y.ts"), "./../B/y");
    }

    #[test]
    fn the_identifier_joins_the_segments_without_the_extension() {
        assert_eq!(
            artifact_import_identifier(&ArtifactImport(path("Query/HomeRoute/entrypoint.ts"))),
            "Query__HomeRoute__entrypoint"
        );
    }
}
