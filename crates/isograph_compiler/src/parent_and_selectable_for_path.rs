use common_lang_types::{DiagnosticResult, EntityName, SelectableName};
use isograph_parser::{SelectionPath, SelectionSetParent, SelectionSetPath};
use prelude::Postfix;

use crate::{
    ClientId, CompilationProfile, Entity, IsographDb, Selectable, entity_named, selectable_named,
};

/// The entity `selection_path` selects on and the selectable it selects, as `client` reads
/// the schema.
pub fn get_parent_and_selectable_for_selection_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_path: &SelectionPath<'_>,
) -> DiagnosticResult<(Entity, Selectable)> {
    get_parent_and_selectable_for_selection_parent(
        db,
        client,
        selection_path.parent.parent.reference(),
        selection_path.inner.name.item.0.unchecked_conversion(),
    )
}

/// The entity the selection set at `selection_set_path` selects on and its selectable named
/// `selectable_name`.
pub fn get_parent_and_selectable_for_selection_parent<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_set_path: &SelectionSetPath<'_>,
    selectable_name: SelectableName,
) -> DiagnosticResult<(Entity, Selectable)> {
    match selection_set_path.parent.reference() {
        SelectionSetParent::SelectableDeclaration(selectable_declaration_path) => {
            parent_object_entity_and_selectable(
                db,
                client,
                selectable_declaration_path.inner.parent_type.item.0,
                selectable_name,
            )
        }
        SelectionSetParent::Selection(selection_path) => {
            let (_, selectable) =
                get_parent_and_selectable_for_selection_path(db, client, selection_path)?;
            parent_object_entity_and_selectable(
                db,
                client,
                selectable.target_entity,
                selectable_name,
            )
        }
    }
}

/// The entity named `parent_entity_name` and its selectable named `selectable_name`.
pub fn parent_object_entity_and_selectable<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    parent_entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<(Entity, Selectable)> {
    let entity = entity_named(db, client, parent_entity_name).clone()?;
    let selectable = selectable_named(db, client, parent_entity_name, selectable_name).clone()?;
    (entity, selectable).wrap_ok()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{Diagnostic, DiagnosticResult, EntityName};
    use intern::string_key::Intern;
    use isograph_parser::{IsographResolutionNode, parse_iso_literal, resolve_offset};
    use prelude::Postfix;
    use span::Span;

    use super::get_parent_and_selectable_for_selection_path;
    use crate::test_profile::TestProfile;
    use crate::{ClientId, Entity, IsographDb, Selectable, entity_named};

    const CLIENT: ClientId = ClientId(1);

    /// The walk's result for the selection whose name holds the first char of the `nth`
    /// (0-based) `needle` in `text`.
    fn walk_at(
        db: &IsographDb<TestProfile>,
        text: &str,
        needle: &str,
        nth: usize,
    ) -> DiagnosticResult<(Entity, Selectable)> {
        let (start, _) = text
            .match_indices(needle)
            .nth(nth)
            .expect("the fixture contains the needle");
        let parsed = parse_iso_literal(text);
        assert_eq!(parsed.errors, vec![], "for literal {text:?}");
        let start = start as u32;
        let node = resolve_offset(parsed.reference(), Span::new(start, start + 1))
            .expect("the fixture parses to an item");
        let IsographResolutionNode::SelectionNameWrapper(selection_name_path) = node else {
            panic!("expected {needle:?} to resolve to a selection name, got {node}");
        };
        get_parent_and_selectable_for_selection_path(
            db,
            CLIENT,
            selection_name_path.parent.reference(),
        )
    }

    fn entity_named_error(db: &IsographDb<TestProfile>, name: &str) -> Option<Diagnostic> {
        let name: EntityName = name.intern().to();
        entity_named(db, CLIENT, name).clone().err()
    }

    #[test]
    fn a_top_level_selection_looks_up_the_declarations_parent_type() {
        let db = IsographDb::<TestProfile>::default();
        let pet = entity_named_error(db.reference(), "Pet");
        assert!(pet.is_some());
        assert_eq!(
            walk_at(db.reference(), "field Pet.fullName { id }", "id", 0).err(),
            pet
        );
    }

    #[test]
    fn a_nested_selection_looks_up_its_parent_selection_first() {
        let db = IsographDb::<TestProfile>::default();
        let text = "field Pet.fullName { owner { name } }";
        let pet = entity_named_error(db.reference(), "Pet");
        assert_eq!(walk_at(db.reference(), text, "name", 0).err(), pet);
        assert_eq!(walk_at(db.reference(), text, "owner", 0).err(), pet);
    }

    #[test]
    fn an_aliased_selection_looks_up_the_declarations_parent_type() {
        let db = IsographDb::<TestProfile>::default();
        let text = "field Pet.fullName { b: bar }";
        let pet = entity_named_error(db.reference(), "Pet");
        assert_eq!(walk_at(db.reference(), text, "b:", 0).err(), pet);
        assert_eq!(walk_at(db.reference(), text, "bar", 0).err(), pet);
    }
}
