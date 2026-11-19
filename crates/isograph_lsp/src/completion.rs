use std::collections::BTreeSet;

use common_lang_types::{RelativePathToSourceFile, SelectableName};
use isograph_compiler::{
    ClientId, CompilationProfile, IsographDb, LiteralId, Selectable, entity_for_selection_set_path,
    extract_iso_literals, parsed_iso_literal_by_id, selectables_for_entity,
};
use isograph_parser::{IsographResolutionNode, resolve_offset};
use prelude::Postfix;
use span::Span;

use crate::line_index::byte_offset_at_lsp_position;

/// The completion items at `position` in `path` as `client` reads it. `None` outside every
/// iso literal.
pub fn completion_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<Vec<lsp_types::CompletionItem>> {
    let (literal_id, before_caret) = iso_literal_before_lsp_position(db, client, path, position)?;
    before_caret
        .and_then(|span| {
            let parsed = parsed_iso_literal_by_id(db, literal_id)?;
            completion_items(db, client, resolve_offset(parsed, span)?)
        })
        .unwrap_or_default()
        .wrap_some()
}

/// The literal whose interior contains the caret at `position` (its end included), and the
/// span of the char before the caret relative to that interior. `None` for the span when
/// the caret is at the start of the interior.
fn iso_literal_before_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Option<Span>)> {
    let offset = byte_offset_at_lsp_position(db, client, path, position)?;
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions
        .iter()
        .enumerate()
        .find_map(|(index, extraction)| {
            let within = offset.checked_sub(extraction.iso_literal_start_index.0 as u32)?;
            let before = extraction.iso_literal_text.get(..within as usize)?;
            (
                LiteralId {
                    client,
                    path,
                    index,
                },
                before
                    .chars()
                    .next_back()
                    .map(|character| Span::new(within - character.len_utf8() as u32, within)),
            )
                .wrap_some()
        })
}

/// The items for a completion request whose char before the caret resolves to `node`. `None`
/// when `node` is not in a selection set, or the set's entity is unknown.
fn completion_items<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: IsographResolutionNode<'_>,
) -> Option<Vec<lsp_types::CompletionItem>> {
    let (selection_set_path, under_caret) = match node.reference() {
        IsographResolutionNode::SelectionSet(selection_set_path) => (selection_set_path, None),
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => (
            selection_name_path.parent.parent.parent.reference(),
            selection_name_path.parent.inner.wrap_some(),
        ),
        IsographResolutionNode::IsoLiteralSlot(_)
        | IsographResolutionNode::ChunkedRoot(_)
        | IsographResolutionNode::EntityDeclaration(_)
        | IsographResolutionNode::EntrypointDeclaration(_)
        | IsographResolutionNode::SelectableDeclaration(_)
        | IsographResolutionNode::Description(_)
        | IsographResolutionNode::EntityNameWrapper(_)
        | IsographResolutionNode::SelectableNameWrapper(_)
        | IsographResolutionNode::UnparsedChunkItems(_)
        | IsographResolutionNode::ChunkedLevel(_)
        | IsographResolutionNode::ChunkedGroup(_)
        | IsographResolutionNode::Chunk(_)
        | IsographResolutionNode::ChunkSeparator(_)
        | IsographResolutionNode::NonBracketToken(_)
        | IsographResolutionNode::OpenBracket(_)
        | IsographResolutionNode::CloseBracket(_)
        | IsographResolutionNode::ArgumentSlot(_)
        | IsographResolutionNode::ObjectEntrySlot(_)
        | IsographResolutionNode::ArgumentList(_)
        | IsographResolutionNode::ObjectLiteral(_)
        | IsographResolutionNode::Argument(_)
        | IsographResolutionNode::ObjectEntry(_)
        | IsographResolutionNode::ArgumentNameWrapper(_)
        | IsographResolutionNode::ValueKeyNameWrapper(_)
        | IsographResolutionNode::VariableUse(_)
        | IsographResolutionNode::VariableNameWrapper(_)
        | IsographResolutionNode::VariableDeclarationOrUsage(_)
        | IsographResolutionNode::VariableDeclarationSlot(_)
        | IsographResolutionNode::VariableDeclarationList(_)
        | IsographResolutionNode::VariableDeclaration(_)
        | IsographResolutionNode::StringLiteralValueWrapper(_)
        | IsographResolutionNode::IntegerValue(_)
        | IsographResolutionNode::BooleanValue(_)
        | IsographResolutionNode::NullValue(_)
        | IsographResolutionNode::SelectionSlot(_)
        | IsographResolutionNode::Selection(_)
        | IsographResolutionNode::NamedTypeAnnotation(_)
        | IsographResolutionNode::ListTypeAnnotation(_)
        | IsographResolutionNode::UnionTypeAnnotation(_)
        | IsographResolutionNode::IsographFieldDirectiveList(_)
        | IsographResolutionNode::IsographFieldDirective(_)
        | IsographResolutionNode::IsographDirectiveNameWrapper(_)
        | IsographResolutionNode::ListLiteral(_)
        | IsographResolutionNode::ListLiteralValue(_)
        | IsographResolutionNode::ListLiteralValueSlot(_) => return None,
    };
    let entity = entity_for_selection_set_path(db, client, selection_set_path).ok()?;
    let selected: BTreeSet<SelectableName> = selection_set_path
        .inner
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .filter(|selection| {
            under_caret
                .is_none_or(|under_caret| selection.item.name.location != under_caret.name.location)
        })
        .map(|selection| selection.item.name.item.0.unchecked_conversion())
        .collect();
    selectables_for_entity(db, client, entity)
        .as_ref()
        .ok()?
        .iter()
        .filter(|selectable| !selected.contains(&selectable.name))
        .map(completion_item)
        .collect::<Vec<_>>()
        .wrap_some()
}

