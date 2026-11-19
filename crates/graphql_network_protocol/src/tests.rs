use std::path::Path;

use common_lang_types::{EntityName, RelativePathToSourceFile, SelectableName};
use graphql_schema_parser::OperationType;
use intern::string_key::Intern;
use isograph_compiler::{
    ClientId, CompilationProfile, CompositeShape, Concreteness, DefinitionLocation, EntityKind,
    EntityShape, FileMembership, IdentitySelection, IncludeFile, IsographDb, NetworkProtocol,
    ProtocolSelectable, RootEntities, ScalarKind, ScalarValueKind, SelectableShape, UnionWrapper,
    Wrapper, WrapperVariant, compiler_errors_in_file, iso_literal_validation_errors_in_file,
    schema_file_paths,
};
use isograph_extract_typescript::TypeScriptHostLanguage;
use prelude::Postfix;
use span::Span;

use crate::{
    DuplicateOperation, EntityLookupError, GraphQLCompositeData, GraphQLEntityData,
    GraphQLFetchPlan, GraphQLNetworkProtocol, GraphQLNotFetchable, GraphQLSelectableData,
    GraphQLSelectableLookupError, IdField, RootOperationTypes, RootOperationTypesError,
    SelectableLookupError, entity_named, graphql_entity, graphql_fetch_plan,
    graphql_schema_file_errors, graphql_selectable, graphql_selectables, parsed_schema_file,
    root_operation_types, scalar_kind, selectable_named,
};

struct GraphQLProfile;

