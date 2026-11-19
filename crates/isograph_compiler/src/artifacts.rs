use common_lang_types::{EntityName, EntityNameAndSelectableName, RelativePathToSourceFile};
use isograph_parser::{IsoLiteralItem, ParsedIsoLiteral};
use pico_macros::memo;
use prelude::Postfix;

use crate::{ClientId, CompilationProfile, IsographDb, parsed_iso_literals_in_file};

/// A path inside the `__isograph` folder, `/`-separated, with no `.` or `..` component.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactPath(pub String);

/// What one iso literal contributes to generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactSource {
    Entity(EntityName),
    Field(EntityNameAndSelectableName),
    Entrypoint(EntityNameAndSelectableName),
}

/// An iso literal with at least one parse error: its file and its index among that file's
/// literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "The iso literal at index {index} in {path} has a parse error, so no artifacts are written."
)]
pub struct LiteralParseFailed {
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

/// One entry per iso literal of `path` as `client` reads it, in source order. `None` when the
/// file has no text.
#[memo]
pub fn artifact_sources_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<Result<ArtifactSource, LiteralParseFailed>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .enumerate()
        .map(|(index, (_, parsed))| {
            artifact_source(parsed).ok_or(LiteralParseFailed { path, index })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

/// The source of a literal that parsed without errors.
fn artifact_source(parsed: &ParsedIsoLiteral) -> Option<ArtifactSource> {
    if !parsed.errors.is_empty() {
        return None;
    }
    match parsed.item.as_ref()?.item.item.as_ref()?.item.reference() {
        IsoLiteralItem::Entity(entity) => ArtifactSource::Entity(entity.name.item.0),
        IsoLiteralItem::Selectable(declaration) => {
            ArtifactSource::Field(EntityNameAndSelectableName {
                parent_entity_name: declaration.parent_type.item.0,
                selectable_name: declaration.name.item.0,
            })
        }
        IsoLiteralItem::Entrypoint(declaration) => {
            ArtifactSource::Entrypoint(EntityNameAndSelectableName {
                parent_entity_name: declaration.parent_type.item.0,
                selectable_name: declaration.name.item.0,
            })
        }
    }
    .wrap_some()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityNameAndSelectableName, RelativePathToSourceFile};
    use intern::string_key::Intern;
    use prelude::Postfix;

    use super::{ArtifactSource, LiteralParseFailed, artifact_sources_in_file};
    use crate::test_profile::TestProfile;
    use crate::{ClientId, FileMembership, IncludeFile, IsographDb};

    const CLIENT: ClientId = ClientId(1);

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };

    fn path() -> RelativePathToSourceFile {
        "src/a.ts".intern().to()
    }

    fn sources(
        db: &IsographDb<TestProfile>,
    ) -> Option<Vec<Result<ArtifactSource, LiteralParseFailed>>> {
        artifact_sources_in_file(db, CLIENT, path()).clone()
    }

    fn named(parent: &str, selectable: &str) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName {
            parent_entity_name: parent.intern().to(),
            selectable_name: selectable.intern().to(),
        }
    }

    #[test]
    fn a_field_an_entity_and_an_entrypoint_are_listed_in_source_order() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path(),
            SOURCE_FILE,
            "iso(`field Pet.fullName { firstName }`)\niso(`entity Owner`)\niso(`entrypoint Query.HomeRoute`)"
                .to_owned(),
        );
        assert_eq!(
            sources(db.reference()),
            vec![
                ArtifactSource::Field(named("Pet", "fullName")).wrap_ok(),
                ArtifactSource::Entity("Owner".intern().to()).wrap_ok(),
                ArtifactSource::Entrypoint(named("Query", "HomeRoute")).wrap_ok(),
            ]
            .wrap_some()
        );
    }

    #[test]
    fn a_literal_with_a_parse_error_fails_and_its_siblings_do_not() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path(),
            SOURCE_FILE,
            "iso(`entrypoint Query.A`)\niso(`entrypoint`)\niso(`entrypoint Query.B`)".to_owned(),
        );
        assert_eq!(
            sources(db.reference()),
            vec![
                ArtifactSource::Entrypoint(named("Query", "A")).wrap_ok(),
                LiteralParseFailed {
                    path: path(),
                    index: 1
                }
                .wrap_err(),
                ArtifactSource::Entrypoint(named("Query", "B")).wrap_ok(),
            ]
            .wrap_some()
        );
    }

    #[test]
    fn a_field_with_a_to_clause_is_a_field() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path(),
            SOURCE_FILE,
            "iso(`field Pet.bestFriend to Pet { id }`)".to_owned(),
        );
        assert_eq!(
            sources(db.reference()),
            ArtifactSource::Field(named("Pet", "bestFriend"))
                .wrap_ok()
                .wrap_vec()
                .wrap_some()
        );
    }

    #[test]
    fn a_file_with_no_literals_has_no_sources() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(path(), SOURCE_FILE, "export const a = 1;\n".to_owned());
        assert_eq!(sources(db.reference()), Vec::new().wrap_some());
    }

    #[test]
    fn a_path_with_no_text_is_none() {
        let mut db = IsographDb::<TestProfile>::default();
        // pico records no dependency on a never-written tracked counter; see refactors/pending/pico-missing-key-dependency.md.
        db.insert_disk_file("src/other.ts".intern().to(), SOURCE_FILE, String::new());
        assert_eq!(sources(db.reference()), None);
    }
}
