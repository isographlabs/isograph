use std::path::Path;

use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, DefinitionLocation, FileMembership, IncludeFile, IsographDb,
    NetworkProtocol, ProtocolSelectable, UnionWrapper, Wrapper, WrapperVariant,
};
use isograph_extract_typescript::TypeScriptHostLanguage;
use prelude::Postfix;
use span::Span;

use crate::{
    EntityLookupError, GraphQLCompositeData, GraphQLEntityData, GraphQLNetworkProtocol,
    GraphQLSelectableData, GraphQLSelectableLookupError, SelectableLookupError, entity_named,
    graphql_entity, graphql_selectable, graphql_selectables, parsed_schema_file, schema_file_paths,
    selectable_named,
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