impl CompilationProfile for GraphQLProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = GraphQLNetworkProtocol;

    fn custom_scalar_types() -> std::collections::BTreeMap<
        common_lang_types::EntityName,
        <Self::HostLanguage as isograph_compiler::HostLanguage>::CustomScalarType,
    > {
        std::collections::BTreeMap::new()
    }
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
fn schema_file_paths_are_disk_paths_and_the_clients_editor_buffer_paths_sorted() {
    let mut db = db_with(&[("b.graphql", ""), ("a.graphql", "")]);
    assert_eq!(
        schema_file_paths(&db, CLIENT).clone(),
        vec![path("a.graphql"), path("b.graphql")]
    );
    db.set_editor_buffer(CLIENT, path("c.graphql"), SCHEMA_FILE, String::new());
    db.set_editor_buffer(CLIENT, path("a.graphql"), SCHEMA_FILE, String::new());
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
    db.set_editor_buffer(
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
fn a_failed_schema_definition_is_one_schema_file_error_on_its_span() {
    let text = "type A { id: ID! }\ntype { x: Int }\nscalar ID";
    let db = db_with(&[("a.graphql", text)]);
    let errors = graphql_schema_file_errors(&db, CLIENT, path("a.graphql"))
        .clone()
        .expect("the test inserted the file");
    assert_eq!(
        errors
            .iter()
            .map(|error| &text[error.location.as_usize_range()])
            .collect::<Vec<_>>(),
        vec!["{"]
    );
    assert_eq!(
        graphql_schema_file_errors(&db, CLIENT, path("b.graphql")).clone(),
        None
    );
    let clean = db_with(&[("a.graphql", "type A { id: ID! }")]);
    assert_eq!(
        graphql_schema_file_errors(&clean, CLIENT, path("a.graphql")).clone(),
        Vec::new().wrap_some()
    );
}

#[test]
fn an_id_field_whose_type_is_not_id_non_null_is_an_error_on_its_type() {
    let text = "type A { id: String }\ntype B { id: ID }\ninterface C { id: [ID!] }\nextend type D { id: Int }\ntype E { id: ID! }\ninput F { id: Int }\ntype D { x: Int }";
    let db = db_with(&[("a.graphql", text)]);
    assert_eq!(
        graphql_schema_file_errors(&db, CLIENT, path("a.graphql"))
            .clone()
            .expect("the test inserted the file")
            .iter()
            .map(|error| (
                error.item.to_string(),
                &text[error.location.as_usize_range()]
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "The `id` field on `A` must have type `ID!`.".to_owned(),
                "String"
            ),
            (
                "The `id` field on `B` must have type `ID!`.".to_owned(),
                "ID"
            ),
            (
                "The `id` field on `C` must have type `ID!`.".to_owned(),
                "[ID!]"
            ),
            (
                "The `id` field on `D` must have type `ID!`.".to_owned(),
                "Int"
            ),
        ]
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
fn an_editor_buffer_that_breaks_a_type_breaks_it_for_that_client_only() {
    let mut db = db_with(&[("a.graphql", "type User { id: ID! }")]);
    db.set_editor_buffer(
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
    assert_eq!(data("ID"), GraphQLEntityData::Scalar(entity("ID")));
    assert_eq!(data("E"), GraphQLEntityData::Enum);
    assert_eq!(data("I"), GraphQLEntityData::InputObject);
    assert_eq!(data("U"), GraphQLEntityData::Union);
    assert_eq!(
        data("Node"),
        GraphQLEntityData::Interface(GraphQLCompositeData {
            interfaces: vec![],
            id_field: IdField::Present,
        })
    );
    assert_eq!(
        data("T"),
        GraphQLEntityData::Object(GraphQLCompositeData {
            interfaces: vec![entity("Node"), entity("Other")],
            id_field: IdField::Present,
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
fn id_field_is_present_only_for_a_field_id_of_type_non_null_id() {
    let schema = "scalar ID scalar String \
        type Present { id: ID! } type Nullable { id: ID } type Text { id: String! } \
        type List { id: [ID!]! } type Missing { name: String } type Duplicated { id: ID! id: ID! } \
        interface Interface { id: ID! } type Extended { name: String } \
        extend type Extended { id: ID! }";
    let db = db_with(&[("a.graphql", schema)]);
    let id_field = |name: &str| match graphql_entity(&db, CLIENT, entity(name))
        .clone()
        .expect("the fixture parses")
        .expect("the fixture defines the type")
        .data
    {
        GraphQLEntityData::Object(composite) | GraphQLEntityData::Interface(composite) => {
            composite.id_field.wrap_some()
        }
        GraphQLEntityData::Scalar(_)
        | GraphQLEntityData::Enum
        | GraphQLEntityData::InputObject
        | GraphQLEntityData::Union => None,
    };
    assert_eq!(id_field("Present"), IdField::Present.wrap_some());
    assert_eq!(id_field("Interface"), IdField::Present.wrap_some());
    assert_eq!(id_field("Extended"), IdField::Present.wrap_some());
    for absent in ["Nullable", "Text", "List", "Missing", "Duplicated"] {
        assert_eq!(id_field(absent), IdField::Absent.wrap_some(), "{absent}");
    }
}

#[test]
fn the_graphql_shapes_name_typename_and_id() {
    let id: SelectableName = selectable("id");
    let composite = |id_field| GraphQLCompositeData {
        interfaces: vec![],
        id_field,
    };
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::Object(composite(
            IdField::Present
        ))),
        EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Concrete,
            identity: IdentitySelection::Select(id)
        })
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::Interface(composite(
            IdField::Present
        ))),
        EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Abstract,
            identity: IdentitySelection::Select(id)
        })
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::Object(composite(
            IdField::Absent
        ))),
        EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Concrete,
            identity: IdentitySelection::Omit
        })
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::Union),
        EntityShape::Composite(CompositeShape {
            concreteness: Concreteness::Abstract,
            identity: IdentitySelection::Omit
        })
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::Scalar(entity("ID"))),
        EntityShape::Leaf
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::Enum),
        EntityShape::Leaf
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_shape(&GraphQLEntityData::InputObject),
        EntityShape::Input
    );
    assert_eq!(
        GraphQLNetworkProtocol::selectable_shape(&GraphQLSelectableData::Field),
        SelectableShape::Direct
    );
    assert_eq!(
        GraphQLNetworkProtocol::selectable_shape(&GraphQLSelectableData::Refinement),
        SelectableShape::Refinement
    );
    assert_eq!(
        GraphQLNetworkProtocol::discriminator(),
        selectable("__typename")
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
    assert_eq!(
        pets.definition,
        located("a.graphql", schema, "pets").wrap_some()
    );
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
    assert_eq!(
        as_a.definition,
        located_in("a.graphql", schema, "A {", "A").wrap_some()
    );
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
        ["asA", "asB", "asC", "__typename"]
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
        ["id", "asOwner", "asPet", "__typename"]
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
        ["asB", "__typename"]
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

/// The operation type of `name`'s plan, which the fixture makes a root fetch.
fn root_plan(db: &Db, name: &str) -> OperationType {
    fetch_plan(db, name)
        .expect("the fixture makes this entity a root operation type")
        .operation_type
}

/// The root entities of `name`'s plan, which the fixture makes a root fetch.
fn plan_roots(db: &Db, name: &str) -> RootEntities {
    fetch_plan(db, name)
        .expect("the fixture makes this entity a root operation type")
        .roots
}

fn root_entities(operation: &str, others: &[&str]) -> RootEntities {
    RootEntities {
        operation: entity(operation),
        others: others.iter().map(|other| entity(other)).collect(),
    }
}

#[test]
fn only_the_root_operation_types_of_the_pet_demo_are_fetchable() {
    let db = db_with(&PET_DEMO_SCHEMA);
    assert_eq!(
        fetch_plan(&db, "Query"),
        GraphQLFetchPlan {
            operation_type: OperationType::Query,
            roots: root_entities("Query", &["Mutation"]),
        }
        .wrap_ok()
    );
    assert_eq!(
        fetch_plan(&db, "Mutation"),
        GraphQLFetchPlan {
            operation_type: OperationType::Mutation,
            roots: root_entities("Mutation", &["Query"]),
        }
        .wrap_ok()
    );
    for not_root in [
        "Pet",
        "PetStats",
        "Node",
        "ICheckin",
        "Viewer",
        "NewsfeedItem",
        "String",
        "PetSpecies",
        "SetPetTaglineParams",
    ] {
        assert_eq!(
            fetch_plan(&db, not_root),
            GraphQLNotFetchable::NotRootOperationType(entity(not_root)).wrap_err(),
            "{not_root}"
        );
    }
    assert_eq!(
        fetch_plan(&db, "Nobody"),
        GraphQLNotFetchable::UnknownEntity(entity("Nobody")).wrap_err()
    );
}

#[test]
fn the_default_root_types_are_fetchable_and_an_unnamed_mutation_is_not() {
    let db = db_with(&[(
        "a.graphql",
        "type Query { a: Int } type Subscription { petAdded: Int }",
    )]);
    assert_eq!(root_plan(&db, "Subscription"), OperationType::Subscription);
    assert_eq!(
        plan_roots(&db, "Subscription"),
        root_entities("Subscription", &["Query", "Mutation"])
    );
    let db = db_with(&[(
        "a.graphql",
        "schema { query: Query } type Query { a: Int } type Mutation { a: Int }",
    )]);
    assert_eq!(
        fetch_plan(&db, "Mutation"),
        GraphQLNotFetchable::NotRootOperationType(entity("Mutation")).wrap_err()
    );
}

#[test]
fn a_root_that_is_not_an_object_is_not_fetchable() {
    let db = db_with(&[(
        "a.graphql",
        "schema { query: Q } union Q = A type A { a: Int }",
    )]);
    assert_eq!(
        fetch_plan(&db, "Q"),
        GraphQLNotFetchable::NotObject(entity("Q")).wrap_err()
    );
}

#[test]
fn a_syntax_error_in_the_query_root_leaves_the_other_roots_fetchable() {
    let db = db_with(&[
        ("a.graphql", "type Mutation { a: Int }"),
        ("b.graphql", "type Query { node(id: ID!): ! }"),
    ]);
    assert!(matches!(
        fetch_plan(&db, "Query"),
        Err(GraphQLNotFetchable::Entity(_))
    ));
    assert_eq!(root_plan(&db, "Mutation"), OperationType::Mutation);
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

/// Each compiler error message of `file` and the text its span covers.
fn compiler_errors(db: &Db, file: &str, contents: &str) -> Vec<(String, String)> {
    compiler_errors_in_file(db, CLIENT, path(file))
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

/// The pet demo with `added` source files.
fn pet_demo_with(added: &[(&str, &str)]) -> Db {
    project_db(&PET_DEMO_SCHEMA, &pet_demo_sources(&[], added))
}

fn messages(errors: &[(&str, &str)]) -> Vec<(String, String)> {
    errors
        .iter()
        .map(|(message, covered)| {
            (
                message.dereference().to_owned(),
                covered.dereference().to_owned(),
            )
        })
        .collect()
}

#[test]
fn the_pet_demo_has_no_compiler_errors() {
    let db = project_db(&PET_DEMO_SCHEMA, &PET_DEMO_SOURCES);
    for (file, contents) in PET_DEMO_SOURCES {
        assert_eq!(compiler_errors(&db, file, contents), vec![], "{file}");
    }
}

#[test]
fn an_inline_cycle_is_a_generation_error_of_each_field_at_its_name() {
    let contents = "export const a = iso(`field Pet.a { b }`)(() => 1)\nexport const b = iso(`field Pet.b { a }`)(() => 1)";
    let db = pet_demo_with(&[("src/a.ts", contents)]);
    assert_eq!(
        compiler_errors(&db, "src/a.ts", contents),
        messages(&[
            ("These fields inline each other: Pet.a -> Pet.b.", "a"),
            ("These fields inline each other: Pet.b -> Pet.a.", "b"),
        ])
    );
}

#[test]
fn a_field_inlining_a_broken_field_in_another_file_has_no_error_of_its_own() {
    let broken = "export const broken = iso(`field Pet.broken { a }`)(() => 1)\nexport const a = iso(`field Pet.a { broken }`)(() => 1)";
    let user = "export const user = iso(`field Pet.user { broken }`)(() => 1)";
    let db = pet_demo_with(&[("src/broken.ts", broken), ("src/user.ts", user)]);
    assert_eq!(compiler_errors(&db, "src/user.ts", user), vec![]);
    assert_eq!(compiler_errors(&db, "src/broken.ts", broken).len(), 2);
}

#[test]
fn a_file_with_a_validation_error_reports_no_generation_error() {
    let contents = "export const a = iso(`field Pet.a { b, nope }`)(() => 1)\nexport const b = iso(`field Pet.b { a }`)(() => 1)";
    let db = pet_demo_with(&[("src/a.ts", contents)]);
    assert_eq!(
        compiler_errors(&db, "src/a.ts", contents),
        messages(&[("Field `Pet.nope` is not defined.", "nope")])
    );
}

#[test]
fn an_entrypoint_that_selects_nothing_is_an_error_at_its_name() {
    let contents =
        "export const Only = iso(`field Query.Only`)(() => 1)\niso(`entrypoint Query.Only`)";
    let db = pet_demo_with(&[("src/a.ts", contents)]);
    assert_eq!(
        compiler_errors(&db, "src/a.ts", contents),
        messages(&[("The entrypoint selects nothing the server returns.", "Only")])
    );
}

#[test]
fn a_selection_is_checked_against_its_targets_shape() {
    let schema = (
        "schema/input.graphql",
        "extend type Query { badInput: SetPetTaglineParams }",
    );
    let contents =
        "export const a = iso(`field Query.a { pets { stats, id { x } }, badInput }`)(() => 1)";
    let schema_files: Vec<(&str, &str)> = PET_DEMO_SCHEMA.iter().copied().chain([schema]).collect();
    let db = project_db(
        &schema_files,
        &pet_demo_sources(&[], &[("src/a.ts", contents)]),
    );
    assert_eq!(
        compiler_errors(&db, "src/a.ts", contents),
        messages(&[
            (
                "`Pet.stats` returns an entity with selectables and needs a selection set.",
                "stats"
            ),
            (
                "`Pet.id` returns a leaf and takes no selection set.",
                "{ x }"
            ),
            (
                "`SetPetTaglineParams` is an input type, which cannot be selected.",
                "badInput"
            ),
        ])
    );
}

#[test]
fn a_response_name_twice_in_one_selection_set_is_an_error_on_the_second() {
    let cases = [
        ("pets { firstName, firstName }", vec!["firstName"]),
        ("pets { a: firstName, a: lastName }", vec!["a"]),
        ("pets { firstName, f: firstName }", vec![]),
        (
            "pets { firstName, potential_new_best_friends { firstName } }",
            vec![],
        ),
        (
            "pets { firstName, firstName, firstName }",
            vec!["firstName", "firstName"],
        ),
    ];
    for (body, covered) in cases {
        let contents = format!("export const a = iso(`field Query.a {{ {body} }}`)(() => 1)");
        let db = pet_demo_with(&[("src/a.ts", contents.as_str())]);
        let errors = compiler_errors(&db, "src/a.ts", contents.as_str());
        assert_eq!(
            errors
                .iter()
                .map(|(_, covered)| covered.as_str())
                .collect::<Vec<_>>(),
            covered,
            "{body}"
        );
        assert!(
            errors.iter().all(|(message, covered)| *message
                == format!("A field with name or alias `{covered}` has already been defined")),
            "{body}"
        );
    }
}

#[test]
fn a_field_parent_and_pointer_target_must_be_composite() {
    let leaf_target = "Invalid client pointer target type. `String` is a leaf. You are attempting to define a pointer to it. In order to do so, the type must be an object, interface or union.";
    let input_target = "Invalid client pointer target type. `SetPetTaglineParams` is an input type. You are attempting to define a pointer to it. In order to do so, the type must be an object, interface or union.";
    let leaf_parent = "Invalid parent type. `String` is a leaf. You are attempting to define a client field on it. In order to do so, the parent object must be an object, interface or union.";
    let cases = [
        (
            "field Pet.p to Pett",
            ("Entity `Pett` is not defined.", "Pett").wrap_vec(),
        ),
        ("field Pet.p to String", (leaf_target, "String").wrap_vec()),
        (
            "field Pet.p to [String!]",
            (leaf_target, "String").wrap_vec(),
        ),
        (
            "field Pet.p to SetPetTaglineParams",
            (input_target, "SetPetTaglineParams").wrap_vec(),
        ),
        ("field Pet.p to [Pet]", vec![]),
        ("field Pet.p to Owner", vec![]),
        ("field String.p", (leaf_parent, "String").wrap_vec()),
    ];
    for (declaration, expected) in cases {
        let contents = format!("export const p = iso(`{declaration}`)(() => 1)");
        let db = pet_demo_with(&[("src/a.ts", contents.as_str())]);
        assert_eq!(
            compiler_errors(&db, "src/a.ts", contents.as_str()),
            messages(&expected),
            "{declaration}"
        );
    }
}

/// The compiler errors of a `field` declaration whose literal is `literal`, beside the pet demo
/// and a schema extension with a list argument.
fn argument_errors_of(literal: &str) -> Vec<(String, String)> {
    let contents = format!("export const a = iso(`{literal}`)(() => 1)");
    let schema = (
        "schema/tagged.graphql",
        "extend type Query { tagged(tags: [String!]!): [Pet!]! }",
    );
    let schema_files: Vec<(&str, &str)> = PET_DEMO_SCHEMA.iter().copied().chain([schema]).collect();
    let db = project_db(
        &schema_files,
        &pet_demo_sources(&[], &[("src/a.ts", contents.as_str())]),
    );
    compiler_errors(&db, "src/a.ts", contents.as_str())
}

#[test]
fn schema_field_arguments_are_checked_against_the_schema() {
    let query =
        |body: &str| format!("field Query.a($id: ID!, $name: String!, $n: Int) {{ {body} }}");
    let cases = [
        (
            "pet { id }",
            vec![(
                "`Query.pet` requires the argument `id`, and this selection does not pass it.",
                "pet",
            )],
        ),
        (
            "pet(id: $id, petId: 1) { id }",
            vec![("`Query.pet` takes no argument `petId`.", "petId")],
        ),
        ("pets { checkins { id } }", vec![]),
        (
            "viewer { newsfeed(limit: 1) { asAdItem { id } } }",
            vec![(
                "`Viewer.newsfeed` requires the argument `skip`, and this selection does not pass it.",
                "newsfeed",
            )],
        ),
        (
            "viewer { newsfeed(limit: 1, skip: 0) { asAdItem(x: 1) { id } } }",
            vec![("`NewsfeedItem.asAdItem` takes no argument `x`.", "x")],
        ),
    ];
    for (body, expected) in cases {
        let literal = query(body);
        let errors = argument_errors_of(literal.as_str());
        assert_eq!(errors, messages(&expected), "{body}");
    }
}

#[test]
fn argument_values_are_type_checked() {
    let query = |body: &str| {
        format!("field Query.a($id: ID!, $name: String!, $n: Int, $ns: [Int!]) {{ {body} }}")
    };
    let cases = [
        (
            "pet(id: $name) { id }",
            vec![(
                "Mismatched type. Received $name, with type String!, expected ID!.",
                "$name",
            )],
        ),
        (
            "pet(id: $nope) { id }",
            vec![("This variable is not defined: $nope", "$nope")],
        ),
        (
            "petByName(name: 1) { id }",
            vec![(
                "Expected input of type String!, found an integer literal.",
                "1",
            )],
        ),
        (
            "pet(id: null) { id }",
            vec![("Expected non null input of type ID!, found null", "null")],
        ),
        ("pet(id: 1) { id }", vec![]),
        ("pet(id: \"1\") { id }", vec![]),
        ("pets { checkins(limit: $n) { id } }", vec![]),
        ("pets { checkins(limit: null) { id } }", vec![]),
        (
            "pets { checkins(limit: 3000000000) { id } }",
            vec![(
                "`3000000000` is outside GraphQL's 32-bit `Int` range.",
                "3000000000",
            )],
        ),
        (
            "pet(id: $n) { id }",
            vec![(
                "Mismatched type. Received $n, with type Int, expected ID!.",
                "$n",
            )],
        ),
        (
            "petsBySpecies(species: \"CAT\") { id }",
            vec![(
                "Expected input of type PetSpecies!, found a string literal.",
                "\"CAT\"",
            )],
        ),
        ("tagged(tags: \"a\") { id }", vec![]),
        (
            "tagged(tags: [\"a\", 1]) { id }",
            vec![(
                "Expected input of type String!, found an integer literal.",
                "1",
            )],
        ),
        ("tagged(tags: []) { id }", vec![]),
        (
            "tagged(tags: [[\"a\"]]) { id }",
            vec![(
                "Expected input of type String!, found a list literal.",
                "[\"a\"]",
            )],
        ),
        (
            "tagged(tags: $ns) { id }",
            vec![(
                "Mismatched type. Received $ns, with type [Int!], expected [String!]!.",
                "$ns",
            )],
        ),
    ];
    for (body, expected) in cases {
        let literal = query(body);
        assert_eq!(
            argument_errors_of(literal.as_str()),
            messages(&expected),
            "{body}"
        );
    }
}

#[test]
fn input_object_values_have_the_input_objects_fields() {
    let mutation = |input: &str| {
        format!(
            "field Mutation.b($id: ID!) {{ set_pet_tagline(input: {input}) {{ pet {{ id }} }} }}"
        )
    };
    let cases = [
        ("{ id: $id, tagline: \"x\" }", vec![]),
        (
            "{ id: $id }",
            vec![(
                "`SetPetTaglineParams` requires the field `tagline`, and this object does not have it.",
                "{ id: $id }",
            )],
        ),
        (
            "{ id: $id, tagline: \"x\", taglin: \"y\" }",
            vec![("`SetPetTaglineParams` has no field `taglin`.", "taglin")],
        ),
        (
            "{ id: $id, tagline: 1 }",
            vec![(
                "Expected input of type String!, found an integer literal.",
                "1",
            )],
        ),
        (
            "\"x\"",
            vec![(
                "Expected input of type SetPetTaglineParams!, found a string literal.",
                "\"x\"",
            )],
        ),
    ];
    for (input, expected) in cases {
        let literal = mutation(input);
        assert_eq!(
            argument_errors_of(literal.as_str()),
            messages(&expected),
            "{input}"
        );
    }
}

#[test]
fn a_client_field_selections_values_are_checked_against_its_variables() {
    let cases = [
        ("pets { PetCheckinsCard(limit: 5) }", vec![]),
        (
            "pets { PetCheckinsCard(limit: \"x\") }",
            vec![(
                "Expected input of type Int, found a string literal.",
                "\"x\"",
            )],
        ),
        (
            "pets { PetCheckinsCard(limt: 5) }",
            vec![(
                "`Pet.PetCheckinsCard` declares no variable `$limt`.",
                "limt",
            )],
        ),
    ];
    for (body, expected) in cases {
        let literal = format!("field Query.a {{ {body} }}");
        assert_eq!(
            argument_errors_of(literal.as_str()),
            messages(&expected),
            "{body}"
        );
    }
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
            "`PetStats` is not a root operation type; entrypoints must be on a root operation type.",
            "PetStats",
        ),
        (
            "iso(`entrypoint Pet.PetSummaryCard`)",
            "`Pet` is not a root operation type; entrypoints must be on a root operation type.",
            "Pet",
        ),
        (
            "iso(`field Node.Card { id }`) iso(`entrypoint Node.Card`)",
            "`Node` is not a root operation type; entrypoints must be on a root operation type.",
            "Node",
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
fn the_built_in_scalars_are_known_and_every_other_scalar_is_custom() {
    let known = |value_kind| ScalarKind::Known(value_kind);
    assert_eq!(
        scalar_kind(entity("String")),
        known(ScalarValueKind::String)
    );
    assert_eq!(scalar_kind(entity("ID")), known(ScalarValueKind::String));
    assert_eq!(scalar_kind(entity("Int")), known(ScalarValueKind::Number));
    assert_eq!(scalar_kind(entity("Float")), known(ScalarValueKind::Number));
    assert_eq!(
        scalar_kind(entity("Boolean")),
        known(ScalarValueKind::Boolean)
    );
    assert_eq!(
        scalar_kind(entity("Url")),
        ScalarKind::Custom(entity("Url"))
    );
}

#[test]
fn an_enum_is_a_string_scalar_and_a_union_is_an_object_of_its_members() {
    let db = db_with(&[(
        "a.graphql",
        "enum E { A } type A { id: ID! } type B { id: ID! } union U = A | B",
    )]);
    let data = |name: &str| {
        graphql_entity(&db, CLIENT, entity(name))
            .clone()
            .expect("the fixture parses")
            .expect("the fixture defines the type")
            .data
    };
    assert_eq!(
        GraphQLNetworkProtocol::entity_kind(data("E").reference()),
        EntityKind::Scalar(ScalarKind::Known(ScalarValueKind::String))
    );
    assert_eq!(
        GraphQLNetworkProtocol::entity_kind(data("U").reference()),
        EntityKind::Object
    );
    assert_eq!(
        GraphQLNetworkProtocol::concrete_types(&db, CLIENT, entity("U"), data("U").reference()),
        vec![entity("A"), entity("B")]
    );
    assert_eq!(
        GraphQLNetworkProtocol::concrete_types(&db, CLIENT, entity("A"), data("A").reference()),
        vec![entity("A")]
    );
}

#[test]
fn an_input_object_field_is_an_input_field_selectable() {
    let schema = "input Filter { and: [Filter!] } extend input Filter { name: String! }";
    let db = db_with(&[("a.graphql", schema)]);
    let and = field(&db, "Filter", "and");
    assert_eq!(
        and.target,
        nullable(WrapperVariant::List(
            Wrapper::Inner(entity("Filter")).boxed()
        ))
    );
    assert_eq!(and.data, GraphQLSelectableData::InputField);
    assert_eq!(
        field(&db, "Filter", "name").target,
        Wrapper::Inner(entity("String"))
    );
    assert_eq!(
        graphql_selectables(&db, CLIENT, entity("Filter"))
            .iter()
            .map(|(name, _)| name.dereference())
            .collect::<Vec<_>>(),
        vec![selectable("and"), selectable("name")]
    );
}

#[test]
fn an_input_field_defined_twice_is_an_error() {
    let db = db_with(&[("a.graphql", "input F { a: Int a: Int }")]);
    assert!(matches!(
        graphql_selectable(&db, CLIENT, entity("F"), selectable("a")).clone(),
        Err(GraphQLSelectableLookupError::DuplicateInputField(_))
    ));
}

mod field_types {
    use common_lang_types::{EntityNameAndSelectableName, SelectionName};
    use intern::string_key::Intern;
    use isograph_compiler::{
        ArtifactInnerType, ArtifactInputType, ArtifactObjectField, ArtifactObjectType, FieldTypes,
        FileMembership, GenerationError, IncludeFile, ScalarKind, ScalarValueKind, UnionWrapper,
        Wrapper, WrapperVariant, field_types,
    };
    use prelude::Postfix;

    use super::{CLIENT, Db, db_with, entity, path, selectable};

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };

    const SCHEMA: &str = "scalar String scalar ID scalar Int \
        type Pet { \"The pet's ID\" id: ID! firstName: String! nickname: String \
        friends: [Pet!]! checkins(limit: Int): [Pet!]! } \
        input Filter { and: [Filter!] name: String }";

    /// A database with `SCHEMA` and one source file whose literals are `literals`, each exported
    /// and called with a resolver.
    fn db_with_literals(literals: &[&str]) -> Db {
        let mut db = db_with(&[("schema.graphql", SCHEMA)]);
        let text = literals
            .iter()
            .enumerate()
            .map(|(index, literal)| {
                format!("export const f{index} = iso(`{literal}`)(() => null);\n")
            })
            .collect::<String>();
        db.insert_disk_file(path("src/a.ts"), SOURCE_FILE, text);
        db
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

    fn key(name: &str) -> SelectionName {
        name.intern().to()
    }

    fn string() -> ArtifactInnerType {
        ArtifactInnerType::Scalar(ScalarKind::Known(ScalarValueKind::String))
    }

    fn nullable<T>(variant: WrapperVariant<T>) -> Wrapper<T> {
        Wrapper::Union(UnionWrapper(vec![variant, WrapperVariant::Null]))
    }

    fn data_field<'a>(object: &'a ArtifactObjectType, name: &str) -> &'a ArtifactObjectField {
        object
            .0
            .iter()
            .find(|field| field.key == key(name))
            .expect("the fixture selects this key")
    }

    #[test]
    fn server_fields_keep_their_wrappers() {
        let db = db_with_literals(&["field Pet.f { firstName, nickname, friends { firstName } }"]);
        let data = types(&db, "Pet", "f").expect("the fixture is valid").data;
        assert_eq!(
            data_field(&data, "firstName").type_,
            Wrapper::Inner(string())
        );
        assert_eq!(
            data_field(&data, "nickname").type_,
            nullable(WrapperVariant::Inner(string()))
        );
        assert_eq!(
            data_field(&data, "friends").type_,
            Wrapper::List(
                Wrapper::Inner(ArtifactInnerType::Object(ArtifactObjectType(
                    ArtifactObjectField {
                        key: key("firstName"),
                        description: None,
                        type_: Wrapper::Inner(string()),
                    }
                    .wrap_vec()
                )))
                .boxed()
            )
        );
    }

    #[test]
    fn keys_are_aliases_in_selection_order_with_descriptions() {
        let db = db_with_literals(&["field Pet.f { n: nickname, id }"]);
        let data = types(&db, "Pet", "f").expect("the fixture is valid").data;
        assert_eq!(
            data.0.iter().map(|field| field.key).collect::<Vec<_>>(),
            vec![key("n"), key("id")]
        );
        assert_eq!(
            data_field(&data, "id").description,
            "The pet's ID"
                .intern()
                .to::<common_lang_types::DescriptionValue>()
                .wrap_some()
        );
        assert_eq!(data_field(&data, "n").description, None);
    }

    #[test]
    fn a_selected_client_field_is_its_output_type_and_is_selected() {
        let db = db_with_literals(&[
            "field Pet.f { firstName }",
            "field Pet.g { f, friends { f } }",
        ]);
        let types = types(&db, "Pet", "g").expect("the fixture is valid");
        assert_eq!(
            data_field(&types.data, "f").type_,
            Wrapper::Inner(ArtifactInnerType::ClientFieldOutput(named("Pet", "f")))
        );
        assert_eq!(types.selected, vec![named("Pet", "f"), named("Pet", "f")]);
    }

    #[test]
    fn a_selected_pointer_is_selected_and_neither_typed_nor_read() {
        let db = db_with_literals(&[
            "field Pet.f { firstName }",
            "field Pet.p to Pet { id }",
            "field Pet.g { id, p { f } }",
        ]);
        let types = types(&db, "Pet", "g").expect("the fixture is valid");
        assert_eq!(
            types
                .data
                .0
                .iter()
                .map(|field| field.key)
                .collect::<Vec<_>>(),
            key("id").wrap_vec()
        );
        assert_eq!(types.reader_ast.0.len(), 1);
        assert_eq!(types.selected, vec![named("Pet", "p"), named("Pet", "f")]);
    }

    #[test]
    fn malformed_selections_are_errors() {
        let db = db_with_literals(&[
            "field Pet.a { firstName { x } }",
            "field Pet.b { friends }",
            "field Pet.c",
            "field Pet.d { c { x } }",
            "field Pet.e { nope }",
        ]);
        assert!(matches!(
            types(&db, "Pet", "a"),
            Err(GenerationError::SelectionSetOnScalar(_))
        ));
        assert!(matches!(
            types(&db, "Pet", "b"),
            Err(GenerationError::MissingSelectionSet(_))
        ));
        assert!(matches!(
            types(&db, "Pet", "d"),
            Err(GenerationError::SelectionSetOnClientField(_))
        ));
        assert!(matches!(
            types(&db, "Pet", "e"),
            Err(GenerationError::Lookup(_))
        ));
        let c = types(&db, "Pet", "c").expect("a field with no selection set is valid");
        assert_eq!(c.data, ArtifactObjectType(Vec::new()));
        assert_eq!(c.parameters, None);
    }

    #[test]
    fn variables_are_parameters_and_input_objects_are_expanded_once() {
        let db = db_with_literals(&["field Pet.f($id: ID!, $limit: Int, $filter: Filter) { id }"]);
        let parameters = types(&db, "Pet", "f")
            .expect("the fixture is valid")
            .parameters
            .expect("the fixture declares variables");
        let types_by_name = parameters
            .parameters
            .iter()
            .map(|parameter| (parameter.name.to_string(), parameter.type_.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            types_by_name,
            vec![
                (
                    "id".to_owned(),
                    Wrapper::Inner(ArtifactInputType::Scalar(ScalarKind::Known(
                        ScalarValueKind::String
                    )))
                ),
                (
                    "limit".to_owned(),
                    nullable(WrapperVariant::Inner(ArtifactInputType::Scalar(
                        ScalarKind::Known(ScalarValueKind::Number)
                    )))
                ),
                (
                    "filter".to_owned(),
                    nullable(WrapperVariant::Inner(ArtifactInputType::InputObject(
                        entity("Filter")
                    )))
                ),
            ]
        );
        assert_eq!(
            parameters.input_objects.keys().copied().collect::<Vec<_>>(),
            vec![entity("Filter")]
        );
        let and = parameters.input_objects[&entity("Filter")]
            .iter()
            .find(|field| field.name == selectable("and"))
            .expect("Filter has `and`");
        assert_eq!(
            and.type_,
            nullable(WrapperVariant::List(
                Wrapper::Inner(ArtifactInputType::InputObject(entity("Filter"))).boxed()
            ))
        );
    }

    #[test]
    fn an_unrelated_edit_leaves_the_types_equal() {
        let mut db = db_with_literals(&["field Pet.f { firstName }"]);
        let before = types(&db, "Pet", "f");
        db.insert_disk_file(
            path("src/b.ts"),
            SOURCE_FILE,
            "export const g = iso(`field Pet.g { id }`)(() => null);\n".to_owned(),
        );
        assert_eq!(types(&db, "Pet", "f"), before);
    }
}

mod merged_selection;
mod query;
mod reader_ast;
