use intern::Lookup;
use isograph_compiler::{
    EntrypointNotFetchableError, EntrypointQueryError, QueryArtifact, SelectionMapErrorKind,
    TopLevelError, query_for_entrypoint,
};
use prelude::Postfix;

use super::{
    CLIENT, Db, PET_DEMO_SCHEMA, PET_DEMO_SOURCES, entity, located, pet_demo_sources, project_db,
    selectable, validation_errors,
};
use crate::{GraphQLNotFetchable, QUERY_TEXT_FILE_NAME};

type QueryError = EntrypointQueryError<GraphQLNotFetchable>;

fn demo(replaced: &[(&str, &str)], added: &[(&str, &str)], schema: &[(&str, &str)]) -> Db {
    let schema_files: Vec<(&str, &str)> = PET_DEMO_SCHEMA
        .iter()
        .copied()
        .chain(schema.iter().copied())
        .collect();
    project_db(&schema_files, &pet_demo_sources(replaced, added))
}

fn query(db: &Db, parent: &str, field: &str) -> Result<QueryArtifact, QueryError> {
    query_for_entrypoint(db, CLIENT, entity(parent), selectable(field)).clone()
}

fn text(db: &Db, parent: &str, field: &str) -> String {
    query(db, parent, field)
        .expect("the fixture's entrypoint has a query")
        .text
        .0
}

fn error(db: &Db, parent: &str, field: &str) -> QueryError {
    query(db, parent, field).expect_err("the fixture's entrypoint fails")
}

fn lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

#[test]
fn home_route_selects_the_inlined_fields_under_pets() {
    let db = demo(&[], &[], &[]);
    assert_eq!(
        text(&db, "Query", "HomeRoute"),
        "query HomeRoute {
  pets {
    __typename
    id
    age
    firstName
    lastName
    nickname
    picture
    species
    tagline
  }
}"
    );
    assert_eq!(
        query(&db, "Query", "HomeRoute")
            .expect("HomeRoute has a query")
            .file_name
            .map(|name| name.lookup()),
        QUERY_TEXT_FILE_NAME.wrap_some()
    );
}

#[test]
fn pet_detail_route_is_the_documented_text() {
    let db = demo(&[], &[], &[]);
    let text = text(&db, "Query", "PetDetailRoute");
    assert_eq!(
        text,
        "query PetDetailRoute($id: ID!) {
  pet____id___v_id: pet(id: $id) {
    __typename
    id
    best_friend_relationship {
      __typename
      best_friend {
        __typename
        id
        firstName
        lastName
      }
      picture_together
    }
    checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0) {
      __typename
      id
      location
      time
    }
    firstName
    lastName
    nickname
    stats {
      __typename
      cuteness
      energy
      hunger
      intelligence
      sociability
      weight
    }
  }
}"
    );
    for absent in [
        "friendName",
        "owner",
        "OwnerBadge",
        "displayName",
        "bestFriend",
    ] {
        assert!(!text.contains(absent), "{absent}");
    }
}

