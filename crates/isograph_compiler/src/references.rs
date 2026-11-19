use common_lang_types::{EntityName, RelativePathToSourceFile};
use isograph_parser::{
    EntityNameWrapper, EntrypointDeclaration, IsoLiteralItem, ListTypeAnnotation,
    SelectableDeclaration, Selection, SelectionSet, TypeAnnotation, UnionVariant,
};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::{
    ClientId, CompilationProfile, IsographDb, SelectableKey, entity_named,
    parsed_iso_literals_in_file, selectable_named, selectable_target,
};

/// The entity or selectable a name in an iso literal refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceTarget {
    Entity(EntityName),
    Selectable(SelectableKey),
}

/// Every reference to an entity or selectable in the iso literals of `path` as `client`
/// reads it, as file-absolute spans. A declaration's own name is not a reference.
#[memo]
pub fn references_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ReferenceTarget>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| {
            let offset = extraction.iso_literal_start_index.0 as u32;
            parsed
                .item
                .iter()
                .filter_map(|parse| parse.item.item.as_ref())
                .flat_map(move |item| item_references(db, client, item.item.reference()))
                .map(move |reference| {
                    reference
                        .item
                        .with_span(reference.location.with_offset(offset))
                })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

type ReferenceIter<'a> = Box<dyn Iterator<Item = WithSpan<ReferenceTarget>> + 'a>;

/// The references in one iso literal item, in literal-relative spans.
fn item_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    item: &'a IsoLiteralItem,
) -> ReferenceIter<'a> {
    match item {
        IsoLiteralItem::Entity(_) => std::iter::empty().boxed(),
        IsoLiteralItem::Entrypoint(declaration) => entrypoint_references(declaration).boxed(),
        IsoLiteralItem::Selectable(declaration) => {
            selectable_declaration_references(db, client, declaration)
        }
    }
}

/// `entrypoint E.x`: `E`, then `E.x`.
fn entrypoint_references(
    declaration: &EntrypointDeclaration,
) -> impl Iterator<Item = WithSpan<ReferenceTarget>> {
    [
        entity_reference(declaration.parent_type),
        ReferenceTarget::Selectable(SelectableKey {
            entity: declaration.parent_type.item.0,
            selectable: declaration.name.item.0,
        })
        .with_span(declaration.name.location),
    ]
    .into_iter()
}

fn entity_reference(name: WithSpan<EntityNameWrapper>) -> WithSpan<ReferenceTarget> {
    ReferenceTarget::Entity(name.item.0).with_span(name.location)
}

/// `field E.x($v: T) to U { .. }`: `E`, every entity in `T` and `U`, then the selections.
/// The selections are walked only when `E` resolves.
fn selectable_declaration_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    declaration: &'a SelectableDeclaration,
) -> ReferenceIter<'a> {
    let variables = declaration
        .variable_definitions
        .iter()
        .flat_map(|list| list.item.0.iter())
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(|variable| type_annotation_references(variable.item.type_.item.reference()));
    let target = declaration
        .target_type
        .iter()
        .flat_map(|target| type_annotation_references(target.item.reference()));
    let selections = declaration
        .selection_set
        .iter()
        .filter_map(move |selection_set| {
            let entity = entity_named(db, client, declaration.parent_type.item.0)
                .as_ref()
                .ok()?;
            selection_set_references(db, client, entity.name, selection_set.item.reference())
                .wrap_some()
        })
        .flatten();
    std::iter::once(entity_reference(declaration.parent_type))
        .chain(variables)
        .chain(target)
        .chain(selections)
        .boxed()
}

/// Every entity `annotation` names.
fn type_annotation_references(annotation: &TypeAnnotation) -> ReferenceIter<'_> {
    match annotation {
        TypeAnnotation::Named(named) => std::iter::once(entity_reference(named.name)).boxed(),
        TypeAnnotation::List(list) => list_type_annotation_references(list),
        TypeAnnotation::Union(union) => union
            .0
            .iter()
            .flat_map(|variant| match variant.item.reference() {
                UnionVariant::Named(named) => std::iter::once(entity_reference(named.name)).boxed(),
                UnionVariant::List(list) => list_type_annotation_references(list),
                UnionVariant::Null(_) => std::iter::empty().boxed(),
            })
            .boxed(),
    }
}

