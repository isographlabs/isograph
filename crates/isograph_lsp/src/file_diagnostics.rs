use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{
    ClientId, CompilationProfile, IsographDb, iso_literal_parse_errors_in_file,
    iso_literal_validation_errors_in_file,
};
use pico_macros::memo;
use prelude::Postfix;
use span::Span;

use crate::line_index;

const DIAGNOSTIC_SOURCE: &str = "isograph";

/// `lsp_types::Diagnostic`s of `path` as `client` reads it.
#[memo]
pub fn lsp_diagnostics_for_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::Diagnostic>> {
    let parse_errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
    let validation_errors = iso_literal_validation_errors_in_file(db, client, path).as_ref()?;
    if parse_errors.is_empty() && validation_errors.is_empty() {
        return Vec::new().wrap_some();
    }
    let line_index = line_index(db, client, path).as_ref()?;
    let diagnostic = |location: Span, message: String| lsp_types::Diagnostic {
        range: line_index.range(location),
        severity: lsp_types::DiagnosticSeverity::ERROR.wrap_some(),
        source: DIAGNOSTIC_SOURCE.to_owned().wrap_some(),
        message,
        ..Default::default()
    };
    parse_errors
        .iter()
        .map(|error| diagnostic(error.location, error.item.to_string()))
        .chain(
            validation_errors
                .iter()
                .map(|error| diagnostic(error.location, error.item.to_string())),
        )
        .collect::<Vec<_>>()
        .wrap_some()
}

#[cfg(test)]
mod tests {
    use crate::test_profile::TestProfile;
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_compiler::{ClientId, IsographDb, iso_literal_parse_errors_in_file};
    use prelude::Postfix;

    use super::lsp_diagnostics_for_file;

    const SOURCE_FILE: isograph_compiler::FileMembership = isograph_compiler::FileMembership {
        source_file: isograph_compiler::IncludeFile::Include,
        schema_file: isograph_compiler::IncludeFile::Skip,
    };

