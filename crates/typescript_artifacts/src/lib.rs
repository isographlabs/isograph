//! TypeScript artifacts: the values generators build, and their printing as TypeScript.

mod entrypoint;
mod imports;
mod iso_overloads;
mod module;
mod swc;

use std::collections::BTreeSet;

use isograph_compiler::{ArtifactPath, ArtifactSource};
use prelude::Postfix;

pub use module::{ArtifactImport, ArtifactModule, ArtifactObjectEntry, ArtifactValue};

/// The path and text of every artifact of a project whose iso literals are `sources`: one
/// entrypoint artifact per entrypoint, then `iso.ts`.
pub fn generate_artifacts(sources: &BTreeSet<ArtifactSource>) -> Vec<(ArtifactPath, String)> {
    sources
        .iter()
        .filter_map(|source| match source {
            ArtifactSource::Entrypoint(entrypoint) => entrypoint.wrap_some(),
            ArtifactSource::Entity(_) | ArtifactSource::Field(_) => None,
        })
        .map(|entrypoint| {
            let module = entrypoint::entrypoint_artifact(entrypoint.dereference());
            let text = swc::print(module::module_of_artifact(module.reference()).reference());
            (module.path, text)
        })
        .chain(std::iter::once({
            let module = iso_overloads::iso_overload_module(sources);
            let text = swc::print(module.reference());
            (module.path, text)
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use common_lang_types::EntityNameAndSelectableName;
    use intern::string_key::Intern;
    use isograph_compiler::{ArtifactPath, ArtifactSource};
    use prelude::Postfix;

    use super::generate_artifacts;

    fn named(parent: &str, selectable: &str) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName {
            parent_entity_name: parent.intern().to(),
            selectable_name: selectable.intern().to(),
        }
    }

    fn artifacts(sources: &[ArtifactSource]) -> Vec<(ArtifactPath, String)> {
        generate_artifacts(sources.iter().copied().collect::<BTreeSet<_>>().reference())
    }

    fn text_at<'a>(artifacts: &'a [(ArtifactPath, String)], path: &str) -> &'a str {
        artifacts
            .iter()
            .find(|(artifact_path, _)| artifact_path.0 == path)
            .map(|(_, text)| text.as_str())
            .expect("the test generated this artifact")
    }

    /// The `iso` overload line whose matched string is `text`.
    fn overload_line<'a>(iso: &'a str, text: &str) -> &'a str {
        let needle = format!("MatchesWhitespaceAndString<\"{text}\", T>");
        iso.lines()
            .find(|line| line.contains(needle.as_str()))
            .expect("the test generated this overload")
    }

    fn position(iso: &str, text: &str) -> usize {
        iso.find(format!("<\"{text}\", T>").as_str())
            .expect("the test generated this overload")
    }

    #[test]
    fn an_entrypoint_artifact_exports_its_kind_and_name() {
        let artifacts = artifacts(&[ArtifactSource::Entrypoint(named("Query", "HomeRoute"))]);
        let text = text_at(artifacts.as_slice(), "Query/HomeRoute/entrypoint.ts");
        assert!(text.starts_with("export default {"), "{text}");
        assert!(text.contains("kind: \"Entrypoint\""), "{text}");
        assert!(text.contains("name: \"Query.HomeRoute\""), "{text}");
    }

    #[test]
    fn iso_imports_the_entrypoint_type_and_its_overload_returns_it() {
        let artifacts = artifacts(&[ArtifactSource::Entrypoint(named("Query", "HomeRoute"))]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(
            iso.contains(
                "import type Query__HomeRoute__entrypoint from \"./Query/HomeRoute/entrypoint\";"
            ),
            "{iso}"
        );
        assert!(
            overload_line(iso, "entrypoint Query.HomeRoute")
                .ends_with("): typeof Query__HomeRoute__entrypoint;"),
            "{iso}"
        );
    }

    #[test]
    fn a_field_overload_returns_identity_and_imports_nothing() {
        let artifacts = artifacts(&[ArtifactSource::Field(named("Pet", "fullName"))]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(
            overload_line(iso, "field Pet.fullName").ends_with("): Identity;"),
            "{iso}"
        );
        assert!(!iso.contains("import"), "{iso}");
    }

    #[test]
    fn an_entity_overload_returns_void() {
        let artifacts = artifacts(&[ArtifactSource::Entity("Owner".intern().to())]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(
            overload_line(iso, "entity Owner").ends_with("): void;"),
            "{iso}"
        );
    }

    #[test]
    fn a_longer_field_name_comes_before_its_prefix() {
        let artifacts = artifacts(&[
            ArtifactSource::Field(named("Pet", "Avatar")),
            ArtifactSource::Field(named("Pet", "AvatarCard")),
        ]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(position(iso, "field Pet.AvatarCard") < position(iso, "field Pet.Avatar"));
    }

    #[test]
    fn fields_precede_entities_which_precede_entrypoints() {
        let artifacts = artifacts(&[
            ArtifactSource::Entrypoint(named("A", "a")),
            ArtifactSource::Entity("Z".intern().to()),
            ArtifactSource::Field(named("Z", "z")),
        ]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(position(iso, "field Z.z") < position(iso, "entity Z"));
        assert!(position(iso, "entity Z") < position(iso, "entrypoint A.a"));
    }

    #[test]
    fn no_sources_is_only_iso_ts_without_imports() {
        let artifacts = artifacts(&[]);
        assert_eq!(
            artifacts
                .iter()
                .map(|(path, _)| path.0.as_str())
                .collect::<Vec<_>>(),
            ["iso.ts"]
        );
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(!iso.contains("import"), "{iso}");
        assert!(iso.ends_with("}\n"), "{iso}");
    }

    #[test]
    fn every_iso_import_names_a_generated_artifact() {
        let artifacts = artifacts(&[
            ArtifactSource::Entrypoint(named("Query", "HomeRoute")),
            ArtifactSource::Entrypoint(named("Mutation", "SetTagline")),
        ]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        let specifiers: Vec<&str> = iso
            .lines()
            .filter(|line| line.starts_with("import type "))
            .filter_map(|line| line.split('"').nth(1))
            .collect();
        assert_eq!(specifiers.len(), 2, "{iso}");
        assert!(
            specifiers.iter().all(|specifier| {
                specifier.strip_prefix("./").is_some_and(|relative| {
                    let generated = format!("{relative}.ts");
                    artifacts.iter().any(|(path, _)| path.0 == generated)
                })
            }),
            "{iso}"
        );
    }

    #[test]
    fn no_line_of_iso_ts_ends_in_whitespace() {
        let artifacts = artifacts(&[
            ArtifactSource::Field(named("Pet", "fullName")),
            ArtifactSource::Entrypoint(named("Query", "HomeRoute")),
        ]);
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(iso.lines().all(|line| line.trim_end() == line), "{iso}");
    }
}
