use intern::Lookup;
use std::collections::BTreeSet;

use isograph_compiler::{
    CompletedIdentity, CompletedSelection, CompletedSelectionMap, CompletedSelectionTarget,
    EntrypointNotFetchableError, EntrypointQueryError, FieldSelectionMapError, FixedRecordId,
    IntegerOutOfRange, NormalizationAlias, NormalizationAst, NormalizationCompositeTarget,
    NormalizationNode, NormalizationRootRecord, NormalizationSelection, NormalizationTarget,
    QueryArtifact, RecordEntity, ResponseKey, RootEntities, SelectionMapErrorKind, TopLevelError,
    field_selection_map, query_for_entrypoint,
};
use prelude::Postfix;

use super::{
    CLIENT, Db, PET_DEMO_SCHEMA, PET_DEMO_SOURCES, entity, located, pet_demo_sources, project_db,
    selectable, validation_errors,
};
use crate::print_operation::{ast_lines, text_lines, without_discriminator};
use crate::{
    GraphQLNetworkProtocol, GraphQLNotFetchable, QUERY_TEXT_FILE_NAME, graphql_fetch_plan,
};

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
fn an_unpassed_variable_fails_the_query_and_an_unused_one_is_not_declared() {
    let db = demo(
        &[],
        &[(
            "src/Opt.ts",
            "iso(`field Pet.Opt($x: Int) { checkins(limit: $x) { id } }`)\n\
             iso(`field Query.UsesOpt { pets { Opt } }`)\n\
             iso(`field Query.PassesNull($unused: Int) { pets { Opt(x: null) } }`)",
        )],
        &[],
    );
    assert!(matches!(
        error(&db, "Query", "UsesOpt"),
        EntrypointQueryError::SelectionMap(FieldSelectionMapError::Located(error))
            if matches!(error.kind, SelectionMapErrorKind::MissingFieldArgument(_))
    ));
    let text = text(&db, "Query", "PassesNull");
    assert_eq!(lines(text.as_str())[0], "query PassesNull {");
    assert!(
        lines(text.as_str()).contains(&"    checkins____limit___l_null: checkins(limit: null) {")
    );
    assert!(!text.contains("$unused"));
}

#[test]
fn an_unpassed_variable_is_a_validation_error_on_the_selection() {
    let contents = "iso(`field Pet.Opt($x: Int, $d: Int = 1) { checkins(limit: $x, skip: $d) { id } }`)\n\
         iso(`field Query.Missing { pets { Opt } }`)\n\
         iso(`field Query.Passed { pets { Opt(x: null) } }`)\n\
         iso(`field Pet.p($n: Int) to Pet { id }`)\n\
         iso(`field Query.PointerMissing { pets { p { id } } }`)";
    let db = demo(&[], &[("src/Opt.ts", contents)], &[]);
    assert_eq!(
        validation_errors(&db, "src/Opt.ts", contents),
        vec![
            (
                "`Pet.Opt` declares `$x` with no default, and this selection does not pass it."
                    .to_owned(),
                "Opt".to_owned()
            ),
            (
                "`Pet.p` declares `$n` with no default, and this selection does not pass it."
                    .to_owned(),
                "p".to_owned()
            ),
        ]
    );
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
    let EntrypointQueryError::SelectionMap(FieldSelectionMapError::Located(selection_map)) =
        error(&after, "Query", "PetDetailRoute")
    else {
        unreachable!("the broken field is a selection map error")
    };
    assert_eq!(
        selection_map.location,
        located("src/PetStatsCard.ts", broken.as_str(), "nope")
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
                EntrypointQueryError::SelectionMap(FieldSelectionMapError::Located(error))
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
        EntrypointQueryError::SelectionMap(FieldSelectionMapError::Located(error)) => {
            error.kind.wrap_some()
        }
        EntrypointQueryError::SelectionMap(FieldSelectionMapError::FieldLookup(_)) => None,
        EntrypointQueryError::Lookup(_)
        | EntrypointQueryError::NotFetchable(_)
        | EntrypointQueryError::TopLevel(_)
        | EntrypointQueryError::IntegerOutOfRange(_) => None,
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

#[test]
fn a_nested_selection_of_a_root_entity_selects_no_identity() {
    let schema = "scalar ID\nscalar String\ntype Query { id: ID! greeting: String query: Query! tags: [String] }\n";
    let db = project_db(
        &[("schema/schema.graphql", schema)],
        &[(
            "src/Nested.ts",
            "iso(`field Query.NestedQuery { greeting, query { tags } }`)",
        )],
    );
    assert_eq!(
        text(&db, "Query", "NestedQuery"),
        "query NestedQuery {
  greeting
  query {
    __typename
    tags
  }
}"
    );
}

#[test]
fn a_renamed_query_root_is_the_root_entity() {
    let schema = "scalar String\nschema { query: RootQuery }\ntype RootQuery { me: RootQuery! name: String }\n";
    let db = project_db(
        &[("schema/schema.graphql", schema)],
        &[("src/Me.ts", "iso(`field RootQuery.Me { me { name } }`)")],
    );
    let roots = graphql_fetch_plan(&db, CLIENT, entity("RootQuery"))
        .clone()
        .expect("RootQuery is the query root")
        .roots;
    assert_eq!(
        roots,
        RootEntities {
            operation: entity("RootQuery"),
            others: BTreeSet::new(),
        }
    );
    let field = field_selection_map(&db, CLIENT, entity("RootQuery"), selectable("Me"))
        .clone()
        .expect("Me merges");
    let completed = CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(
        field.selections.reference(),
        roots.reference(),
    )
    .expect("Me selects me");
    let me = completed
        .iter()
        .find_map(|selection| match selection {
            CompletedSelection::Selectable(selection) => match selection.target.reference() {
                CompletedSelectionTarget::Composite(target) => target.wrap_some(),
                CompletedSelectionTarget::Leaf => None,
            },
            CompletedSelection::Discriminator
            | CompletedSelection::Identity(_)
            | CompletedSelection::Conditional(_) => None,
        })
        .expect("me is composite");
    assert_eq!(me.entity, RecordEntity::Known(entity("RootQuery")));
    assert_eq!(me.identity, CompletedIdentity::Fixed(FixedRecordId::root()));
}

fn ast(db: &Db, parent: &str, field: &str) -> NormalizationAst {
    query(db, parent, field)
        .expect("the fixture's entrypoint has a query")
        .normalization_ast
}

#[test]
fn the_pet_demo_asts_are_their_queries_without_the_discriminator() {
    let db = demo(&[], &[], &[]);
    for (parent, field) in [
        ("Query", "HomeRoute"),
        ("Query", "PetDetailRoute"),
        ("Query", "SpeciesRoute"),
        ("Mutation", "SetTagline"),
    ] {
        let artifact = query(&db, parent, field).expect("the demo's queries generate");
        assert_eq!(
            ast_lines(artifact.normalization_ast.reference()),
            without_discriminator(text_lines(artifact.text.reference()), "__typename"),
            "{field}"
        );
        assert_eq!(
            artifact.normalization_ast.root,
            NormalizationRootRecord {
                entity_name: entity(parent),
                id: FixedRecordId::root(),
            },
            "{field}"
        );
        assert_eq!(
            artifact.normalization_ast.discriminator_response_key,
            selectable("__typename")
        );
    }
}

/// The top-level `Selection` node of `ast` whose selectable is `name`.
fn top_selection<'a>(ast: &'a NormalizationAst, name: &str) -> &'a NormalizationSelection {
    ast.selections
        .0
        .iter()
        .find_map(|node| match node {
            NormalizationNode::Selection(selection)
                if selection.selectable_name == selectable(name) =>
            {
                selection.wrap_some()
            }
            NormalizationNode::Selection(_) | NormalizationNode::Conditional(_) => None,
        })
        .expect("the AST selects this at the top level")
}

