use std::collections::BTreeSet;

use common_lang_types::VariableName;
use intern::string_key::Intern;
use isograph_compiler::{
    Condition, FieldSelectionMap, IdentitySelection, InlineCycle, MergeError, MergedSelection,
    MergedSelectionMap, MergedSelectionTarget, MergedValue, SelectableKey, SelectionMapError,
    SelectionMapErrorKind, UnknownFieldArgument, field_selection_map, inlined_fields,
};
use prelude::Postfix;

use super::{
    CLIENT, Db, PET_DEMO_SCHEMA, PET_DEMO_SOURCES, entity, located, pet_demo_sources, project_db,
    selectable,
};

/// The pet demo with `replaced` and `added` sources, and `schema` beside the demo's schema.
fn demo(replaced: &[(&str, &str)], added: &[(&str, &str)], schema: &[(&str, &str)]) -> Db {
    let schema_files: Vec<(&str, &str)> = PET_DEMO_SCHEMA
        .iter()
        .copied()
        .chain(schema.iter().copied())
        .collect();
    project_db(&schema_files, &pet_demo_sources(replaced, added))
}

fn field_map(db: &Db, parent: &str, field: &str) -> Result<FieldSelectionMap, SelectionMapError> {
    field_selection_map(db, CLIENT, entity(parent), selectable(field)).clone()
}

fn selections(db: &Db, parent: &str, field: &str) -> MergedSelectionMap {
    field_map(db, parent, field)
        .expect("the fixture's field has a merged selection map")
        .selections
}

fn error(db: &Db, parent: &str, field: &str) -> SelectionMapError {
    field_map(db, parent, field).expect_err("the fixture's field fails")
}

/// One line per selection: two spaces per depth, then the alias of a selectable selection
/// (with ` {` when it has selections) or `... on T {` for a conditional.
fn outline(map: &MergedSelectionMap) -> Vec<String> {
    outline_at(map, 0)
}

fn outline_at(map: &MergedSelectionMap, depth: usize) -> Vec<String> {
    let indent = "  ".repeat(depth);
    map.iter()
        .flat_map(|selection| match selection {
            MergedSelection::Selectable(selection) => match selection.target.reference() {
                MergedSelectionTarget::Leaf => {
                    format!("{indent}{}", selection.key.normalization_alias()).wrap_vec()
                }
                MergedSelectionTarget::Composite(target) => std::iter::once(format!(
                    "{indent}{} {{",
                    selection.key.normalization_alias()
                ))
                .chain(outline_at(target.selections.reference(), depth + 1))
                .collect(),
            },
            MergedSelection::Conditional(conditional) => match conditional.condition {
                Condition::SelectionEquals(on) => {
                    std::iter::once(format!("{indent}... on {on} {{"))
                        .chain(outline_at(conditional.selections.reference(), depth + 1))
                        .collect()
                }
            },
        })
        .collect()
}

fn lines(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.dereference().to_owned())
        .collect()
}

/// The identity of the composite selection reached by following `path` of selectable names.
fn identity_at(map: &MergedSelectionMap, path: &[&str]) -> IdentitySelection {
    let (first, rest) = path.split_first().expect("the test names a path");
    let target = map
        .iter()
        .find_map(|selection| match selection {
            MergedSelection::Selectable(selection) if selection.key.name == selectable(first) => {
                match selection.target.reference() {
                    MergedSelectionTarget::Composite(target) => target.wrap_some(),
                    MergedSelectionTarget::Leaf => None,
                }
            }
            MergedSelection::Selectable(_) | MergedSelection::Conditional(_) => None,
        })
        .expect("the path names composite selections");
    match rest {
        [] => target.identity,
        rest => identity_at(target.selections.reference(), rest),
    }
}

fn key(parent: &str, field: &str) -> SelectableKey {
    SelectableKey {
        entity: entity(parent),
        selectable: selectable(field),
    }
}