    fn client() -> ClientId {
        ClientId(1)
    }

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn diagnostics_of(contents: &str) -> Vec<lsp_types::Diagnostic> {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, contents.to_owned());
        lsp_diagnostics_for_file(&db, client(), path)
            .clone()
            .expect("the test interned this path")
    }

    #[test]
    fn missing_file_has_no_diagnostics() {
        let db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        assert_eq!(lsp_diagnostics_for_file(&db, client(), path).as_ref(), None);
    }

    #[test]
    fn file_with_no_literals_has_empty_diagnostics() {
        assert_eq!(diagnostics_of("const a = 1;"), Vec::new());
    }

    #[test]
    fn each_diagnostic_is_an_error_from_isograph_with_the_parse_error_message() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "iso(`entrypoint`)".to_owned());
        let errors = iso_literal_parse_errors_in_file(&db, client(), path)
            .clone()
            .expect("the test interned this path");
        let diagnostics = lsp_diagnostics_for_file(&db, client(), path)
            .clone()
            .expect("the test interned this path");
        assert!(!errors.is_empty());
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
            errors
                .iter()
                .map(|error| error.item.to_string())
                .collect::<Vec<_>>()
        );
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.severity == lsp_types::DiagnosticSeverity::ERROR.wrap_some()
                && diagnostic.source == "isograph".to_owned().wrap_some()
                && diagnostic.code.is_none()
        }));
    }

    #[test]
    fn a_stray_close_parenthesis_is_reported_as_unexpected_close_parenthesis() {
        let diagnostics =
            diagnostics_of("iso(`field Pet.x { id ) }`) iso(`entity Pet`) iso(`field Pet.id`)");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            "Unexpected ')'".wrap_vec()
        );
        assert_eq!(
            diagnostics[0].range,
            lsp_types::Range::new(
                lsp_types::Position::new(0, 22),
                lsp_types::Position::new(0, 23)
            )
        );
    }

    #[test]
    fn crlf_and_lone_cr_before_the_literal_put_the_range_on_line_one() {
        for contents in [
            "const a = 1;\r\niso(`entrypoint`)",
            "const a = 1;\riso(`entrypoint`)",
        ] {
            let diagnostics = diagnostics_of(contents);
            assert!(!diagnostics.is_empty());
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.range.start.line == 1),
                "{contents:?}"
            );
        }
    }

    #[test]
    fn a_surrogate_pair_before_the_literal_adds_two_to_the_start_character() {
        let plain = diagnostics_of("iso(`entrypoint`)");
        let wide = diagnostics_of("😀iso(`entrypoint`)");
        assert!(!plain.is_empty());
        assert_eq!(
            wide.iter()
                .map(|diagnostic| diagnostic.range.start.character)
                .collect::<Vec<_>>(),
            plain
                .iter()
                .map(|diagnostic| diagnostic.range.start.character + 2)
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod validation_tests {
    use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
    use intern::string_key::Intern;
    use isograph_compiler::{
        ClientId, DuplicateDefinitionError, FileMembership, IncludeFile, IsographDb, SelectableKey,
        SelectionSetOnFieldWithoutTargetError, UnknownEntityError, UnknownSelectableError,
        ValidationError, iso_literal_validation_errors_in_file,
    };
    use prelude::Postfix;

    use super::lsp_diagnostics_for_file;
    use crate::test_profile::TestProfile;

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };

    const OWNER: &str = "iso(`entity Owner`) iso(`field Owner.displayName`)";

    fn client() -> ClientId {
        ClientId(1)
    }

    fn path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn entity(s: &str) -> EntityName {
        s.intern().to()
    }

    fn key(entity_name: &str, selectable: &str) -> SelectableKey {
        SelectableKey {
            entity: entity(entity_name),
            selectable: selectable.intern().to::<SelectableName>(),
        }
    }

    /// A database with each `(path, contents)` on disk as a source file.
    fn db_with(files: &[(&str, &str)]) -> IsographDb<TestProfile> {
        let mut db = IsographDb::<TestProfile>::default();
        for (file, contents) in files {
            db.insert_disk_file(path(file), SOURCE_FILE, contents.dereference().to_owned());
        }
        db
    }

    /// Each validation error of `src/a.ts` (with `OWNER` in `src/Owner.ts` and `others`
    /// beside it) and the text its span covers.
    fn errors_in(contents: &str, others: &[(&str, &str)]) -> Vec<(ValidationError, String)> {
        let db = db_with(
            &[
                [("src/Owner.ts", OWNER), ("src/a.ts", contents)].as_slice(),
                others,
            ]
            .concat(),
        );
        iso_literal_validation_errors_in_file(&db, client(), path("src/a.ts"))
            .clone()
            .expect("the test inserted src/a.ts")
            .into_iter()
            .map(|error| {
                (
                    error.item,
                    contents[error.location.as_usize_range()].to_owned(),
                )
            })
            .collect()
    }

    fn unknown_entity(name: &str) -> ValidationError {
        ValidationError::UnknownEntity(UnknownEntityError(entity(name)))
    }

    fn unknown_selectable(entity_name: &str, selectable: &str) -> ValidationError {
        ValidationError::UnknownSelectable(UnknownSelectableError(key(entity_name, selectable)))
    }

    #[test]
    fn an_undefined_parent_entity_is_one_error_on_its_name() {
        assert_eq!(
            errors_in("iso(`field Ownr.x { a { b }, c }`)", &[]),
            (unknown_entity("Ownr"), "Ownr".to_owned()).wrap_vec()
        );
    }

    #[test]
    fn an_undefined_selection_is_an_error_on_its_field_name_and_stops_its_branch() {
        assert_eq!(
            errors_in(
                "iso(`field Owner.x { nope { deeper }, displayName, nickname }`)",
                &[]
            ),
            vec![
                (unknown_selectable("Owner", "nope"), "nope".to_owned()),
                (
                    unknown_selectable("Owner", "nickname"),
                    "nickname".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn an_aliased_selection_is_checked_by_its_field_name() {
        assert_eq!(
            errors_in("iso(`field Owner.x { alias: displayName }`)", &[]),
            vec![]
        );
        assert_eq!(
            errors_in("iso(`field Owner.x { alias: nope }`)", &[]),
            (unknown_selectable("Owner", "nope"), "nope".to_owned()).wrap_vec()
        );
    }

    #[test]
    fn arguments_are_not_checked() {
        assert_eq!(
            errors_in(
                "iso(`field Owner.x { displayName(id: 1, other: $v) }`)",
                &[]
            ),
            vec![]
        );
    }

    #[test]
    fn a_selection_set_under_a_field_without_a_target_is_an_error_on_the_set() {
        assert_eq!(
            errors_in("iso(`field Owner.x { displayName { id } }`)", &[]),
            (
                ValidationError::SelectionSetOnFieldWithoutTarget(
                    SelectionSetOnFieldWithoutTargetError(key("Owner", "displayName"))
                ),
                "{ id }".to_owned()
            )
                .wrap_vec()
        );
    }

    #[test]
    fn a_selection_through_to_selects_on_the_target() {
        let pet = "iso(`entity Pet`) iso(`field Pet.owner to Owner! { id }`) iso(`field Pet.id`)";
        assert_eq!(
            errors_in(
                "iso(`field Pet.card { owner { displayName, nope } }`)",
                &[("src/Pet.ts", pet)]
            ),
            (unknown_selectable("Owner", "nope"), "nope".to_owned()).wrap_vec()
        );
    }

    #[test]
    fn nothing_is_checked_under_an_undefined_target() {
        let pet = "iso(`entity Pet`) iso(`field Pet.owner to Ownr { id }`) iso(`field Pet.id`)";
        assert_eq!(
            errors_in(
                "iso(`field Pet.card { owner { nope }, id }`)",
                &[("src/Pet.ts", pet)]
            ),
            vec![]
        );
    }

    #[test]
    fn a_duplicated_entity_is_an_error_on_every_definition() {
        let contents = "iso(`entity Owner`) iso(`entity Owner`)";
        let duplicate =
            ValidationError::DuplicateDefinition(DuplicateDefinitionError::Entity(entity("Owner")));
        assert_eq!(
            errors_in(contents, &[]),
            vec![
                (duplicate.clone(), "Owner".to_owned()),
                (duplicate, "Owner".to_owned()),
            ]
        );
        let db = db_with(&[("src/Owner.ts", OWNER), ("src/a.ts", contents)]);
        let owner_errors =
            iso_literal_validation_errors_in_file(&db, client(), path("src/Owner.ts"))
                .clone()
                .expect("the test inserted src/Owner.ts");
        assert_eq!(
            owner_errors
                .into_iter()
                .map(|error| error.item)
                .collect::<Vec<_>>(),
            ValidationError::DuplicateDefinition(DuplicateDefinitionError::Entity(entity("Owner")))
                .wrap_vec()
        );
    }

    #[test]
    fn a_duplicated_field_is_an_error_on_every_definition() {
        let duplicate = ValidationError::DuplicateDefinition(DuplicateDefinitionError::Selectable(
            key("Owner", "d"),
        ));
        assert_eq!(
            errors_in("iso(`field Owner.d`) iso(`field Owner.d`)", &[]),
            vec![
                (duplicate.clone(), "d".to_owned()),
                (duplicate, "d".to_owned()),
            ]
        );
    }

    #[test]
    fn an_entrypoint_checks_its_entity_then_its_field() {
        assert_eq!(
            errors_in("iso(`entrypoint Owner.missing`)", &[]),
            (unknown_selectable("Owner", "missing"), "missing".to_owned()).wrap_vec()
        );
        assert_eq!(
            errors_in("iso(`entrypoint Ownr.missing`)", &[]),
            (unknown_entity("Ownr"), "Ownr".to_owned()).wrap_vec()
        );
        assert_eq!(
            errors_in("iso(`entrypoint Owner.displayName`)", &[]),
            vec![]
        );
    }

    #[test]
    fn each_message_names_the_reference() {
        let messages: Vec<String> = errors_in(
            "iso(`field Ownr.a`) iso(`field Owner.b { nope, displayName { id } }`) iso(`entity Owner`)",
            &[],
        )
        .into_iter()
        .map(|(error, _)| error.to_string())
        .collect();
        assert_eq!(
            messages,
            vec![
                "Entity `Ownr` is not defined.",
                "Field `Owner.nope` is not defined.",
                "Field `Owner.displayName` has no target to select into.",
                "Entity `Owner` is defined more than once.",
            ]
        );
    }

    #[test]
    fn pet_demo_like_source_only_literals_have_no_diagnostics() {
        let files = [
            (
                "src/Owner.ts",
                "iso(`entity Owner`)\niso(`field Owner.displayName \"How the owner signs.\"`)\niso(`field Owner.OwnerBadge @component { displayName }`)",
            ),
            (
                "src/Pet.ts",
                "iso(`entity Pet`)\niso(`field Pet.id`)\niso(`field Pet.owner to Owner! { id }`)",
            ),
            (
                "src/Card.ts",
                "iso(`field Pet.card($id: ID!) @component { owner { displayName, OwnerBadge }, id }`)\niso(`entrypoint Pet.card`)",
            ),
        ];
        let db = db_with(&files);
        for (file, _) in files {
            assert_eq!(
                lsp_diagnostics_for_file(&db, client(), path(file)).clone(),
                Vec::new().wrap_some(),
                "{file}"
            );
        }
    }

    #[test]
    fn parse_errors_come_before_validation_errors() {
        let db = db_with(&[("src/a.ts", "iso(`field Ownr.x { ) }`)")]);
        let messages: Vec<String> = lsp_diagnostics_for_file(&db, client(), path("src/a.ts"))
            .clone()
            .expect("the test inserted src/a.ts")
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect();
        assert_eq!(
            messages,
            vec![
                "Unexpected ')'".to_owned(),
                "Entity `Ownr` is not defined.".to_owned(),
            ]
        );
    }
}