#[test]
fn species_route_has_a_variable_a_literal_and_an_id_under_node() {
    let db = demo(&[], &[], &[]);
    let text = text(&db, "Query", "SpeciesRoute");
    let lines = lines(text.as_str());
    assert_eq!(lines[0], "query SpeciesRoute($species: PetSpecies!) {");
    assert!(
        lines.contains(
            &"  petsBySpecies____species___v_species: petsBySpecies(species: $species) {"
        )
    );
    assert!(lines.contains(&"    favorite_phrase"));
    assert!(lines.contains(&"  viewer {"));
    let node = lines
        .iter()
        .position(|line| *line == r#"  node____id___s_1: node(id: "1") {"#)
        .expect("SpeciesRoute selects node");
    assert_eq!(lines[node + 1], "    __typename");
    assert_eq!(lines[node + 2], "    id");
    assert_eq!(lines[node + 3], "  }");
}

#[test]
fn set_tagline_is_a_mutation_with_an_object_argument() {
    let db = demo(&[], &[], &[]);
    assert_eq!(
        text(&db, "Mutation", "SetTagline"),
        "mutation SetTagline($id: ID!, $tagline: String!) {
  set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: set_pet_tagline(input: { id: $id, tagline: $tagline }) {
    __typename
    pet {
      __typename
      id
      tagline
    }
  }
}"
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
fn an_eager_field_with_two_argument_sets_prints_two_blocks() {
    let db = demo(
        &[("src/PetDetailRoute.ts", DETAIL_EAGER_CHECKINS)],
        &[],
        &[],
    );
    let text = text(&db, "Query", "PetDetailRoute");
    let lines = lines(text.as_str());
    assert!(
        lines.contains(&"    checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0) {")
    );
    assert!(
        lines.contains(&"    checkins____limit___l_3____skip___l_0: checkins(limit: 3, skip: 0) {")
    );
}

#[test]
fn an_unset_variable_omits_the_argument_and_an_unused_one_is_not_declared() {
    let db = demo(
        &[],
        &[(
            "src/Opt.ts",
            "iso(`field Pet.Opt($x: Int) { checkins(limit: $x) { id } }`)\n\
             iso(`field Query.UsesOpt($unused: Int) { pets { Opt } }`)",
        )],
        &[],
    );
    let text = text(&db, "Query", "UsesOpt");
    assert_eq!(lines(text.as_str())[0], "query UsesOpt {");
    assert!(lines(text.as_str()).contains(&"    checkins {"));
    assert!(!text.contains("$unused"));
}

#[test]
fn a_union_refinement_prints_an_inline_fragment_after_the_discriminator() {
    let db = demo(
        &[],
        &[(
            "src/Feed.ts",
            "iso(`field Query.Feed { viewer { newsfeed(limit: 2, skip: 0) { asBlogItem { title } } } }`)",
        )],
        &[],
    );
    let text = text(&db, "Query", "Feed");
    assert_eq!(
        lines(text.as_str())[1..12].to_vec(),
        vec![
            "  viewer {",
            "    __typename",
            "    id",
            "    newsfeed____limit___l_2____skip___l_0: newsfeed(limit: 2, skip: 0) {",
            "      __typename",
            "      ... on BlogItem {",
            "        __typename",
            "        id",
            "        title",
            "      }",
            "    }",
        ]
    );
}

#[test]
fn a_broken_field_fails_only_the_queries_that_reach_it() {
    let broken = PET_DEMO_SOURCES
        .iter()
        .find(|(file, _)| *file == "src/PetStatsCard.ts")
        .expect("the demo has PetStatsCard")
        .1
        .replace("    nickname\n", "    nickname\n    nope\n");
    let before = demo(&[], &[], &[]);
    let after = demo(&[("src/PetStatsCard.ts", broken.as_str())], &[], &[]);
    let EntrypointQueryError::SelectionMap(selection_map) =
        error(&after, "Query", "PetDetailRoute")
    else {
        unreachable!("the broken field is a selection map error")
    };
    assert_eq!(
        selection_map.location,
        located("src/PetStatsCard.ts", broken.as_str(), "nope").wrap_some()
    );
    for (parent, field) in [
        ("Query", "HomeRoute"),
        ("Query", "SpeciesRoute"),
        ("Mutation", "SetTagline"),
    ] {
        assert_eq!(
            query(&after, parent, field),
            query(&before, parent, field),
            "{field}"
        );
        assert!(query(&after, parent, field).is_ok(), "{field}");
    }
}

#[test]
fn renaming_a_reader_alias_leaves_the_query_equal() {
    let renamed = PET_DEMO_SOURCES
        .iter()
        .find(|(file, _)| *file == "src/PetBestFriendCard.ts")
        .expect("the demo has PetBestFriendCard")
        .1
        .replace("friendName:", "buddyName:");
    let mut db = demo(&[], &[], &[]);
    let before = query(&db, "Query", "PetDetailRoute");
    db.insert_disk_file(
        super::path("src/PetBestFriendCard.ts"),
        super::SOURCE_FILE,
        renamed,
    );
    assert_eq!(query(&db, "Query", "PetDetailRoute"), before);
}

#[test]
fn a_cycle_fails_its_query_without_panicking_and_other_queries_succeed() {
    let db = demo(
        &[],
        &[(
            "src/Loop.ts",
            "iso(`field Pet.a { b }`)\niso(`field Pet.b { a }`)\niso(`field Pet.s { s }`)\n\
             iso(`field Query.Loop { pets { a } }`)\niso(`field Query.SelfLoop { pets { s } }`)",
        )],
        &[],
    );
    for field in ["Loop", "SelfLoop"] {
        assert!(
            matches!(
                error(&db, "Query", field),
                EntrypointQueryError::SelectionMap(error)
                    if matches!(error.kind, SelectionMapErrorKind::InlineCycle(_))
            ),
            "{field}"
        );
    }
    assert!(query(&db, "Query", "HomeRoute").is_ok());
}

#[test]
fn walk_errors_reach_the_query() {
    let db = demo(
        &[],
        &[(
            "src/Bad.ts",
            "iso(`field Query.UndefinedVariable { pets { checkins(limit: $nope) { id } } }`)\n\
             iso(`field Query.DuplicateArgument { pet(id: 1, id: 2) { id } }`)\n\
             iso(`field Query.UnknownFieldArgument { pets { PetCheckinsCard(other: 1) } }`)\n\
             iso(`field Query.SelectionSetOnLeaf { pets { age { x } } }`)\n\
             iso(`field Query.MissingSelectionSet { pets { stats } }`)\n\
             iso(`field Query.TargetIsSourceEntity { ownerOfAll { id } }`)",
        )],
        &[(
            "schema/test.graphql",
            "extend type Query { ownerOfAll: Owner }",
        )],
    );
    let kind = |parent: &str, field: &str| match error(&db, parent, field) {
        EntrypointQueryError::SelectionMap(error) => error.kind.wrap_some(),
        EntrypointQueryError::Lookup(_)
        | EntrypointQueryError::NotFetchable(_)
        | EntrypointQueryError::TopLevel(_) => None,
    };
    assert!(matches!(
        kind("Query", "UndefinedVariable"),
        Some(SelectionMapErrorKind::UndefinedVariable(_))
    ));
    assert!(matches!(
        kind("Query", "DuplicateArgument"),
        Some(SelectionMapErrorKind::DuplicateArgument(_))
    ));
    assert!(matches!(
        kind("Query", "UnknownFieldArgument"),
        Some(SelectionMapErrorKind::UnknownFieldArgument(_))
    ));
    assert!(matches!(
        kind("Query", "SelectionSetOnLeaf"),
        Some(SelectionMapErrorKind::SelectionSetOnLeaf(_))
    ));
    assert!(matches!(
        kind("Query", "MissingSelectionSet"),
        Some(SelectionMapErrorKind::MissingSelectionSet(_))
    ));
    assert_eq!(
        kind("Query", "TargetIsSourceEntity"),
        SelectionMapErrorKind::TargetIsSourceEntity(entity("Owner")).wrap_some()
    );
}

#[test]
fn an_entrypoint_parent_that_cannot_be_fetched_is_not_fetchable() {
    let db = demo(
        &[],
        &[(
            "src/Unfetchable.ts",
            "iso(`field PetStats.Card { weight }`)\niso(`field String.X`)\niso(`field Node.Card { id }`)",
        )],
        &[],
    );
    assert_eq!(
        error(&db, "Owner", "OwnerBadge"),
        EntrypointQueryError::NotFetchable(EntrypointNotFetchableError::SourceEntity(entity(
            "Owner"
        )))
    );
    for (parent, field) in [
        ("PetStats", "Card"),
        ("Pet", "PetSummaryCard"),
        ("Node", "Card"),
    ] {
        assert_eq!(
            error(&db, parent, field),
            EntrypointQueryError::NotFetchable(EntrypointNotFetchableError::Protocol(
                GraphQLNotFetchable::NotRootOperationType(entity(parent))
            )),
            "{parent}"
        );
    }
    assert_eq!(
        error(&db, "String", "X"),
        EntrypointQueryError::NotFetchable(EntrypointNotFetchableError::NotComposite(entity(
            "String"
        )))
    );
    assert!(matches!(
        error(&db, "Query", "pets"),
        EntrypointQueryError::NotFetchable(EntrypointNotFetchableError::ProtocolSelectable(_))
    ));
    assert!(matches!(
        error(&db, "Query", "Nope"),
        EntrypointQueryError::Lookup(_)
    ));
}

#[test]
fn an_entrypoint_that_selects_nothing_from_the_server_is_empty() {
    let db = demo(
        &[],
        &[(
            "src/Empty.ts",
            "iso(`field Query.me to Pet { pets { id } }`)\n\
             iso(`field Query.OnlyPointer { me { id } }`)\n\
             iso(`field Query.Nothing`)",
        )],
        &[],
    );
    for field in ["OnlyPointer", "Nothing"] {
        assert_eq!(
            error(&db, "Query", field),
            EntrypointQueryError::TopLevel(TopLevelError::Empty),
            "{field}"
        );
    }
}

#[test]
fn an_entrypoint_on_a_leaf_is_a_validation_error_on_the_parent() {
    let contents = "iso(`field String.X`)\niso(`entrypoint String.X`)";
    let db = demo(&[], &[("src/Leaf.ts", contents)], &[]);
    assert_eq!(
        validation_errors(&db, "src/Leaf.ts", contents),
        vec![(
            "`String` has no selectables, so it cannot parent an entrypoint.".to_owned(),
            "String".to_owned()
        )]
    );
}