fn completion_item<TSelectableData>(
    selectable: &Selectable<TSelectableData>,
) -> lsp_types::CompletionItem {
    lsp_types::CompletionItem {
        label: selectable.name.to_string(),
        kind: lsp_types::CompletionItemKind::FIELD.wrap_some(),
        detail: selectable.target_entity().map(|target| target.to_string()),
        documentation: selectable
            .description
            .map(|description| lsp_types::Documentation::String(description.to_string())),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
    use intern::string_key::Intern;
    use isograph_compiler::{
        ClientId, FileMembership, IncludeFile, IsographDb, selectables_for_entity,
    };
    use lsp_types::Position;
    use prelude::Postfix;

    use super::completion_at_lsp_position;
    use crate::test_profile::TestProfile;

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };

    const PET: &str = "iso(`entity Pet`) iso(`field Pet.owner to Pet \"Who owns it.\"`) iso(`field Pet.name`) iso(`field Pet.plain`)";

    fn client() -> ClientId {
        ClientId(1)
    }

    fn path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    /// The completion items at the `|` in the one-line `marked`, with `PET` in `src/Pet.ts`
    /// and `marked` without its `|` in `src/x.ts`.
    fn items_at(marked: &str) -> Option<Vec<lsp_types::CompletionItem>> {
        let caret = marked.find('|').expect("the fixture marks the caret");
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(path("src/Pet.ts"), SOURCE_FILE, PET.to_owned());
        db.insert_disk_file(path("src/x.ts"), SOURCE_FILE, marked.replacen('|', "", 1));
        completion_at_lsp_position(
            &db,
            client(),
            path("src/x.ts"),
            Position::new(0, caret as u32),
        )
    }

    fn labels_at(marked: &str) -> Option<Vec<String>> {
        items_at(marked).map(|items| items.into_iter().map(|item| item.label).collect())
    }

    fn labels(names: &[&str]) -> Option<Vec<String>> {
        names
            .iter()
            .map(|name| name.dereference().to_owned())
            .collect::<Vec<_>>()
            .wrap_some()
    }

    #[test]
    fn an_empty_selection_set_offers_every_field_of_its_entity() {
        for marked in ["iso(`field Pet.card { | }`)", "iso(`field Pet.card {|}`)"] {
            assert_eq!(
                labels_at(marked),
                labels(&["card", "name", "owner", "plain"]),
                "{marked:?}"
            );
        }
    }

    #[test]
    fn an_item_names_its_target_and_description() {
        let items = items_at("iso(`field Pet.card { | }`)").expect("the caret is in a literal");
        let owner = items
            .iter()
            .find(|item| item.label == "owner")
            .expect("Pet.owner is offered");
        assert_eq!(owner.kind, lsp_types::CompletionItemKind::FIELD.wrap_some());
        assert_eq!(owner.detail, "Pet".to_owned().wrap_some());
        assert_eq!(
            owner.documentation,
            lsp_types::Documentation::String("Who owns it.".to_owned()).wrap_some()
        );
        assert_eq!(owner.insert_text, None);
        let name = items
            .iter()
            .find(|item| item.label == "name")
            .expect("Pet.name is offered");
        assert_eq!(name.detail, None);
        assert_eq!(name.documentation, None);
    }

    #[test]
    fn a_partially_typed_name_offers_every_field_including_itself() {
        assert_eq!(
            labels_at("iso(`field Pet.card { na| }`)"),
            labels(&["card", "name", "owner", "plain"])
        );
        assert_eq!(
            labels_at("iso(`field Pet.card { name, na| }`)"),
            labels(&["card", "owner", "plain"])
        );
    }

    #[test]
    fn a_nested_set_offers_the_fields_of_the_target() {
        assert_eq!(
            labels_at("iso(`field Pet.card { owner { | } }`)"),
            labels(&["card", "name", "owner", "plain"])
        );
    }

    #[test]
    fn fields_already_selected_are_excluded_by_field_name() {
        assert_eq!(
            labels_at("iso(`field Pet.card { name, | }`)"),
            labels(&["card", "owner", "plain"])
        );
        assert_eq!(
            labels_at("iso(`field Pet.card { n: name, | }`)"),
            labels(&["card", "owner", "plain"])
        );
        assert_eq!(
            labels_at("iso(`field Pet.card { owner { name }, | }`)"),
            labels(&["card", "name", "plain"])
        );
    }

    #[test]
    fn outside_every_literal_is_none() {
        assert_eq!(labels_at("const a = 1;| iso(`field Pet.card { }`)"), None);
        assert_eq!(labels_at("iso(`field Pet.card { }`) |"), None);
    }

    #[test]
    fn inside_a_literal_but_not_in_a_selection_set_is_empty() {
        for marked in [
            "iso(`fi|eld Pet.card { }`)",
            "iso(`field Pet.card { name(x: 1|) }`)",
            "iso(`|field Pet.card { }`)",
            "iso(`field Pet.card|`)",
        ] {
            assert_eq!(labels_at(marked), labels(&[]), "{marked:?}");
        }
    }

    #[test]
    fn an_unknown_parent_is_empty() {
        assert_eq!(labels_at("iso(`field Nope.card { | }`)"), labels(&[]));
        assert_eq!(
            labels_at("iso(`field Pet.card { plain { | } }`)"),
            labels(&[])
        );
    }

    fn entity(name: &str) -> EntityName {
        name.intern().to()
    }

    #[test]
    fn selectables_for_entity_leaves_out_a_duplicated_field() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            path("src/Pet.ts"),
            SOURCE_FILE,
            "iso(`entity Pet`) iso(`field Pet.b`) iso(`field Pet.a`) iso(`field Pet.a`) iso(`field Other.c`)".to_owned(),
        );
        assert_eq!(
            selectables_for_entity(&db, client(), entity("Pet"))
                .clone()
                .map(|selectables| selectables
                    .into_iter()
                    .map(|selectable| selectable.name)
                    .collect::<Vec<_>>()),
            "b".intern().to::<SelectableName>().wrap_vec().wrap_ok()
        );
        assert!(
            selectables_for_entity(&db, client(), entity("Other"))
                .clone()
                .is_err()
        );
    }
}
