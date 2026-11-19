use std::collections::BTreeSet;

use common_lang_types::{EntityNameAndSelectableName, SelectionName};
use intern::string_key::Intern;
use isograph_compiler::{
    ArtifactInnerType, ArtifactObjectField, ArtifactObjectType, ArtifactSource, Condition,
    EntrypointNotFetchableError, EntrypointQueryError, EntrypointQueryFailed,
    FieldSelectionMapError, FieldTypes, GenerationError, LinkType, NormalizationArgument,
    NormalizationArguments, NormalizationObjectEntry, NormalizationValue, ProjectArtifactsError,
    ReaderLinkedSelection, ReaderRefinementSelection, ReaderResolverSelection,
    ReaderScalarSelection, ReaderSelection, ReaderSelectionSet, SelectionMapErrorKind,
    UnionWrapper, UnknownFieldArgument, Wrapper, WrapperVariant, artifact_sources_in_file,
    field_types, project_artifacts,
};
use prelude::Postfix;

use super::{
    CLIENT, Db, PET_DEMO_SCHEMA, PET_DEMO_SOURCES, entity, path, pet_demo_sources, project_db,
    selectable,
};

/// Every field the pet demo declares.
const PET_DEMO_FIELDS: [(&str, &str); 13] = [
    ("Mutation", "SetTagline"),
    ("Owner", "OwnerBadge"),
    ("Owner", "displayName"),
    ("Pet", "PetBestFriendCard"),
    ("Pet", "PetCheckinsCard"),
    ("Pet", "PetStatsCard"),
    ("Pet", "PetSummaryCard"),
    ("Pet", "bestFriend"),
    ("Pet", "fullName"),
    ("Pet", "owner"),
    ("Query", "HomeRoute"),
    ("Query", "PetDetailRoute"),
    ("Query", "SpeciesRoute"),
];

fn demo(added: &[(&str, &str)]) -> Db {
    project_db(&PET_DEMO_SCHEMA, &pet_demo_sources(&[], added))
}

fn named(parent: &str, field: &str) -> EntityNameAndSelectableName {
    EntityNameAndSelectableName {
        parent_entity_name: entity(parent),
        selectable_name: selectable(field),
    }
}

fn types(db: &Db, parent: &str, field: &str) -> Result<FieldTypes, GenerationError> {
    field_types(db, CLIENT, named(parent, field)).clone()
}

fn reader(db: &Db, parent: &str, field: &str) -> ReaderSelectionSet {
    types(db, parent, field)
        .expect("the fixture's field is valid")
        .reader_ast
}

fn error(db: &Db, parent: &str, field: &str) -> GenerationError {
    types(db, parent, field).expect_err("the fixture's field fails")
}

fn alias(name: &str) -> SelectionName {
    name.intern().to()
}

fn arguments(arguments: Vec<(&str, NormalizationValue)>) -> NormalizationArguments {
    NormalizationArguments(
        arguments
            .into_iter()
            .map(|(name, value)| NormalizationArgument {
                name: name.intern().to(),
                value,
            })
            .collect(),
    )
}

fn variable(name: &str) -> NormalizationValue {
    NormalizationValue::Variable(name.intern().to())
}

fn scalar(alias_name: &str, selectable_name: &str) -> ReaderSelection {
    ReaderSelection::Scalar(ReaderScalarSelection {
        alias: alias(alias_name),
        selectable: selectable(selectable_name),
        arguments: NormalizationArguments::default(),
    })
}

fn linked(
    name: &str,
    passed: NormalizationArguments,
    selections: Vec<ReaderSelection>,
) -> ReaderSelection {
    ReaderSelection::Linked(ReaderLinkedSelection {
        alias: alias(name),
        selectable: selectable(name),
        arguments: passed,
        selections: ReaderSelectionSet(selections),
    })
}

fn resolver(parent: &str, field: &str, passed: NormalizationArguments) -> ReaderSelection {
    ReaderSelection::Resolver(ReaderResolverSelection {
        alias: alias(field),
        field: named(parent, field),
        arguments: passed,
    })
}

/// The arguments of the only `Resolver` node of `reader`, at any depth.
fn resolver_arguments(reader: &ReaderSelectionSet) -> NormalizationArguments {
    let mut found = resolvers(reader);
    assert_eq!(found.len(), 1, "{reader:?}");
    found.remove(0).arguments.clone()
}

fn resolvers(reader: &ReaderSelectionSet) -> Vec<&ReaderResolverSelection> {
    reader
        .0
        .iter()
        .flat_map(|selection| match selection {
            ReaderSelection::Scalar(_) => Vec::new(),
            ReaderSelection::Linked(linked) => resolvers(linked.selections.reference()),
            ReaderSelection::Refinement(refinement) => resolvers(refinement.selections.reference()),
            ReaderSelection::Resolver(resolver) => resolver.wrap_vec(),
        })
        .collect()
}