#[test]
fn pet_detail_route_inlines_its_fields_and_omits_pointers() {
    let db = demo(&[], &[], &[]);
    assert_eq!(
        outline(selections(&db, "Query", "PetDetailRoute").reference()),
        lines(&[
            "pet____id___v_id {",
            "  best_friend_relationship {",
            "    best_friend {",
            "      firstName",
            "      id",
            "      lastName",
            "    }",
            "    picture_together",
            "  }",
            "  checkins____limit___l_5____skip___l_0 {",
            "    id",
            "    location",
            "    time",
            "  }",
            "  firstName",
            "  id",
            "  lastName",
            "  nickname",
            "  stats {",
            "    cuteness",
            "    energy",
            "    hunger",
            "    intelligence",
            "    sociability",
            "    weight",
            "  }",
        ])
        .into_iter()
        .filter(|line| !line.trim_start().starts_with('}'))
        .collect::<Vec<_>>()
    );
}

#[test]
fn a_reader_alias_does_not_reach_the_map() {
    let db = demo(&[], &[], &[]);
    let outline = outline(selections(&db, "Pet", "PetBestFriendCard").reference());
    assert!(outline.iter().all(|line| !line.contains("friendName")));
    assert!(outline.contains(&"    firstName".to_owned()));
}

#[test]
fn the_field_variables_are_listed_in_source_order_with_defaults() {
    let db = demo(&[], &[], &[]);
    let set_tagline = field_map(&db, "Mutation", "SetTagline").expect("SetTagline merges");
    assert_eq!(
        set_tagline
            .variables
            .iter()
            .map(|variable| variable.name)
            .collect::<Vec<_>>(),
        vec!["id".intern().to::<VariableName>(), "tagline".intern().to()]
    );
    let checkins = field_map(&db, "Pet", "PetCheckinsCard").expect("PetCheckinsCard merges");
    assert_eq!(
        checkins
            .variables
            .iter()
            .map(|variable| variable.default_value.clone())
            .collect::<Vec<_>>(),
        vec![MergedValue::Integer(3).wrap_some()]
    );
}