fn composite(selection: &NormalizationSelection) -> &NormalizationCompositeTarget {
    match selection.target.reference() {
        NormalizationTarget::Composite(target) => target,
        NormalizationTarget::Leaf => unreachable!("the fixture's selection is composite"),
    }
}

#[test]
fn species_route_s_node_selection_reads_the_discriminator_and_the_id() {
    let db = demo(&[], &[], &[]);
    let ast = ast(&db, "Query", "SpeciesRoute");
    let node = composite(top_selection(ast.reference(), "node"));
    assert_eq!(node.entity, RecordEntity::Discriminator);
    assert_eq!(node.identity, CompletedIdentity::Select(selectable("id")));
    let viewer = composite(top_selection(ast.reference(), "viewer"));
    assert_eq!(viewer.entity, RecordEntity::Known(entity("Viewer")));
}

#[test]
fn pet_detail_route_s_top_node_is_the_aliased_pet_selection_without_pointers() {
    let db = demo(&[], &[], &[]);
    let ast = ast(&db, "Query", "PetDetailRoute");
    let [NormalizationNode::Selection(pet)] = ast.selections.0.as_slice() else {
        unreachable!("PetDetailRoute selects only pet")
    };
    assert_eq!(pet.selectable_name, selectable("pet"));
    assert_eq!(
        pet.response_key,
        ResponseKey::Alias(NormalizationAlias("pet____id___v_id".to_owned()))
    );
    let names: Vec<String> = ast_lines(ast.reference())
        .into_iter()
        .map(|line| format!("{line:?}"))
        .collect();
    assert!(
        names
            .iter()
            .all(|line| !line.contains("\"owner\"") && !line.contains("\"bestFriend\""))
    );
    assert_eq!(composite(pet).entity, RecordEntity::Known(entity("Pet")));
}

#[test]
fn an_integer_outside_i32_fails_the_query() {
    let db = demo(
        &[],
        &[(
            "src/Big.ts",
            "iso(`field Query.Big { pet(id: 4294967296) { id } }`)",
        )],
        &[],
    );
    assert_eq!(
        error(&db, "Query", "Big"),
        EntrypointQueryError::IntegerOutOfRange(IntegerOutOfRange(4_294_967_296))
    );
}