/// That every node of `reader` has the key of the data field at its position, at every depth.
fn assert_reader_matches_data(reader: &ReaderSelectionSet, data: &ArtifactObjectType) {
    assert_eq!(reader.0.len(), data.0.len(), "{reader:?} {data:?}");
    reader
        .0
        .iter()
        .zip(data.0.iter())
        .for_each(|(node, field)| match node {
            ReaderSelection::Scalar(scalar) => assert_eq!(scalar.alias, field.key),
            ReaderSelection::Resolver(resolver) => assert_eq!(resolver.alias, field.key),
            ReaderSelection::Linked(linked) => {
                assert_eq!(linked.alias, field.key);
                assert_reader_matches_data(linked.selections.reference(), nested_object(field));
            }
            ReaderSelection::Refinement(refinement) => {
                assert_eq!(refinement.alias, field.key);
                assert_reader_matches_data(refinement.selections.reference(), nested_object(field));
            }
        });
}

fn nested_object(field: &ArtifactObjectField) -> &ArtifactObjectType {
    field
        .type_
        .inner_values()
        .find_map(|inner| match inner {
            ArtifactInnerType::Object(object) => object.wrap_some(),
            ArtifactInnerType::Scalar(_) | ArtifactInnerType::ClientFieldOutput(_) => None,
        })
        .expect("a Linked or Refinement node's data field is an object")
}

#[test]
fn full_name_reads_two_scalars_with_no_arguments() {
    let db = demo(&[]);
    assert_eq!(
        reader(&db, "Pet", "fullName"),
        ReaderSelectionSet(vec![
            scalar("firstName", "firstName"),
            scalar("lastName", "lastName")
        ])
    );
}

#[test]
fn every_pet_demo_reader_has_the_keys_of_its_data_type_in_order() {
    let db = demo(&[]);
    for (parent, field) in PET_DEMO_FIELDS {
        let types = types(&db, parent, field).expect("the pet demo is valid");
        assert_reader_matches_data(types.reader_ast.reference(), types.data.reference());
    }
}

#[test]
fn pet_best_friend_card_reads_the_reader_alias_and_nests_its_resolver() {
    let db = demo(&[]);
    assert_eq!(
        reader(&db, "Pet", "PetBestFriendCard"),
        ReaderSelectionSet(vec![
            scalar("id", "id"),
            linked(
                "best_friend_relationship",
                NormalizationArguments::default(),
                vec![
                    scalar("picture_together", "picture_together"),
                    linked(
                        "best_friend",
                        NormalizationArguments::default(),
                        vec![
                            scalar("id", "id"),
                            scalar("friendName", "firstName"),
                            resolver("Pet", "fullName", NormalizationArguments::default()),
                        ],
                    ),
                ],
            ),
        ])
    );
}

#[test]
fn pet_detail_route_reads_no_pointer_and_still_selects_them() {
    let db = demo(&[]);
    let types = types(&db, "Query", "PetDetailRoute").expect("the pet demo is valid");
    assert_eq!(
        types.reader_ast,
        ReaderSelectionSet(
            linked(
                "pet",
                arguments(vec![("id", variable("id"))]),
                vec![
                    scalar("id", "id"),
                    resolver("Pet", "fullName", NormalizationArguments::default()),
                    resolver("Pet", "PetStatsCard", NormalizationArguments::default()),
                    resolver(
                        "Pet",
                        "PetCheckinsCard",
                        arguments(vec![("limit", NormalizationValue::Integer(5))]),
                    ),
                    resolver(
                        "Pet",
                        "PetBestFriendCard",
                        NormalizationArguments::default()
                    ),
                ],
            )
            .wrap_vec()
        )
    );
    let pet = types.data.0.first().expect("PetDetailRoute selects pet");
    assert_eq!(
        nested_object(pet)
            .0
            .iter()
            .map(|field| field.key)
            .collect::<Vec<_>>(),
        vec![
            alias("id"),
            alias("fullName"),
            alias("PetStatsCard"),
            alias("PetCheckinsCard"),
            alias("PetBestFriendCard"),
        ]
    );
    for field in [
        named("Pet", "bestFriend"),
        named("Pet", "owner"),
        named("Owner", "OwnerBadge"),
    ] {
        assert!(types.selected.contains(field.reference()), "{field:?}");
    }
}

