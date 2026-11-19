use common_lang_types::{DiagnosticResult, EntityName, SelectableName};
use isograph_parser::{
    SelectableNameWrapperParent, SelectableNameWrapperPath, SelectionPath, SelectionSetParent,
    SelectionSetPath,
};
use prelude::Postfix;

use crate::{ClientId, CompilationProfile, IsographDb, Selectable, entity_named, selectable_named};

/// The selectable `selection_path` selects, as `client` reads the schema.
pub fn selectable_for_selection_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_path: &SelectionPath<'_>,
) -> DiagnosticResult<Selectable> {
    selectable_for_selection_parent(
        db,
        client,
        selection_path.parent.parent.reference(),
        selection_path.inner.name.item.0.unchecked_conversion(),
    )
}

/// The selectable named `selectable_name` on the entity the selection set at
/// `selection_set_path` selects on.
pub fn selectable_for_selection_parent<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_set_path: &SelectionSetPath<'_>,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    match selection_set_path.parent.reference() {
        SelectionSetParent::SelectableDeclaration(selectable_declaration_path) => {
            selectable_for_entity(
                db,
                client,
                selectable_declaration_path.inner.parent_type.item.0,
                selectable_name,
            )
        }
        SelectionSetParent::Selection(selection_path) => {
            let parent_selectable = selectable_for_selection_path(db, client, selection_path)?;
            selectable_for_entity(db, client, parent_selectable.target_entity, selectable_name)
        }
    }
}

/// The selectable named `selectable_name` on the entity named `entity_name`.
pub fn selectable_for_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    entity_named(db, client, entity_name).clone()?;
    selectable_named(db, client, entity_name, selectable_name).clone()
}

/// The selectable that the entrypoint or selectable declaration at the parent of
/// `selectable_name_path` names.
pub fn selectable_for_selectable_name_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selectable_name_path: &SelectableNameWrapperPath<'_>,
) -> DiagnosticResult<Selectable> {
    let parent_type = match selectable_name_path.parent.reference() {
        SelectableNameWrapperParent::EntrypointDeclaration(entrypoint_declaration_path) => {
            entrypoint_declaration_path.inner.parent_type.item
        }
        SelectableNameWrapperParent::SelectableDeclaration(selectable_declaration_path) => {
            selectable_declaration_path.inner.parent_type.item
        }
    };
    selectable_named(db, client, parent_type.0, selectable_name_path.inner.0).clone()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{Diagnostic, DiagnosticResult, EntityName};
    use intern::string_key::Intern;
    use isograph_parser::{IsographResolutionNode, parse_iso_literal, resolve_offset};
    use prelude::Postfix;
    use span::Span;

    use super::{selectable_for_selectable_name_path, selectable_for_selection_path};
    use crate::test_profile::TestProfile;
    use crate::{ClientId, IsographDb, Selectable, entity_named, selectable_named};

    const CLIENT: ClientId = ClientId(1);

    /// The walk's result for the selection whose name holds the first char of the `nth`
    /// (0-based) `needle` in `text`.
    fn walk_at(
        db: &IsographDb<TestProfile>,
        text: &str,
        needle: &str,
        nth: usize,
    ) -> DiagnosticResult<Selectable> {
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
        selectable_for_selection_path(db, CLIENT, selection_name_path.parent.reference())
    }

    /// The lookup for the declaration whose name holds the first char of `needle` in `text`.
    fn declaration_name_at(
        db: &IsographDb<TestProfile>,
        text: &str,
        needle: &str,
    ) -> DiagnosticResult<Selectable> {
        let start = text.find(needle).expect("the fixture contains the needle") as u32;
        let parsed = parse_iso_literal(text);
        assert_eq!(parsed.errors, vec![], "for literal {text:?}");
        let node = resolve_offset(parsed.reference(), Span::new(start, start + 1))
            .expect("the fixture parses to an item");
        let IsographResolutionNode::SelectableNameWrapper(selectable_name_path) = node else {
            panic!("expected {needle:?} to resolve to a selectable name, got {node}");
        };
        selectable_for_selectable_name_path(db, CLIENT, selectable_name_path.reference())
    }

    fn selectable_named_error(
        db: &IsographDb<TestProfile>,
        entity: &str,
        selectable: &str,
    ) -> Option<Diagnostic> {
        selectable_named(db, CLIENT, entity.intern().to(), selectable.intern().to())
            .clone()
            .err()
    }

    #[test]
    fn a_declaration_name_looks_up_the_selectable_on_its_parent_type() {
        let db = IsographDb::<TestProfile>::default();
        let pet_x = selectable_named_error(db.reference(), "Pet", "x");
        assert!(pet_x.is_some());
        assert_eq!(
            declaration_name_at(db.reference(), "field Pet.x { id }", "x {").err(),
            pet_x
        );
        assert_eq!(
            declaration_name_at(db.reference(), "entrypoint Query.X", "X").err(),
            selectable_named_error(db.reference(), "Query", "X")
        );
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
