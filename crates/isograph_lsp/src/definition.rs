use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{ClientId, CompilationProfile, DefinitionLocation, IsographDb};
use prelude::Postfix;

use crate::entity_or_selectable::entity_or_selectable_of_node;
use crate::hover::resolved_position_at_lsp_position;

/// Where the schema item named at `position` in `path` is defined, as `client` reads it.
pub fn definition_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<DefinitionLocation> {
    let (_, node, _) = resolved_position_at_lsp_position(db, client, path, position)?;
    let item = entity_or_selectable_of_node(db, client, node.reference())?.ok()?;
    item.definition().wrap_some()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{
        DescriptionValue, Diagnostic, EntityName, RelativePathToSourceFile, SelectableName,
    };
    use intern::string_key::Intern;
    use isograph_compiler::{
        ClientId, DefinitionLocation, FileMembership, IncludeFile, IsographDb, entity_named,
        selectable_named, selectable_target,
    };
    use lsp_types::Position;
    use prelude::Postfix;
    use span::Span;

    use super::definition_at_lsp_position;
    use crate::hover_for_position;
    use crate::test_profile::TestProfile;

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };
    const NEITHER_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Skip,
        schema_file: IncludeFile::Skip,
    };

    const PET: &str = "iso(`entity Pet`)";
    const FULL_NAME: &str = "iso(`field Pet.fullName { id }`)";
    const OWNER: &str = "iso(`field Pet.owner to Pet { id }`)";

    fn client() -> ClientId {
        ClientId(1)
    }

    fn path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn entity(s: &str) -> EntityName {
        s.intern().to()
    }

    fn selectable(s: &str) -> SelectableName {
        s.intern().to()
    }

    /// A database with each `(path, contents)` on disk as a source file.
    fn db_with(files: &[(&str, &str)]) -> IsographDb<TestProfile> {
        let mut db = IsographDb::<TestProfile>::default();
        for (file, contents) in files {
            db.insert_disk_file(path(file), SOURCE_FILE, contents.dereference().to_owned());
        }
        db
    }

    /// The position of the first char of the first `needle` in the one-line `contents`.
    fn position_of(contents: &str, needle: &str) -> Position {
        Position::new(
            0,
            contents
                .find(needle)
                .expect("the fixture contains the needle") as u32,
        )
    }

    fn hover_text(db: &IsographDb<TestProfile>, file: &str, position: Position) -> Option<String> {
        hover_for_position(db, client(), path(file), position).map(|hover| match hover.contents {
            lsp_types::HoverContents::Markup(markup) => markup.value,
            contents => panic!("expected markup contents, got {contents:?}"),
        })
    }

    fn definition(
        db: &IsographDb<TestProfile>,
        file: &str,
        position: Position,
    ) -> Option<DefinitionLocation> {
        definition_at_lsp_position(db, client(), path(file), position)
    }

    /// The location of the first `needle` in `contents`, the text of `file`.
    fn located(file: &str, contents: &str, needle: &str) -> DefinitionLocation {
        let start = contents
            .find(needle)
            .expect("the fixture contains the needle");
        DefinitionLocation {
            path: path(file),
            span: Span::from_usize(start, start + needle.len()),
        }
    }

    fn error(message: &str) -> Diagnostic {
        Diagnostic::new(message.to_owned(), None)
    }

    #[test]
    fn a_selection_resolves_to_a_field_declared_in_another_file() {
        let selecting = "iso(`field Pet.x { fullName }`)";
        let db = db_with(&[
            ("src/Pet.ts", PET),
            ("src/fullName.ts", FULL_NAME),
            ("src/x.ts", selecting),
        ]);
        let at = position_of(selecting, "fullName");
        assert_eq!(
            hover_text(&db, "src/x.ts", at),
            "field **Pet.fullName**".to_owned().wrap_some()
        );
        assert_eq!(
            definition(&db, "src/x.ts", at),
            located("src/fullName.ts", FULL_NAME, "fullName").wrap_some()
        );
        let pet = position_of(selecting, "Pet");
        assert_eq!(
            hover_text(&db, "src/x.ts", pet),
            "entity **Pet**".to_owned().wrap_some()
        );
        assert_eq!(
            definition(&db, "src/x.ts", pet),
            located("src/Pet.ts", PET, "Pet").wrap_some()
        );
    }

    #[test]
    fn a_nested_selection_selects_on_the_target_of_its_parent() {
        let selecting = "iso(`field Pet.x { owner { fullName } }`)";
        let db = db_with(&[
            ("src/Pet.ts", PET),
            ("src/fullName.ts", FULL_NAME),
            ("src/owner.ts", OWNER),
            ("src/x.ts", selecting),
        ]);
        assert_eq!(
            hover_text(&db, "src/x.ts", position_of(selecting, "fullName")),
            "field **Pet.fullName**".to_owned().wrap_some()
        );
        assert_eq!(
            definition(&db, "src/x.ts", position_of(selecting, "fullName")),
            located("src/fullName.ts", FULL_NAME, "fullName").wrap_some()
        );
        assert_eq!(
            hover_text(&db, "src/x.ts", position_of(selecting, "owner")),
            "field **Pet.owner**: **Pet**".to_owned().wrap_some()
        );
        assert_eq!(
            definition(&db, "src/x.ts", position_of(selecting, "owner")),
            located("src/owner.ts", OWNER, "owner").wrap_some()
        );
    }

    fn owner_target(contents: &str) -> Option<EntityName> {
        let db = db_with(&[("src/owner.ts", contents)]);
        selectable_named(&db, client(), entity("Pet"), selectable("owner"))
            .clone()
            .expect("the fixture declares Pet.owner once")
            .target_entity
    }

    #[test]
    fn list_and_nullability_wrappers_do_not_change_the_target() {
        for contents in [
            "iso(`field Pet.owner to Pet { id }`)",
            "iso(`field Pet.owner to Pet! { id }`)",
            "iso(`field Pet.owner to [Pet!]! { id }`)",
            "iso(`field Pet.owner to [[Pet]] { id }`)",
        ] {
            assert_eq!(
                owner_target(contents),
                entity("Pet").wrap_some(),
                "{contents:?}"
            );
        }
    }

    #[test]
    fn a_field_without_to_or_with_a_list_of_no_type_has_no_target() {
        assert_eq!(owner_target("iso(`field Pet.owner { id }`)"), None);
        assert_eq!(owner_target("iso(`field Pet.owner to [!] { id }`)"), None);
    }

    #[test]
    fn the_description_of_a_declaration_is_the_selectables() {
        let db = db_with(&[(
            "src/owner.ts",
            "iso(`field Pet.owner \"The owner\" { id }`)",
        )]);
        assert_eq!(
            selectable_named(&db, client(), entity("Pet"), selectable("owner"))
                .clone()
                .map(|owner| owner.description),
            "The owner"
                .intern()
                .to::<DescriptionValue>()
                .wrap_some()
                .wrap_ok()
        );
    }

    #[test]
    fn a_selection_set_under_a_field_without_a_target_resolves_nothing() {
        let plain = "iso(`field Pet.plain { id }`)";
        let selecting = "iso(`field Pet.x { plain { id } }`)";
        let db = db_with(&[
            ("src/Pet.ts", PET),
            ("src/plain.ts", plain),
            ("src/x.ts", selecting),
        ]);
        let id = position_of(selecting, "id");
        assert_eq!(hover_text(&db, "src/x.ts", id), None);
        assert_eq!(definition(&db, "src/x.ts", id), None);
        assert_eq!(
            hover_text(&db, "src/x.ts", position_of(selecting, "plain")),
            "field **Pet.plain**".to_owned().wrap_some()
        );
        let plain_selectable = selectable_named(&db, client(), entity("Pet"), selectable("plain"))
            .clone()
            .expect("the fixture declares Pet.plain once");
        assert_eq!(
            selectable_target(&plain_selectable),
            error("Field `Pet.plain` has no target to select into.").wrap_err()
        );
    }

    #[test]
    fn a_duplicated_entity_does_not_resolve() {
        let selecting = "iso(`field Pet.x { id }`)";
        for files in [
            vec![
                ("src/Pet.ts", PET),
                ("src/Pet2.ts", PET),
                ("src/x.ts", selecting),
            ],
            vec![
                ("src/Pet.ts", "iso(`entity Pet`) iso(`entity Pet`)"),
                ("src/x.ts", selecting),
            ],
        ] {
            let db = db_with(&files);
            let pet = position_of(selecting, "Pet");
            assert_eq!(hover_text(&db, "src/x.ts", pet), None, "{files:?}");
            assert_eq!(definition(&db, "src/x.ts", pet), None, "{files:?}");
            assert_eq!(
                entity_named(&db, client(), entity("Pet")).clone(),
                error("Entity `Pet` is defined more than once.").wrap_err()
            );
        }
    }

    #[test]
    fn a_duplicated_field_does_not_resolve() {
        let db = db_with(&[("src/a.ts", FULL_NAME), ("src/b.ts", FULL_NAME)]);
        assert_eq!(
            selectable_named(&db, client(), entity("Pet"), selectable("fullName")).clone(),
            error("Field `Pet.fullName` is defined more than once.").wrap_err()
        );
    }

    #[test]
    fn missing_declarations_are_not_defined_errors() {
        let db = db_with(&[]);
        assert_eq!(
            entity_named(&db, client(), entity("Pet")).clone(),
            error("Entity `Pet` is not defined.").wrap_err()
        );
        assert_eq!(
            selectable_named(&db, client(), entity("Pet"), selectable("x")).clone(),
            error("Field `Pet.x` is not defined.").wrap_err()
        );
    }

    #[test]
    fn a_broken_literal_elsewhere_does_not_change_a_lookup() {
        let selecting = "iso(`field Pet.x { fullName }`)";
        let files = [
            ("src/Pet.ts", PET),
            ("src/fullName.ts", FULL_NAME),
            ("src/x.ts", selecting),
        ];
        let clean = db_with(&files);
        let broken = db_with(
            &[
                files.as_slice(),
                &[("src/broken.ts", "iso(`field Pet.y { ) }`)")],
            ]
            .concat(),
        );
        for needle in ["fullName", "Pet"] {
            let at = position_of(selecting, needle);
            assert!(hover_text(&clean, "src/x.ts", at).is_some());
            assert_eq!(
                hover_text(&broken, "src/x.ts", at),
                hover_text(&clean, "src/x.ts", at)
            );
            assert_eq!(
                definition(&broken, "src/x.ts", at),
                definition(&clean, "src/x.ts", at)
            );
        }
    }

    #[test]
    fn a_literal_with_parse_errors_contributes_what_it_parsed() {
        let broken = "iso(`field Pet.fullName { ) }`)";
        let db = db_with(&[("src/fullName.ts", broken)]);
        assert_eq!(
            selectable_named(&db, client(), entity("Pet"), selectable("fullName"))
                .clone()
                .map(|full_name| full_name.definition),
            located("src/fullName.ts", broken, "fullName").wrap_ok()
        );
    }

    #[test]
    fn a_clients_buffer_counts_for_that_client_only() {
        let mut db = db_with(&[("src/fullName.ts", "iso(`field Pet.other { id }`)")]);
        db.set_open_file(
            client(),
            path("src/fullName.ts"),
            SOURCE_FILE,
            FULL_NAME.to_owned(),
        );
        assert_eq!(
            selectable_named(&db, client(), entity("Pet"), selectable("fullName"))
                .clone()
                .map(|full_name| full_name.definition),
            located("src/fullName.ts", FULL_NAME, "fullName").wrap_ok()
        );
        assert_eq!(
            selectable_named(&db, ClientId(2), entity("Pet"), selectable("fullName")).clone(),
            error("Field `Pet.fullName` is not defined.").wrap_err()
        );
    }

    #[test]
    fn a_file_that_is_not_a_source_file_contributes_nothing() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(path("other/Pet.ts"), NEITHER_FILE, PET.to_owned());
        db.set_open_file(
            client(),
            path("other/Open.ts"),
            NEITHER_FILE,
            PET.to_owned(),
        );
        assert_eq!(
            entity_named(&db, client(), entity("Pet")).clone(),
            error("Entity `Pet` is not defined.").wrap_err()
        );
    }
}