#[test]
fn checkins_arguments_keep_the_variable_and_are_sorted_by_name() {
    let db = demo(&[]);
    let reader = reader(&db, "Pet", "PetCheckinsCard");
    let checkins = reader
        .0
        .iter()
        .find_map(|selection| match selection {
            ReaderSelection::Linked(linked) if linked.selectable == selectable("checkins") => {
                linked.wrap_some()
            }
            ReaderSelection::Scalar(_)
            | ReaderSelection::Linked(_)
            | ReaderSelection::Refinement(_)
            | ReaderSelection::Resolver(_) => None,
        })
        .expect("PetCheckinsCard selects checkins");
    assert_eq!(
        checkins.arguments,
        arguments(vec![
            ("limit", variable("limit")),
            ("skip", NormalizationValue::Integer(0)),
        ])
    );
}

#[test]
fn set_tagline_passes_an_object_of_variables() {
    let db = demo(&[]);
    let reader = reader(&db, "Mutation", "SetTagline");
    let [ReaderSelection::Linked(set_pet_tagline)] = reader.0.as_slice() else {
        panic!("SetTagline selects one linked field: {reader:?}");
    };
    assert_eq!(
        set_pet_tagline.arguments,
        arguments(vec![(
            "input",
            NormalizationValue::Object(vec![
                NormalizationObjectEntry {
                    name: "id".intern().to(),
                    value: variable("id"),
                },
                NormalizationObjectEntry {
                    name: "tagline".intern().to(),
                    value: variable("tagline"),
                },
            ]),
        )])
    );
}

#[test]
fn a_field_with_no_selection_set_reads_nothing() {
    let db = demo(&[]);
    assert_eq!(
        reader(&db, "Owner", "displayName"),
        ReaderSelectionSet(Vec::new())
    );
}

const DEFAULTS: &str = "export const X = iso(`field Pet.X($n: Int = 3) { id }`)(() => null);
export const Y = iso(`field Pet.Y($m: Int) { id }`)(() => null);
export const SelectsX = iso(`field Pet.SelectsX { X }`)(() => null);
export const PassesX = iso(`field Pet.PassesX { X(n: 5) }`)(() => null);
export const Z = iso(`field Pet.Z($a: Int) { X(n: $a) }`)(() => null);
export const SelectsY = iso(`field Pet.SelectsY { Y }`)(() => null);
export const PassesQ = iso(`field Pet.PassesQ { X(q: 1) }`)(() => null);
";

#[test]
fn a_resolver_node_carries_the_defaults_of_the_variables_it_does_not_pass() {
    let db = demo(&[("src/Defaults.ts", DEFAULTS)]);
    assert_eq!(
        resolver_arguments(reader(&db, "Pet", "SelectsX").reference()),
        arguments(vec![("n", NormalizationValue::Integer(3))])
    );
    assert_eq!(
        resolver_arguments(reader(&db, "Pet", "PassesX").reference()),
        arguments(vec![("n", NormalizationValue::Integer(5))])
    );
    assert_eq!(
        resolver_arguments(reader(&db, "Pet", "Z").reference()),
        arguments(vec![("n", variable("a"))])
    );
    assert_eq!(
        resolver_arguments(reader(&db, "Pet", "SelectsY").reference()),
        NormalizationArguments::default()
    );
}

const REFINEMENT: &str =
    "export const R = iso(`field Query.R { node(id: \"1\") { asPet { firstName } } }`)(() => null);
export const RefinementArguments = iso(`field Query.RefinementArguments { node(id: \"1\") { asPet(x: 1) { firstName } } }`)(() => null);
export const UndefinedVariable = iso(`field Query.UndefinedVariable { pet(id: $nope) { id } }`)(() => null);
export const OutOfRange = iso(`field Pet.OutOfRange { checkins(limit: 4294967296) { id } }`)(() => null);
";

#[test]
fn a_refinement_reads_its_selections_under_its_condition() {
    let db = demo(&[("src/Refinement.ts", REFINEMENT)]);
    let types = types(&db, "Query", "R").expect("the fixture is valid");
    assert_eq!(
        types.reader_ast,
        ReaderSelectionSet(
            linked(
                "node",
                arguments(vec![("id", NormalizationValue::String("1".intern().to()))]),
                ReaderSelection::Refinement(ReaderRefinementSelection {
                    alias: alias("asPet"),
                    condition: Condition::SelectionEquals(entity("Pet")),
                    selections: ReaderSelectionSet(scalar("firstName", "firstName").wrap_vec()),
                })
                .wrap_vec(),
            )
            .wrap_vec()
        )
    );
    let node = types.data.0.first().expect("R selects node");
    let as_pet = nested_object(node).0.first().expect("node selects asPet");
    assert!(
        matches!(
            as_pet.type_.reference(),
            Wrapper::Union(UnionWrapper(variants))
                if matches!(
                    variants.as_slice(),
                    [WrapperVariant::Inner(ArtifactInnerType::Object(object)), WrapperVariant::Null]
                        if object.0.iter().map(|field| field.key).collect::<Vec<_>>()
                            == alias("firstName").wrap_vec()
                )
        ),
        "{as_pet:?}"
    );
}