fn list_type_annotation_references(list: &ListTypeAnnotation) -> ReferenceIter<'_> {
    list.inner
        .iter()
        .flat_map(|inner| type_annotation_references(inner.item.reference()))
        .boxed()
}

/// The references in `selection_set`, a selection set on `parent`.
fn selection_set_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection_set: &'a SelectionSet,
) -> ReferenceIter<'a> {
    selection_set
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(move |selection| {
            selection_references(db, client, parent, selection.item.reference())
        })
        .boxed()
}

/// `P.f` at the name of `selection`, then the references in its selection set when that set
/// selects on a resolved entity.
fn selection_references<'a, TProfile: CompilationProfile>(
    db: &'a IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection: &'a Selection,
) -> ReferenceIter<'a> {
    let key = SelectableKey {
        entity: parent,
        selectable: selection.name.item.0.unchecked_conversion(),
    };
    let nested = selection
        .selection_set
        .iter()
        .filter_map(move |nested| {
            selection_set_references(
                db,
                client,
                nested_selection_set_entity(db, client, key)?,
                nested.item.reference(),
            )
            .wrap_some()
        })
        .flatten();
    std::iter::once(ReferenceTarget::Selectable(key).with_span(selection.name.location))
        .chain(nested)
        .boxed()
}