#[test]
fn a_nested_object_argument_keeps_its_variables_in_key_order() {
    let db = demo(&[], &[], &[]);
    let map = selections(&db, "Mutation", "SetTagline");
    assert_eq!(
        outline(map.reference()),
        lines(&[
            "set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c {",
            "  pet {",
            "    id",
            "    tagline",
        ])
    );
    assert_eq!(
        map.used_variables(),
        ["id", "tagline"]
            .into_iter()
            .map(|name| name.intern().to::<VariableName>())
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn composite_targets_carry_their_protocol_identity() {
    let db = demo(&[], &[], &[]);
    let detail = selections(&db, "Query", "PetDetailRoute");
    let id = IdentitySelection::Select(selectable("id"));
    assert_eq!(identity_at(detail.reference(), &["pet"]), id);
    assert_eq!(
        identity_at(detail.reference(), &["pet", "stats"]),
        IdentitySelection::Omit
    );
    assert_eq!(
        identity_at(detail.reference(), &["pet", "best_friend_relationship"]),
        IdentitySelection::Omit
    );
    assert_eq!(
        identity_at(
            detail.reference(),
            &["pet", "best_friend_relationship", "best_friend"]
        ),
        id
    );
}

#[test]
fn selections_of_one_selectable_from_several_fields_merge() {
    let db = demo(&[], &[], &[]);
    let home = outline(selections(&db, "Query", "HomeRoute").reference());
    assert_eq!(
        home,
        lines(&[
            "pets {",
            "  age",
            "  firstName",
            "  id",
            "  lastName",
            "  nickname",
            "  picture",
            "  species",
            "  tagline",
        ])
    );
}

#[test]
fn species_route_keeps_a_literal_argument_and_a_variable() {
    let db = demo(&[], &[], &[]);
    assert_eq!(
        outline(selections(&db, "Query", "SpeciesRoute").reference()),
        lines(&[
            "node____id___s_1 {",
            "  id",
            "petsBySpecies____species___v_species {",
            "  favorite_phrase",
            "  firstName",
            "  id",
            "  lastName",
            "viewer {",
            "  id",
        ])
    );
}

const DETAIL_EAGER_CHECKINS: &str = "import { iso } from './__isograph/iso';
export const PetDetailRoute = iso(`
  field Query.PetDetailRoute($id: ID!) {
    pet(id: $id) {
      PetCheckinsCard(limit: 5)
      PetCheckinsCard
    }
  }
`)(function PetDetailRoute() { return null; });
";

#[test]
fn an_eager_field_selected_with_two_argument_sets_is_two_entries_and_a_default_fills_in() {
    let db = demo(
        &[("src/PetDetailRoute.ts", DETAIL_EAGER_CHECKINS)],
        &[],
        &[],
    );
    assert_eq!(
        outline(selections(&db, "Query", "PetDetailRoute").reference()),
        lines(&[
            "pet____id___v_id {",
            "  checkins____limit___l_3____skip___l_0 {",
            "    id",
            "    location",
            "    time",
            "  checkins____limit___l_5____skip___l_0 {",
            "    id",
            "    location",
            "    time",
            "  id",
        ])
    );
}

#[test]
fn an_unset_variable_without_a_default_removes_the_argument() {
    let db = demo(
        &[],
        &[(
            "src/Opt.ts",
            "iso(`field Pet.Opt($x: Int) { checkins(limit: $x) { id } }`)\n\
             iso(`field Query.UsesOpt { pets { Opt } }`)",
        )],
        &[],
    );
    assert_eq!(
        outline(selections(&db, "Query", "UsesOpt").reference()),
        lines(&["pets {", "  checkins {", "    id"])
    );
}

#[test]
fn a_parent_variable_passed_to_an_inlined_field_stays_a_variable_of_the_parent() {
    let db = demo(
        &[],
        &[(
            "src/Passes.ts",
            "iso(`field Query.Passes($n: Int) { pets { PetCheckinsCard(limit: $n) } }`)",
        )],
        &[],
    );
    let map = selections(&db, "Query", "Passes");
    assert_eq!(
        outline(map.reference()),
        lines(&[
            "pets {",
            "  checkins____limit___v_n____skip___l_0 {",
            "    id",
            "    location",
            "    time",
            "  id",
        ])
    );
    assert_eq!(
        map.used_variables(),
        ["n"]
            .into_iter()
            .map(|name| name.intern().to::<VariableName>())
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn an_unused_variable_is_declared_and_not_used() {
    let db = demo(
        &[],
        &[(
            "src/Unused.ts",
            "iso(`field Query.Unused($unused: Int) { pets { id } }`)",
        )],
        &[],
    );
    let field = field_map(&db, "Query", "Unused").expect("Unused merges");
    assert_eq!(field.variables.len(), 1);
    assert_eq!(field.selections.used_variables(), BTreeSet::new());
}

const FEED: &str = "iso(`field Query.Feed { viewer { newsfeed(limit: 2, skip: 0) { asBlogItem { title }, BlogTitle, asAdItem { message } } } }`)
iso(`field NewsfeedItem.BlogTitle { asBlogItem { author, title } }`)";

#[test]
fn refinements_are_conditionals_that_merge_by_entity() {
    let db = demo(&[], &[("src/Feed.ts", FEED)], &[]);
    let map = selections(&db, "Query", "Feed");
    assert_eq!(
        outline(map.reference()),
        lines(&[
            "viewer {",
            "  newsfeed____limit___l_2____skip___l_0 {",
            "    ... on AdItem {",
            "      message",
            "    ... on BlogItem {",
            "      author",
            "      title",
        ])
    );
    assert_eq!(
        identity_at(map.reference(), &["viewer", "newsfeed"]),
        IdentitySelection::Omit
    );
    let newsfeed_conditional_identities: Vec<IdentitySelection> = {
        let viewer = match map.iter().next() {
            Some(MergedSelection::Selectable(viewer)) => viewer,
            Some(MergedSelection::Conditional(_)) | None => {
                unreachable!("the fixture's first selection is viewer")
            }
        };
        let MergedSelectionTarget::Composite(viewer) = viewer.target.reference() else {
            unreachable!("viewer is composite")
        };
        let Some(MergedSelection::Selectable(newsfeed)) = viewer.selections.iter().next() else {
            unreachable!("the fixture selects newsfeed")
        };
        let MergedSelectionTarget::Composite(newsfeed) = newsfeed.target.reference() else {
            unreachable!("newsfeed is composite")
        };
        newsfeed
            .selections
            .iter()
            .filter_map(|selection| match selection {
                MergedSelection::Conditional(conditional) => conditional.identity.wrap_some(),
                MergedSelection::Selectable(_) => None,
            })
            .collect()
    };
    assert_eq!(
        newsfeed_conditional_identities,
        vec![IdentitySelection::Select(selectable("id")); 2]
    );
}

#[test]
fn a_field_without_a_selection_set_has_an_empty_map_and_contributes_nothing() {
    let db = demo(
        &[],
        &[(
            "src/Empty.ts",
            "iso(`field Pet.Nothing`)\niso(`field Query.SelectsNothing { pets { Nothing } }`)\n\
             iso(`field Query.OnlyPointer { pets { bestFriend { id } } }`)",
        )],
        &[],
    );
    assert_eq!(
        selections(&db, "Pet", "Nothing"),
        MergedSelectionMap::empty()
    );
    assert_eq!(
        outline(selections(&db, "Query", "SelectsNothing").reference()),
        lines(&["pets {"])
    );
    assert_eq!(
        outline(selections(&db, "Query", "OnlyPointer").reference()),
        lines(&["pets {"])
    );
}

#[test]
fn a_client_pointer_and_its_selection_set_add_nothing_and_take_no_part_in_cycles() {
    let db = demo(
        &[],
        &[(
            "src/Pointers.ts",
            "iso(`field Pet.p to Pet { stats { weight } }`)\n\
             iso(`field Pet.loop to Pet { loop { id } }`)\n\
             iso(`field Query.UsesPointers { pets { p { nickname }, loop { id } } }`)",
        )],
        &[],
    );
    assert_eq!(
        outline(selections(&db, "Query", "UsesPointers").reference()),
        lines(&["pets {"])
    );
    assert_eq!(
        inlined_fields(&db, CLIENT, entity("Pet"), selectable("loop")),
        &BTreeSet::new()
    );
}

#[test]
fn inlined_fields_lists_client_fields_and_not_pointers() {
    let db = demo(&[], &[], &[]);
    assert_eq!(
        inlined_fields(&db, CLIENT, entity("Query"), selectable("PetDetailRoute")).clone(),
        [
            key("Pet", "PetBestFriendCard"),
            key("Pet", "PetCheckinsCard"),
            key("Pet", "PetStatsCard"),
            key("Pet", "fullName")
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        inlined_fields(&db, CLIENT, entity("Pet"), selectable("PetStatsCard")).clone(),
        BTreeSet::new()
    );
}

const CYCLES: &str = "iso(`field Pet.a { b }`)
iso(`field Pet.b { a }`)
iso(`field Pet.s { s }`)
iso(`field Pet.x { y }`)
iso(`field Pet.y { z }`)
iso(`field Pet.z { stats { weight }, x }`)
iso(`field Query.Loop { pets { a } }`)
iso(`field Pet.reachesCycle { nickname, a }`)";

#[test]
fn mutual_self_and_longer_cycles_are_errors_and_do_not_panic() {
    let db = demo(&[], &[("src/Cycles.ts", CYCLES)], &[]);
    assert_eq!(
        error(&db, "Pet", "a").kind,
        SelectionMapErrorKind::InlineCycle(InlineCycle(vec![key("Pet", "a"), key("Pet", "b")]))
    );
    assert_eq!(
        error(&db, "Pet", "b").kind,
        SelectionMapErrorKind::InlineCycle(InlineCycle(vec![key("Pet", "b"), key("Pet", "a")]))
    );
    assert_eq!(
        error(&db, "Pet", "s").kind,
        SelectionMapErrorKind::InlineCycle(InlineCycle(vec![key("Pet", "s")]))
    );
    assert_eq!(
        error(&db, "Pet", "x").kind,
        SelectionMapErrorKind::InlineCycle(InlineCycle(vec![
            key("Pet", "x"),
            key("Pet", "y"),
            key("Pet", "z")
        ]))
    );
    assert!(matches!(
        error(&db, "Query", "Loop").kind,
        SelectionMapErrorKind::InlineCycle(_)
    ));
    assert_eq!(
        error(&db, "Pet", "reachesCycle").kind,
        SelectionMapErrorKind::InlineCycle(InlineCycle(vec![key("Pet", "a"), key("Pet", "b")]))
    );
    assert_eq!(
        error(&db, "Pet", "s").location,
        located("src/Cycles.ts", CYCLES, "s {")
            .wrap_some()
            .map(|mut location| {
                location.span.end = location.span.start + 1;
                location
            })
    );
    assert!(field_map(&db, "Query", "HomeRoute").is_ok());
    assert!(field_map(&db, "Query", "PetDetailRoute").is_ok());
}

#[test]
fn a_broken_field_fails_the_fields_that_inline_it_at_its_own_location_and_no_others() {
    let broken = PET_DEMO_SOURCES
        .iter()
        .find(|(file, _)| *file == "src/PetStatsCard.ts")
        .expect("the demo has PetStatsCard")
        .1
        .replace("    nickname\n", "    nickname\n    nope\n");
    let before = demo(&[], &[], &[]);
    let after = demo(&[("src/PetStatsCard.ts", broken.as_str())], &[], &[]);
    let expected = located("src/PetStatsCard.ts", broken.as_str(), "nope").wrap_some();
    assert_eq!(error(&after, "Pet", "PetStatsCard").location, expected);
    assert!(matches!(
        error(&after, "Pet", "PetStatsCard").kind,
        SelectionMapErrorKind::Lookup(_)
    ));
    assert_eq!(
        error(&after, "Query", "PetDetailRoute"),
        error(&after, "Pet", "PetStatsCard")
    );
    for (parent, field) in [
        ("Query", "HomeRoute"),
        ("Query", "SpeciesRoute"),
        ("Mutation", "SetTagline"),
    ] {
        assert_eq!(
            field_map(&after, parent, field),
            field_map(&before, parent, field),
            "{field}"
        );
        assert!(field_map(&after, parent, field).is_ok(), "{field}");
    }
}

#[test]
fn renaming_a_reader_alias_leaves_the_maps_equal() {
    let renamed = PET_DEMO_SOURCES
        .iter()
        .find(|(file, _)| *file == "src/PetBestFriendCard.ts")
        .expect("the demo has PetBestFriendCard")
        .1
        .replace("friendName:", "buddyName:");
    let mut db = demo(&[], &[], &[]);
    let before_card = field_map(&db, "Pet", "PetBestFriendCard");
    let before_route = field_map(&db, "Query", "PetDetailRoute");
    db.insert_disk_file(
        super::path("src/PetBestFriendCard.ts"),
        super::SOURCE_FILE,
        renamed,
    );
    assert_eq!(field_map(&db, "Pet", "PetBestFriendCard"), before_card);
    assert_eq!(field_map(&db, "Query", "PetDetailRoute"), before_route);
}

/// The error of `Query.X` with selection set `body`, and where it is in its file.
fn query_error(body: &str, schema: &[(&str, &str)]) -> (SelectionMapErrorKind, String) {
    let contents = format!("iso(`field Query.X($id: ID!) {{ {body} }}`)");
    let db = demo(&[], &[("src/X.ts", contents.as_str())], schema);
    let SelectionMapError { location, kind } = error(&db, "Query", "X");
    let location = location.expect("a selection error has a location");
    (kind, contents[location.span.as_usize_range()].to_owned())
}

#[test]
fn argument_errors_are_located_at_their_node() {
    assert_eq!(
        query_error("pets { checkins(limit: $nope) { id } }", &[]),
        (
            SelectionMapErrorKind::UndefinedVariable("nope".intern().to()),
            "$nope".to_owned()
        )
    );
    assert_eq!(
        query_error("pet(id: 1, id: 2) { id }", &[]),
        (
            SelectionMapErrorKind::DuplicateArgument("id".intern().to()),
            "(id: 1, id: 2)".to_owned()
        )
    );
    assert_eq!(
        query_error("pets { PetCheckinsCard(other: 1) }", &[]),
        (
            SelectionMapErrorKind::UnknownFieldArgument(UnknownFieldArgument {
                field: key("Pet", "PetCheckinsCard"),
                argument: "other".intern().to(),
            }),
            "other: 1".to_owned()
        )
    );
    assert_eq!(
        query_error("pet(id: [1]) { id }", &[]).0,
        SelectionMapErrorKind::ListArgument
    );
    assert_eq!(
        query_error("pet(id: { a: 1, a: 2 }) { id }", &[]).0,
        SelectionMapErrorKind::DuplicateObjectKey("a".intern().to())
    );
}

#[test]
fn a_variable_default_cannot_use_a_variable() {
    let db = demo(
        &[],
        &[("src/D.ts", "iso(`field Pet.D($a: Int = $b) { id }`)")],
        &[],
    );
    assert_eq!(
        error(&db, "Pet", "D").kind,
        SelectionMapErrorKind::VariableInDefault("b".intern().to())
    );
}

#[test]
fn shape_errors_are_located_at_the_selection_name() {
    assert_eq!(
        query_error("pets { age { x } }", &[]),
        (
            SelectionMapErrorKind::SelectionSetOnLeaf(key("Pet", "age")),
            "age".to_owned()
        )
    );
    assert_eq!(
        query_error("pets { stats }", &[]),
        (
            SelectionMapErrorKind::MissingSelectionSet(key("Pet", "stats")),
            "stats".to_owned()
        )
    );
    assert_eq!(
        query_error(
            "viewer { newsfeed(limit: 1, skip: 0) { asBlogItem(x: 1) { title } } }",
            &[]
        ),
        (
            SelectionMapErrorKind::ArgumentsOnRefinement(key("NewsfeedItem", "asBlogItem")),
            "asBlogItem".to_owned()
        )
    );
    assert_eq!(
        query_error("viewer { newsfeed(limit: 1, skip: 0) { asBlogItem } }", &[]),
        (
            SelectionMapErrorKind::MissingSelectionSet(key("NewsfeedItem", "asBlogItem")),
            "asBlogItem".to_owned()
        )
    );
    assert_eq!(
        query_error(
            "ownerOfAll { id }",
            &[(
                "schema/test.graphql",
                "extend type Query { ownerOfAll: Owner }"
            )]
        ),
        (
            SelectionMapErrorKind::TargetIsSourceEntity(entity("Owner")),
            "ownerOfAll".to_owned()
        )
    );
    assert_eq!(
        query_error(
            "params { id }",
            &[(
                "schema/test.graphql",
                "extend type Query { params: SetPetTaglineParams }"
            )]
        ),
        (
            SelectionMapErrorKind::InputTarget(entity("SetPetTaglineParams")),
            "params".to_owned()
        )
    );
    assert!(matches!(
        query_error("nope", &[]),
        (SelectionMapErrorKind::Lookup(_), located) if located == "nope"
    ));
}

#[test]
fn two_selections_with_one_alias_are_a_collision_at_the_second() {
    let (kind, at) = query_error(r#"pet(id: "a b") { id }, pet(id: "a_b") { id }"#, &[]);
    assert!(matches!(
        kind,
        SelectionMapErrorKind::Merge(MergeError::AliasCollision(_))
    ));
    assert_eq!(at, "pet");
}

#[test]
fn a_schema_field_has_no_map() {
    let db = demo(&[], &[], &[]);
    assert_eq!(
        error(&db, "Pet", "age").kind,
        SelectionMapErrorKind::NotASourceField(key("Pet", "age"))
    );
    assert!(matches!(
        error(&db, "Pet", "nope").kind,
        SelectionMapErrorKind::Lookup(_)
    ));
    assert_eq!(error(&db, "Pet", "nope").location, None);
}