#[test]
fn malformed_arguments_are_errors() {
    let db = demo(&[
        ("src/Refinement.ts", REFINEMENT),
        ("src/Defaults.ts", DEFAULTS),
    ]);
    assert!(matches!(
        error(&db, "Query", "RefinementArguments"),
        GenerationError::ArgumentsOnRefinement(key)
            if key.entity == entity("Node") && key.selectable == selectable("asPet")
    ));
    assert!(matches!(
        error(&db, "Query", "UndefinedVariable"),
        GenerationError::SelectionMap(FieldSelectionMapError::Located(error))
            if error.kind == SelectionMapErrorKind::UndefinedVariable("nope".intern().to())
    ));
    assert!(matches!(
        error(&db, "Pet", "PassesQ"),
        GenerationError::SelectionMap(FieldSelectionMapError::Located(error))
            if matches!(
                error.kind.reference(),
                SelectionMapErrorKind::UnknownFieldArgument(UnknownFieldArgument { field, argument })
                    if field.selectable == selectable("X")
                        && argument.reference() == "q".intern().to::<common_lang_types::ArgumentName>().reference()
            )
    ));
    assert!(matches!(
        error(&db, "Pet", "OutOfRange"),
        GenerationError::IntegerOutOfRange(out_of_range) if out_of_range.0 == 4_294_967_296
    ));
}

const UNREACHABLE_AND_NODE: &str =
    "export const Unused = iso(`field Pet.Unused { id }`)(() => null);
export const NodeField = iso(`field Node.NodeField { id }`)(() => null);
";

/// Every artifact source of the files of `files`.
fn sources(db: &Db, files: &[(&str, &str)]) -> BTreeSet<ArtifactSource> {
    files
        .iter()
        .flat_map(|(file, _)| {
            artifact_sources_in_file(db, CLIENT, path(file))
                .clone()
                .expect("the test inserted the file")
        })
        .map(|source| source.expect("the fixture's literals parse"))
        .collect()
}

#[test]
fn project_artifacts_carry_the_readers_of_reachable_fields_and_the_concrete_entities() {
    let added = [("src/Unused.ts", UNREACHABLE_AND_NODE)];
    let db = demo(&added);
    let files: Vec<(&str, &str)> = PET_DEMO_SOURCES
        .iter()
        .copied()
        .chain(added.iter().copied())
        .collect();
    let project = project_artifacts(&db, CLIENT, sources(&db, files.as_slice()).reference())
        .expect("the fixture is valid");
    let artifacts_of = |parent: &str, field: &str| {
        project
            .fields
            .iter()
            .find(|artifacts| artifacts.field == named(parent, field))
            .expect("the fixture declares the field")
    };
    let route = artifacts_of("Query", "PetDetailRoute");
    assert_eq!(
        route.resolver_reader,
        reader(&db, "Query", "PetDetailRoute").wrap_some()
    );
    assert_eq!(artifacts_of("Pet", "Unused").resolver_reader, None);
    assert_eq!(
        artifacts_of("Pet", "fullName").concrete_entities,
        entity("Pet").wrap_vec()
    );
    assert_eq!(
        artifacts_of("Node", "NodeField").concrete_entities,
        ["AdItem", "BlogItem", "Checkin", "Image", "Pet", "Viewer"]
            .into_iter()
            .map(entity)
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_entrypoint_on_a_node_entity_fails_the_project_artifacts() {
    let added = [(
        "src/PetTagline.ts",
        "iso(`field Pet.PetTagline { tagline }`) iso(`entrypoint Pet.PetTagline`)",
    )];
    let db = demo(&added);
    let files: Vec<(&str, &str)> = PET_DEMO_SOURCES
        .iter()
        .copied()
        .chain(added.iter().copied())
        .collect();
    assert_eq!(
        project_artifacts(&db, CLIENT, sources(&db, files.as_slice()).reference()),
        ProjectArtifactsError::EntrypointQuery(EntrypointQueryFailed {
            entrypoint: named("Pet", "PetTagline"),
            error: EntrypointQueryError::NotFetchable(
                EntrypointNotFetchableError::WrappedNotSupported(entity("Pet"))
            ),
        })
        .wrap_err()
    );
}

#[test]
fn every_entity_a_field_is_declared_on_has_a_link_type() {
    let db = demo(&[]);
    let project = project_artifacts(&db, CLIENT, sources(&db, &PET_DEMO_SOURCES).reference())
        .expect("the pet demo is valid");
    assert_eq!(
        project.link_types,
        ["Mutation", "Owner", "Pet", "Query"]
            .into_iter()
            .map(|name| LinkType {
                entity: entity(name),
                concrete_types: entity(name).wrap_vec(),
            })
            .collect::<Vec<_>>()
    );
}