/// The entity a selection set under a selection of `key` selects on.
fn nested_selection_set_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    key: SelectableKey,
) -> Option<EntityName> {
    let selectable = selectable_named(db, client, key.entity, key.selectable)
        .as_ref()
        .ok()?;
    let target = selectable_target(selectable).ok()?;
    entity_named(db, client, target)
        .as_ref()
        .ok()
        .map(|entity| entity.name)
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpanPostfix};

    use super::{ReferenceTarget, references_in_file};
    use crate::test_profile::TestProfile;
    use crate::{ClientId, FileMembership, IncludeFile, IsographDb, SelectableKey};

    const CLIENT: ClientId = ClientId(1);
    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };
    const PET: &str = "iso(`entity Pet`)";
    const OWNER: &str = "iso(`field Pet.owner to Pet { id }`)";

    fn path(s: &str) -> RelativePathToSourceFile {
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

    fn entity(name: &str) -> ReferenceTarget {
        ReferenceTarget::Entity(name.intern().to())
    }

    fn selectable(entity: &str, name: &str) -> ReferenceTarget {
        ReferenceTarget::Selectable(SelectableKey {
            entity: entity.intern().to(),
            selectable: name.intern().to(),
        })
    }

    /// The references in `src/x.ts`, whose text is `contents`, each with the text its span
    /// covers.
    fn references_in_x<'a>(
        db: &IsographDb<TestProfile>,
        contents: &'a str,
    ) -> Vec<(ReferenceTarget, &'a str)> {
        references_in_file(db, CLIENT, path("src/x.ts"))
            .as_ref()
            .expect("the fixture writes src/x.ts")
            .iter()
            .map(|reference| {
                (
                    reference.item,
                    &contents[reference.location.as_usize_range()],
                )
            })
            .collect()
    }

    /// The references in `contents` as `src/x.ts` next to `files`.
    fn references_beside(files: &[(&str, &str)], contents: &str) -> Vec<(ReferenceTarget, String)> {
        let db = db_with(&[files, &[("src/x.ts", contents)]].concat());
        references_in_x(db.reference(), contents)
            .into_iter()
            .map(|(target, text)| (target, text.to_owned()))
            .collect()
    }

    fn named(target: ReferenceTarget, text: &str) -> (ReferenceTarget, String) {
        (target, text.to_owned())
    }

    #[test]
    fn entity_names_outside_declarations_are_entity_references() {
        let contents = "iso(`entity Pet`) iso(`entrypoint Query.Home`) \
                        iso(`field Pet.x($id: ID, $ids: [Key!]) to [Owner]`) \
                        iso(`field Pet.y to Pet!`) iso(`field Pet.z to [Pet!]!`)";
        assert_eq!(
            references_beside(&[], contents),
            vec![
                named(entity("Query"), "Query"),
                named(selectable("Query", "Home"), "Home"),
                named(entity("Pet"), "Pet"),
                named(entity("ID"), "ID"),
                named(entity("Key"), "Key"),
                named(entity("Owner"), "Owner"),
                named(entity("Pet"), "Pet"),
                named(entity("Pet"), "Pet"),
                named(entity("Pet"), "Pet"),
                named(entity("Pet"), "Pet"),
            ]
        );
    }

    #[test]
    fn spans_are_file_absolute_in_each_literal() {
        let contents = "iso(`entity Pet`)\niso(`field Pet.x { id }`)";
        let db = db_with(&[("src/x.ts", contents)]);
        let start_of = |needle: &str| {
            contents
                .find(needle)
                .expect("the fixture contains the needle")
        };
        assert_eq!(
            references_in_file(db.reference(), CLIENT, path("src/x.ts")).clone(),
            vec![
                entity("Pet").with_span(Span::from_usize(start_of("Pet.x"), start_of("Pet.x") + 3)),
                selectable("Pet", "id")
                    .with_span(Span::from_usize(start_of("id"), start_of("id") + 2)),
            ]
            .wrap_some()
        );
    }

    #[test]
    fn selections_resolve_top_down_through_pointers() {
        let contents = "iso(`field Pet.x { owner { n: fullName, bogus } }`)";
        assert_eq!(
            references_beside(&[("src/Pet.ts", PET), ("src/owner.ts", OWNER)], contents),
            vec![
                named(entity("Pet"), "Pet"),
                named(selectable("Pet", "owner"), "owner"),
                named(selectable("Pet", "fullName"), "fullName"),
                named(selectable("Pet", "bogus"), "bogus"),
            ]
        );
    }

    #[test]
    fn a_failed_lookup_stops_its_branch_and_not_its_siblings() {
        let contents = "iso(`field Pet.x { plain { a }, lost { b }, missing { c }, id }`) \
                        iso(`field Gone.y { z }`)";
        let files = [
            ("src/Pet.ts", PET),
            ("src/plain.ts", "iso(`field Pet.plain { id }`)"),
            ("src/lost.ts", "iso(`field Pet.lost to Gone { id }`)"),
        ];
        assert_eq!(
            references_beside(&files, contents),
            vec![
                named(entity("Pet"), "Pet"),
                named(selectable("Pet", "plain"), "plain"),
                named(selectable("Pet", "lost"), "lost"),
                named(selectable("Pet", "missing"), "missing"),
                named(selectable("Pet", "id"), "id"),
                named(entity("Gone"), "Gone"),
            ]
        );
    }

    #[test]
    fn a_duplicated_parent_entity_stops_the_selection_walk() {
        let contents = "iso(`field Pet.x { id }`)";
        assert_eq!(
            references_beside(&[("src/Pet.ts", PET), ("src/Pet2.ts", PET)], contents),
            vec![named(entity("Pet"), "Pet")]
        );
    }

    #[test]
    fn a_literal_with_parse_errors_contributes_what_it_parsed() {
        let contents = "iso(`field Pet.x { a, ) }`)";
        assert_eq!(
            references_beside(&[("src/Pet.ts", PET)], contents),
            vec![
                named(entity("Pet"), "Pet"),
                named(selectable("Pet", "a"), "a"),
            ]
        );
    }

    #[test]
    fn a_file_without_text_has_no_references() {
        let db = db_with(&[]);
        assert_eq!(
            references_in_file(db.reference(), CLIENT, path("src/x.ts")).clone(),
            None
        );
    }

    #[test]
    fn a_file_without_literals_or_with_an_empty_literal_has_none() {
        for contents in ["const x = 1;", "iso(``)", "iso(`  `)"] {
            assert_eq!(references_beside(&[], contents), Vec::new(), "{contents:?}");
        }
    }
}
