use std::path::Path;

use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
use graphql_schema_parser::OperationType;
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, DefinitionLocation, FileMembership, IncludeFile, IsographDb,
    NetworkProtocol, NonNullVariableType, ProtocolSelectable, UnionWrapper, VariableType, Wrapper,
    WrapperVariant, iso_literal_validation_errors_in_file,
};
use isograph_extract_typescript::TypeScriptHostLanguage;
use prelude::Postfix;
use span::Span;

use crate::{
    DuplicateOperation, EntityLookupError, FetchRefinement, GraphQLCompositeData,
    GraphQLEntityData, GraphQLFetchArgument, GraphQLFetchPlan, GraphQLNetworkProtocol,
    GraphQLNodeFetch, GraphQLNotFetchable, GraphQLRootFetch, GraphQLSelectableData,
    GraphQLSelectableLookupError, NodeFieldError, NodeFieldTypeMismatch, RootOperationTypes,
    RootOperationTypesError, SelectableLookupError, entity_named, graphql_entity,
    graphql_fetch_plan, graphql_selectable, graphql_selectables, parsed_schema_file,
    root_operation_types, schema_file_paths, selectable_named,
};

struct GraphQLProfile;

impl CompilationProfile for GraphQLProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = GraphQLNetworkProtocol;
}

type Db = IsographDb<GraphQLProfile>;

const SCHEMA_FILE: FileMembership = FileMembership {
    source_file: IncludeFile::Skip,
    schema_file: IncludeFile::Include,
};

const CLIENT: ClientId = ClientId(1);

fn path(s: &str) -> RelativePathToSourceFile {
    s.intern().to()
}

fn entity(s: &str) -> EntityName {
    s.intern().to()
}

fn selectable(s: &str) -> SelectableName {
    s.intern().to()
}

/// A database with each `(path, contents)` on disk as a schema file. Every file is inserted
/// before the first lookup (integration.md, Known gap).
fn db_with(files: &[(&str, &str)]) -> Db {
    let mut db = Db::default();
    for (file, contents) in files {
        db.insert_disk_file(path(file), SCHEMA_FILE, contents.dereference().to_owned());
    }
    db
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

/// The location of `name` at the start of the first `context` in `contents`.
fn located_in(file: &str, contents: &str, context: &str, name: &str) -> DefinitionLocation {
    let start = contents
        .find(context)
        .expect("the fixture contains the context");
    DefinitionLocation {
        path: path(file),
        span: Span::from_usize(start, start + name.len()),
    }
}

fn entity_error(db: &Db, name: &str) -> EntityLookupError {
    entity_named(db, CLIENT, entity(name))
        .clone()
        .expect_err("the fixture makes this lookup fail")
}

fn field(db: &Db, parent: &str, name: &str) -> ProtocolSelectable<GraphQLSelectableData> {
    graphql_selectable(db, CLIENT, entity(parent), selectable(name))
        .clone()
        .expect("the fixture defines this selectable without error")
        .expect("the fixture defines this selectable")
}

fn nullable(variant: WrapperVariant<EntityName>) -> Wrapper<EntityName> {
    Wrapper::Union(UnionWrapper(vec![variant, WrapperVariant::Null]))
}

#[test]
fn should_include_schema_file_includes_graphql_and_graphqls() {
    for file in ["a.graphql", "schema/a.graphqls"] {
        assert_eq!(
            GraphQLNetworkProtocol::should_include_schema_file(Path::new(file)),
            IncludeFile::Include,
            "{file}"
        );
    }
    for file in ["a.gql", "a.ts", "a.json", "graphql", ""] {
        assert_eq!(
            GraphQLNetworkProtocol::should_include_schema_file(Path::new(file)),
            IncludeFile::Skip,
            "{file}"
        );
    }
}

#[test]
fn schema_file_paths_are_disk_paths_and_the_clients_open_paths_sorted() {
    let mut db = db_with(&[("b.graphql", ""), ("a.graphql", "")]);
    assert_eq!(
        schema_file_paths(&db, CLIENT).clone(),
        vec![path("a.graphql"), path("b.graphql")]
    );
    db.set_open_file(CLIENT, path("c.graphql"), SCHEMA_FILE, String::new());
    db.set_open_file(CLIENT, path("a.graphql"), SCHEMA_FILE, String::new());
    assert_eq!(
        schema_file_paths(&db, CLIENT).clone(),
        vec![path("a.graphql"), path("b.graphql"), path("c.graphql")]
    );
    assert_eq!(
        schema_file_paths(&db, ClientId(2)).clone(),
        vec![path("a.graphql"), path("b.graphql")]
    );
    db.remove_disk_file(path("b.graphql"));
    assert_eq!(
        schema_file_paths(&db, ClientId(2)).clone(),
        vec![path("a.graphql")]
    );
}

#[test]
fn parsed_schema_file_parses_the_clients_buffer_and_is_none_without_text() {
    let mut db = db_with(&[("a.graphql", "type Disk { a: A }")]);
    db.set_open_file(
        CLIENT,
        path("a.graphql"),
        SCHEMA_FILE,
        "type Open { a: A }".to_owned(),
    );
    let definitions = |client: ClientId| {
        parsed_schema_file(&db, client, path("a.graphql"))
            .clone()
            .expect("the file has text")
            .definitions
            .len()
    };
    assert_eq!(definitions(CLIENT), 1);
    assert!(
        entity_named(&db, CLIENT, entity("Open"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
    assert_eq!(
        entity_named(&db, CLIENT, entity("Disk")).clone(),
        None.wrap_ok()
    );
    assert!(
        entity_named(&db, ClientId(2), entity("Disk"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
    assert_eq!(
        parsed_schema_file(&db, CLIENT, path("missing.graphql")).clone(),
        None
    );
}

#[test]
fn a_definition_merges_with_extensions_in_any_order_and_file() {
    for files in [
        [
            ("a.graphql", "type User { id: ID! }"),
            ("b.graphql", "extend type User { name: String }"),
        ],
        [
            ("a.graphql", "extend type User { name: String }"),
            ("b.graphql", "type User { id: ID! }"),
        ],
    ] {
        let db = db_with(&files);
        let user = entity_named(&db, CLIENT, entity("User"))
            .clone()
            .expect("User parses")
            .expect("User is defined");
        assert_eq!(user.extensions.len(), 1, "{files:?}");
        assert_ne!(user.definition.path, user.extensions[0].path, "{files:?}");
        let extension_file = user.extensions[0].path;
        assert_eq!(
            selectable_named(&db, CLIENT, entity("User"), selectable("name"))
                .clone()
                .expect("name parses")
                .map(|field| field.path),
            extension_file.wrap_some(),
            "{files:?}"
        );
        assert!(
            selectable_named(&db, CLIENT, entity("User"), selectable("id"))
                .clone()
                .is_ok_and(|f| f.is_some())
        );
    }
}

#[test]
fn two_definitions_are_a_duplicate_of_that_entity_only() {
    let db = db_with(&[
        ("a.graphql", "type User { a: A } type Query { a: A }"),
        ("b.graphql", "type User { b: B }"),
    ]);
    let EntityLookupError::DuplicateDefinition(duplicate) = entity_error(&db, "User") else {
        panic!("expected a duplicate definition");
    };
    assert_eq!(duplicate.first.path, path("a.graphql"));
    assert_eq!(duplicate.second.path, path("b.graphql"));
    assert!(
        entity_named(&db, CLIENT, entity("Query"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );

    let one_file = db_with(&[("a.graphql", "type User { a: A } type User { b: B }")]);
    assert!(matches!(
        entity_error(&one_file, "User"),
        EntityLookupError::DuplicateDefinition(_)
    ));
}

#[test]
fn an_extension_without_a_definition_or_of_another_kind_is_an_error_of_that_entity() {
    let db = db_with(&[("a.graphql", "extend type User { a: A } type Other { a: A }")]);
    assert!(matches!(
        entity_error(&db, "User"),
        EntityLookupError::ExtensionWithoutDefinition(_)
    ));
    assert!(
        entity_named(&db, CLIENT, entity("Other"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );

    let mismatch = db_with(&[("a.graphql", "type User { a: A } extend input User { b: B }")]);
    assert!(matches!(
        entity_error(&mismatch, "User"),
        EntityLookupError::ExtensionKindMismatch(_)
    ));
}

#[test]
fn a_broken_definition_fails_only_lookups_of_its_name() {
    let db = db_with(&[
        ("a.graphql", "type User { a: ! }"),
        ("b.graphql", "type Query { user: User }"),
    ]);
    let EntityLookupError::Syntax(syntax) = entity_error(&db, "User") else {
        panic!("expected a syntax error");
    };
    assert_eq!(syntax.failed.path, path("a.graphql"));
    assert!(
        entity_named(&db, CLIENT, entity("Query"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
    assert_eq!(
        field(&db, "Query", "user").target,
        nullable(WrapperVariant::Inner(entity("User")))
    );

    let same_file = db_with(&[("a.graphql", "type Broken { a: ! } type User { id: ID! }")]);
    assert!(
        entity_named(&same_file, CLIENT, entity("User"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
}

#[test]
fn missing_names_are_none() {
    let db = db_with(&[(
        "a.graphql",
        "type User { id: ID! } type Query { a: Missing }",
    )]);
    assert_eq!(
        entity_named(&db, CLIENT, entity("Nobody")).clone(),
        None.wrap_ok()
    );
    assert_eq!(
        entity_named(&db, CLIENT, entity("Missing")).clone(),
        None.wrap_ok()
    );
    assert_eq!(
        selectable_named(&db, CLIENT, entity("User"), selectable("nope")).clone(),
        None.wrap_ok()
    );
    assert_eq!(
        field(&db, "Query", "a").target,
        nullable(WrapperVariant::Inner(entity("Missing")))
    );
}

#[test]
fn a_field_defined_twice_is_an_error_of_that_field_only() {
    let db = db_with(&[(
        "a.graphql",
        "type User { id: ID! other: Int } extend type User { id: ID }",
    )]);
    assert!(matches!(
        selectable_named(&db, CLIENT, entity("User"), selectable("id")).clone(),
        Err(SelectableLookupError::DuplicateField(_))
    ));
    assert!(
        selectable_named(&db, CLIENT, entity("User"), selectable("other"))
            .clone()
            .is_ok_and(|f| f.is_some())
    );
}

#[test]
fn an_open_buffer_that_breaks_a_type_breaks_it_for_that_client_only() {
    let mut db = db_with(&[("a.graphql", "type User { id: ID! }")]);
    db.set_open_file(
        CLIENT,
        path("a.graphql"),
        SCHEMA_FILE,
        "type User { id: ! }".to_owned(),
    );
    assert!(matches!(
        entity_error(&db, "User"),
        EntityLookupError::Syntax(_)
    ));
    assert!(
        entity_named(&db, ClientId(2), entity("User"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
}

#[test]
fn graphql_entity_data_follows_the_kind_of_type() {
    let schema = "scalar ID enum E { A } input I { a: Int } union U = T \
        interface Node { id: ID! } type T implements Node { id: ID! } \
        extend type T implements Other";
    let db = db_with(&[("a.graphql", schema)]);
    let data = |name: &str| {
        graphql_entity(&db, CLIENT, entity(name))
            .clone()
            .expect("the fixture parses")
            .expect("the fixture defines the type")
            .data
    };
    assert_eq!(data("ID"), GraphQLEntityData::Leaf);
    assert_eq!(data("E"), GraphQLEntityData::Leaf);
    assert_eq!(data("I"), GraphQLEntityData::InputObject);
    assert_eq!(data("U"), GraphQLEntityData::Union);
    assert_eq!(
        data("Node"),
        GraphQLEntityData::Interface(GraphQLCompositeData { interfaces: vec![] })
    );
    assert_eq!(
        data("T"),
        GraphQLEntityData::Object(GraphQLCompositeData {
            interfaces: vec![entity("Node"), entity("Other")]
        })
    );
    assert_eq!(
        graphql_entity(&db, CLIENT, entity("T"))
            .clone()
            .expect("T parses")
            .map(|t| t.definition),
        located_in("a.graphql", schema, "T implements", "T").wrap_some()
    );
}

#[test]
fn a_field_targets_its_type_as_a_wrapper() {
    let schema = "type Query { pets: [Pet!]! pet: Pet maybe: [Pet] \"Its id.\" id: ID! }";
    let db = db_with(&[("a.graphql", schema)]);
    let pets = field(&db, "Query", "pets");
    assert_eq!(
        pets.target,
        Wrapper::List(Wrapper::Inner(entity("Pet")).boxed())
    );
    assert_eq!(pets.data, GraphQLSelectableData::Field);
    assert_eq!(pets.definition, located("a.graphql", schema, "pets"));
    assert_eq!(
        field(&db, "Query", "pet").target,
        nullable(WrapperVariant::Inner(entity("Pet")))
    );
    assert_eq!(
        field(&db, "Query", "maybe").target,
        nullable(WrapperVariant::List(
            nullable(WrapperVariant::Inner(entity("Pet"))).boxed()
        ))
    );
    let id = field(&db, "Query", "id");
    assert_eq!(id.target, Wrapper::Inner(entity("ID")));
    assert_eq!(
        id.description.map(|d| d.to_string()),
        "Its id.".to_owned().wrap_some()
    );
}

#[test]
fn a_union_refines_to_each_object_member_of_its_definition_and_extensions() {
    let schema = "union U = A | B type A { a: Int } type B { b: Int } type C { c: Int }";
    let extension = "extend union U = C";
    let db = db_with(&[("a.graphql", schema), ("b.graphql", extension)]);
    let as_a = field(&db, "U", "asA");
    assert_eq!(as_a.data, GraphQLSelectableData::Refinement);
    assert_eq!(as_a.target, nullable(WrapperVariant::Inner(entity("A"))));
    assert_eq!(as_a.definition, located_in("a.graphql", schema, "A {", "A"));
    assert_eq!(
        field(&db, "U", "asC").target,
        nullable(WrapperVariant::Inner(entity("C")))
    );
    assert_eq!(
        graphql_selectable(&db, CLIENT, entity("U"), selectable("asD")).clone(),
        None.wrap_ok()
    );
    assert_eq!(
        graphql_selectables(&db, CLIENT, entity("U"))
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>(),
        ["asA", "asB", "asC"]
    );
}

#[test]
fn an_interface_refines_to_each_object_implementing_it_and_not_to_interfaces() {
    let db = db_with(&[
        (
            "a.graphql",
            "interface Node { id: ID! } type Pet implements Node { id: ID! } \
             interface Named implements Node { id: ID! } type Owner { id: ID! }",
        ),
        ("b.graphql", "extend type Owner implements Node"),
    ]);
    assert_eq!(
        field(&db, "Node", "asPet").target,
        nullable(WrapperVariant::Inner(entity("Pet")))
    );
    assert_eq!(
        field(&db, "Node", "asOwner").target,
        nullable(WrapperVariant::Inner(entity("Owner")))
    );
    assert_eq!(
        graphql_selectable(&db, CLIENT, entity("Node"), selectable("asNamed")).clone(),
        None.wrap_ok()
    );
    assert_eq!(
        graphql_selectables(&db, CLIENT, entity("Node"))
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>(),
        ["id", "asOwner", "asPet"]
    );
}

#[test]
fn a_broken_refinement_target_fails_that_refinement_only() {
    let db = db_with(&[(
        "a.graphql",
        "union U = A | B type A { a: ! } type B { b: Int }",
    )]);
    assert!(matches!(
        graphql_selectable(&db, CLIENT, entity("U"), selectable("asA")).clone(),
        Err(GraphQLSelectableLookupError::RefinementTarget(_))
    ));
    assert!(
        graphql_selectable(&db, CLIENT, entity("U"), selectable("asB"))
            .clone()
            .is_ok_and(|s| s.is_some())
    );
    assert_eq!(
        graphql_selectables(&db, CLIENT, entity("U"))
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>(),
        ["asB"]
    );
}

#[test]
fn a_field_named_like_a_refinement_collides_with_it() {
    let db = db_with(&[(
        "a.graphql",
        "interface I { asPet: Pet other: Int } type Pet implements I { asPet: Pet other: Int }",
    )]);
    assert!(matches!(
        graphql_selectable(&db, CLIENT, entity("I"), selectable("asPet")).clone(),
        Err(GraphQLSelectableLookupError::RefinementCollision(_))
    ));
    assert!(
        graphql_selectable(&db, CLIENT, entity("I"), selectable("other"))
            .clone()
            .is_ok_and(|s| s.is_some())
    );
}

#[test]
fn scalars_are_entities_and_the_selectables_of_a_broken_type_are_empty() {
    let db = db_with(&[("a.graphql", "scalar String type Broken { a: ! }")]);
    assert!(
        graphql_entity(&db, CLIENT, entity("String"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
    assert!(graphql_selectables(&db, CLIENT, entity("Broken")).is_empty());
}

#[test]
fn adding_and_removing_a_schema_file_changes_lookups() {
    let mut db = db_with(&[("a.graphql", "type User { id: ID! }")]);
    let extensions = |db: &Db| {
        entity_named(db, CLIENT, entity("User"))
            .clone()
            .expect("User parses")
            .expect("User is defined")
            .extensions
            .len()
    };
    assert_eq!(extensions(&db), 0);
    db.insert_disk_file(
        path("b.graphql"),
        SCHEMA_FILE,
        "extend type User { name: String }".to_owned(),
    );
    assert_eq!(extensions(&db), 1);
    db.remove_disk_file(path("b.graphql"));
    assert_eq!(extensions(&db), 0);
}

#[test]
fn a_deleted_schema_file_contributes_nothing() {
    let mut db = db_with(&[
        ("a.graphql", "type User { id: ID! }"),
        ("b.graphql", "type Gone"),
    ]);
    db.remove_disk_file(path("b.graphql"));
    assert!(
        entity_named(&db, CLIENT, entity("User"))
            .clone()
            .is_ok_and(|e| e.is_some())
    );
    assert_eq!(
        entity_named(&db, CLIENT, entity("Gone")).clone(),
        None.wrap_ok()
    );
}

const PET_DEMO_SCHEMA: [(&str, &str); 3] = [
    (
        "schema/schema.graphql",
        include_str!("../../../demos/pet-demo/schema/schema.graphql"),
    ),
    (
        "schema/extensions.graphql",
        include_str!("../../../demos/pet-demo/schema/extensions.graphql"),
    ),
    (
        "schema/scalars.graphql",
        include_str!("../../../demos/pet-demo/schema/scalars.graphql"),
    ),
];

fn roots(files: &[(&str, &str)]) -> Result<RootOperationTypes, RootOperationTypesError> {
    root_operation_types(&db_with(files), CLIENT).clone()
}

#[test]
fn root_operation_types_follow_the_schema_definition_and_extensions() {
    assert_eq!(
        roots(&PET_DEMO_SCHEMA),
        RootOperationTypes {
            query: entity("Query"),
            mutation: entity("Mutation").wrap_some(),
            subscription: None,
        }
        .wrap_ok()
    );
    assert_eq!(
        roots(&[("a.graphql", "type Query { a: Int }")]),
        RootOperationTypes {
            query: entity("Query"),
            mutation: entity("Mutation").wrap_some(),
            subscription: entity("Subscription").wrap_some(),
        }
        .wrap_ok()
    );
    assert_eq!(
        roots(&[
            ("a.graphql", "schema { query: RootQuery }"),
            ("b.graphql", "extend schema { subscription: S }"),
        ]),
        RootOperationTypes {
            query: entity("RootQuery"),
            mutation: None,
            subscription: entity("S").wrap_some(),
        }
        .wrap_ok()
    );
    assert_eq!(
        roots(&[("a.graphql", "type Broken { a: ! } schema { query: Query }")]),
        RootOperationTypes {
            query: entity("Query"),
            mutation: None,
            subscription: None,
        }
        .wrap_ok()
    );
}

#[test]
fn a_broken_schema_definition_is_an_error_of_the_root_types() {
    let Err(RootOperationTypesError::DuplicateSchemaDefinition(duplicate)) = roots(&[
        ("b.graphql", "schema { query: B }"),
        ("a.graphql", "schema { query: A }"),
    ]) else {
        panic!("expected a duplicate schema definition");
    };
    assert_eq!(duplicate.first.path, path("a.graphql"));
    assert_eq!(duplicate.second.path, path("b.graphql"));
    assert!(matches!(
        roots(&[(
            "a.graphql",
            "schema { query: Q } extend schema { query: A }"
        )]),
        Err(RootOperationTypesError::DuplicateOperation(
            DuplicateOperation {
                operation: OperationType::Query,
                ..
            }
        ))
    ));
    assert!(matches!(
        roots(&[("a.graphql", "schema { mutation: M }")]),
        Err(RootOperationTypesError::MissingQuery(_))
    ));
    assert!(matches!(
        roots(&[("a.graphql", "schema { query: }")]),
        Err(RootOperationTypesError::Syntax(_))
    ));
    assert!(matches!(
        roots(&[("a.graphql", "extend schema { query: Q }")]),
        Err(RootOperationTypesError::ExtensionWithoutDefinition(_))
    ));
}

fn fetch_plan(db: &Db, name: &str) -> Result<GraphQLFetchPlan, GraphQLNotFetchable> {
    graphql_fetch_plan(db, CLIENT, entity(name)).clone()
}

/// The fetch plan of `name`, which the fixture makes a `node` fetch.
fn node_plan(db: &Db, name: &str) -> GraphQLNodeFetch {
    let Ok(GraphQLFetchPlan::Node(node)) = fetch_plan(db, name) else {
        panic!("the fixture fetches {name} through node");
    };
    node
}

/// The operation type of `name`'s plan, which the fixture makes a root fetch.
fn root_plan(db: &Db, name: &str) -> GraphQLRootFetch {
    let Ok(GraphQLFetchPlan::Root(root)) = fetch_plan(db, name) else {
        panic!("the fixture makes {name} a root operation type");
    };
    root
}

fn refinement_name(refinement: &FetchRefinement) -> Option<EntityName> {
    match refinement {
        FetchRefinement::Exact => None,
        FetchRefinement::InlineFragment(target) => target.name.wrap_some(),
    }
}

fn id_argument() -> GraphQLFetchArgument {
    GraphQLFetchArgument {
        name: "id".intern().to(),
        variable_type: VariableType::NonNull(NonNullVariableType::Named(entity("ID"))),
    }
}

#[test]
fn the_pet_demo_entities_are_fetched_as_roots_or_through_node() {
    let db = db_with(&PET_DEMO_SCHEMA);
    let query = root_plan(&db, "Query");
    assert_eq!(query.operation_type, OperationType::Query);
    assert_eq!(query.entity.name, entity("Query"));
    assert_eq!(
        root_plan(&db, "Mutation").operation_type,
        OperationType::Mutation
    );
    let pet = node_plan(&db, "Pet");
    assert_eq!(pet.arguments, vec![id_argument()]);
    assert_eq!(pet.field_target.name, entity("Node"));
    assert_eq!(refinement_name(&pet.refinement), entity("Pet").wrap_some());
    assert_eq!(
        refinement_name(&node_plan(&db, "ICheckin").refinement),
        entity("ICheckin").wrap_some()
    );
    assert_eq!(node_plan(&db, "Node").refinement, FetchRefinement::Exact);
    assert_eq!(
        fetch_plan(&db, "PetStats"),
        GraphQLNotFetchable::NoFetchRoot(entity("PetStats")).wrap_err()
    );
    assert_eq!(
        fetch_plan(&db, "NewsfeedItem"),
        GraphQLNotFetchable::NotObjectOrInterface(entity("NewsfeedItem")).wrap_err()
    );
    assert_eq!(
        fetch_plan(&db, "String"),
        GraphQLNotFetchable::NotObjectOrInterface(entity("String")).wrap_err()
    );
    assert_eq!(
        fetch_plan(&db, "Nobody"),
        GraphQLNotFetchable::UnknownEntity(entity("Nobody")).wrap_err()
    );
}

const NODE_SCHEMA: &str = "interface Node { id: ID! } type Pet implements Node { id: ID! } \
    type PetStats { weight: Int } type Viewer { id: ID! } type Mutation { a: Int } \
    union Searchable = Pet";

#[test]
fn a_broken_node_field_fails_node_fetches_only() {
    for (node, expected) in [
        (
            "node(id: ID!): [Node]",
            GraphQLNotFetchable::NodeField(NodeFieldError::ReturnsList(entity("Query"))),
        ),
        (
            "node(id: ID!): Viewer",
            GraphQLNotFetchable::NodeField(NodeFieldError::TypeMismatch(NodeFieldTypeMismatch {
                query_root: entity("Query"),
                returns: entity("Viewer"),
                entity: entity("Pet"),
            })),
        ),
        (
            "other: Int",
            GraphQLNotFetchable::NodeField(NodeFieldError::NotFound(entity("Query"))),
        ),
    ] {
        let query = format!("type Query {{ {node} }}");
        let db = db_with(&[("a.graphql", NODE_SCHEMA), ("b.graphql", query.as_str())]);
        assert_eq!(fetch_plan(&db, "Pet"), expected.wrap_err(), "{node}");
        assert_eq!(
            root_plan(&db, "Query").operation_type,
            OperationType::Query,
            "{node}"
        );
        assert_eq!(
            root_plan(&db, "Mutation").operation_type,
            OperationType::Mutation,
            "{node}"
        );
    }
}

#[test]
fn node_may_return_a_union_containing_the_entity() {
    let db = db_with(&[
        ("a.graphql", NODE_SCHEMA),
        ("b.graphql", "type Query { node(id: ID!): Searchable }"),
    ]);
    let pet = node_plan(&db, "Pet");
    assert_eq!(pet.field_target.name, entity("Searchable"));
    assert_eq!(refinement_name(&pet.refinement), entity("Pet").wrap_some());
}

#[test]
fn a_node_argument_with_a_default_is_nullable() {
    let db = db_with(&[
        ("a.graphql", NODE_SCHEMA),
        (
            "b.graphql",
            "type Query { node(id: ID!, hint: Int = 0, ids: [ID!]): Node }",
        ),
    ]);
    assert_eq!(
        node_plan(&db, "Pet").arguments,
        vec![
            id_argument(),
            GraphQLFetchArgument {
                name: "hint".intern().to(),
                variable_type: VariableType::Nullable(NonNullVariableType::Named(entity("Int"))),
            },
            GraphQLFetchArgument {
                name: "ids".intern().to(),
                variable_type: VariableType::Nullable(NonNullVariableType::List(
                    VariableType::NonNull(NonNullVariableType::Named(entity("ID"))).boxed()
                )),
            },
        ]
    );
}

#[test]
fn a_query_root_field_returning_the_entity_changes_no_plan() {
    let db = db_with(&PET_DEMO_SCHEMA);
    assert!(matches!(
        fetch_plan(&db, "Pet"),
        Ok(GraphQLFetchPlan::Node(_))
    ));
    assert_eq!(
        fetch_plan(&db, "PetStats"),
        GraphQLNotFetchable::NoFetchRoot(entity("PetStats")).wrap_err()
    );
}

#[test]
fn the_default_root_types_are_fetchable_and_an_unnamed_mutation_is_not() {
    let db = db_with(&[(
        "a.graphql",
        "type Query { a: Int } type Subscription { petAdded: Int }",
    )]);
    assert_eq!(
        root_plan(&db, "Subscription").operation_type,
        OperationType::Subscription
    );
    let db = db_with(&[(
        "a.graphql",
        "schema { query: Query } type Query { a: Int } type Mutation { a: Int }",
    )]);
    assert_eq!(
        fetch_plan(&db, "Mutation"),
        GraphQLNotFetchable::NoFetchRoot(entity("Mutation")).wrap_err()
    );
}

#[test]
fn a_syntax_error_in_the_query_root_fails_node_fetches_and_not_other_roots() {
    let db = db_with(&[
        ("a.graphql", NODE_SCHEMA),
        ("b.graphql", "type Query { node(id: ID!): ! }"),
    ]);
    assert!(matches!(
        fetch_plan(&db, "Pet"),
        Err(GraphQLNotFetchable::NodeField(NodeFieldError::Selectable(
            _
        )))
    ));
    assert_eq!(
        root_plan(&db, "Mutation").operation_type,
        OperationType::Mutation
    );
}

const SOURCE_FILE: FileMembership = FileMembership {
    source_file: IncludeFile::Include,
    schema_file: IncludeFile::Skip,
};

const PET_DEMO_SOURCES: [(&str, &str); 10] = [
    (
        "src/HomeRoute.ts",
        include_str!("../../../demos/pet-demo/src/HomeRoute.ts"),
    ),
    (
        "src/Owner.ts",
        include_str!("../../../demos/pet-demo/src/Owner.ts"),
    ),
    (
        "src/Pet.ts",
        include_str!("../../../demos/pet-demo/src/Pet.ts"),
    ),
    (
        "src/PetBestFriendCard.ts",
        include_str!("../../../demos/pet-demo/src/PetBestFriendCard.ts"),
    ),
    (
        "src/PetCheckinsCard.ts",
        include_str!("../../../demos/pet-demo/src/PetCheckinsCard.ts"),
    ),
    (
        "src/PetDetailRoute.ts",
        include_str!("../../../demos/pet-demo/src/PetDetailRoute.ts"),
    ),
    (
        "src/PetStatsCard.ts",
        include_str!("../../../demos/pet-demo/src/PetStatsCard.ts"),
    ),
    (
        "src/PetSummaryCard.ts",
        include_str!("../../../demos/pet-demo/src/PetSummaryCard.ts"),
    ),
    (
        "src/SetTagline.ts",
        include_str!("../../../demos/pet-demo/src/SetTagline.ts"),
    ),
    (
        "src/SpeciesRoute.ts",
        include_str!("../../../demos/pet-demo/src/SpeciesRoute.ts"),
    ),
];

/// A database with `schema` as schema files and `sources` as source files, every file
/// inserted before the first lookup.
fn project_db(schema: &[(&str, &str)], sources: &[(&str, &str)]) -> Db {
    let mut db = db_with(schema);
    for (file, contents) in sources {
        db.insert_disk_file(path(file), SOURCE_FILE, contents.dereference().to_owned());
    }
    db
}

/// The pet demo's sources with `replaced` substituted for the source of the same path and
/// `added` beside them.
fn pet_demo_sources<'a>(
    replaced: &[(&'a str, &'a str)],
    added: &[(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    PET_DEMO_SOURCES
        .iter()
        .map(|(file, contents)| {
            replaced
                .iter()
                .find(|(replaced_file, _)| replaced_file == file)
                .copied()
                .unwrap_or((file, contents))
        })
        .chain(added.iter().copied())
        .collect()
}

/// Each validation error message of `file` and the text its span covers.
fn validation_errors(db: &Db, file: &str, contents: &str) -> Vec<(String, String)> {
    iso_literal_validation_errors_in_file(db, CLIENT, path(file))
        .clone()
        .expect("the test inserted the file")
        .into_iter()
        .map(|error| {
            (
                error.item.to_string(),
                contents[error.location.as_usize_range()].to_owned(),
            )
        })
        .collect()
}

#[test]
fn the_pet_demo_has_no_validation_errors() {
    let db = project_db(&PET_DEMO_SCHEMA, &PET_DEMO_SOURCES);
    for (file, contents) in PET_DEMO_SOURCES {
        assert_eq!(validation_errors(&db, file, contents), vec![], "{file}");
    }
}

#[test]
fn an_entrypoint_whose_parent_cannot_be_fetched_is_an_error_on_the_parent() {
    let cases = [
        (
            "iso(`field PetStats.Card { weight }`) iso(`entrypoint PetStats.Card`)",
            "`PetStats` cannot be fetched: it is not a root operation type and does not implement `Node`.",
            "PetStats",
        ),
        (
            "iso(`entrypoint Owner.OwnerBadge`)",
            "`Owner` is declared in source files; only entities from the schema can parent an entrypoint.",
            "Owner",
        ),
        (
            "iso(`entrypoint Query.pets`)",
            "`Query.pets` comes from the schema; an entrypoint names a `field` declaration.",
            "pets",
        ),
    ];
    for (contents, message, covered) in cases {
        let db = project_db(
            &PET_DEMO_SCHEMA,
            &pet_demo_sources(&[], &[("src/a.ts", contents)]),
        );
        assert_eq!(
            validation_errors(&db, "src/a.ts", contents),
            (message.to_owned(), covered.to_owned()).wrap_vec(),
            "{contents}"
        );
    }
}

#[test]
fn a_broken_node_field_is_an_error_on_node_fetched_entrypoints_only() {
    const SUMMARY: &str = "iso(`entrypoint Pet.PetSummaryCard`)";
    let db = project_db(
        &PET_DEMO_SCHEMA,
        &pet_demo_sources(&[], &[("src/a.ts", SUMMARY)]),
    );
    assert_eq!(validation_errors(&db, "src/a.ts", SUMMARY), vec![]);

    let schema = PET_DEMO_SCHEMA[0]
        .1
        .replace("node(id: ID!): Node", "node(id: ID!): [Node]");
    let db = project_db(
        &[
            (PET_DEMO_SCHEMA[0].0, schema.as_str()),
            PET_DEMO_SCHEMA[1],
            PET_DEMO_SCHEMA[2],
        ],
        &pet_demo_sources(&[], &[("src/a.ts", SUMMARY)]),
    );
    assert_eq!(
        validation_errors(&db, "src/a.ts", SUMMARY),
        (
            "`Query.node` returns a list; it returns one value.".to_owned(),
            "Pet".to_owned()
        )
            .wrap_vec()
    );
    assert_eq!(
        validation_errors(&db, "src/HomeRoute.ts", PET_DEMO_SOURCES[0].1),
        vec![]
    );
}
