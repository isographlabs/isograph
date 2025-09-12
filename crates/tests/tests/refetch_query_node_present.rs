use std::{fs, path::PathBuf};

use common_lang_types::{ClientScalarSelectableName, CurrentWorkingDirectory, SelectableName, ServerObjectEntityName};
use graphql_network_protocol::GraphQLNetworkProtocol;
use intern::string_key::Intern;
use isograph_compiler::{
    create_schema::create_schema,
    get_validated_schema::process_iso_literals_for_schema,
    source_files::initialize_sources,
};
use isograph_config::{
    absolute_and_relative_paths,
    compilation_options::{CompilerConfig, CompilerConfigOptions, GenerateFileExtensionsOption},
};
use isograph_schema::IsographDatabase;

fn write_file(path: &PathBuf, contents: &str) {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).expect("failed to create directories");
    }
    fs::write(path, contents).expect("failed to write file");
}

#[test]
fn refetch_node_present_generates_refetch_field_on_object() {
    // Create a temp workspace
    let tmp_root = std::env::temp_dir().join("isograph-refetch-node-present-test");
    let _ = fs::remove_dir_all(&tmp_root);
    fs::create_dir_all(&tmp_root).expect("failed to create tmp root");

    // Project structure
    let project_root = tmp_root.join("project");
    let schema_path = project_root.join("schema.graphql");
    let artifact_dir = project_root.join("__isograph");
    fs::create_dir_all(&artifact_dir).expect("failed to create artifact dir");

    // Schema with Node implementer AND Query.node defined
    let schema = r#"
schema { query: Query }

interface Node { id: ID! }

type Item implements Node { id: ID!, name: String }

type Query {
  node(id: ID!): Node
}
"#;
    write_file(&schema_path, schema);

    // Prepare DB and config
    let mut db: IsographDatabase<GraphQLNetworkProtocol> = Default::default();
    let cwd_str = tmp_root
        .canonicalize()
        .expect("canonicalize cwd")
        .to_string_lossy()
        .to_string();
    db.set(CurrentWorkingDirectory(cwd_str.intern().into()));

    let mut options = CompilerConfigOptions::default();
    options.no_babel_transform = true;
    options.include_file_extensions_in_import_statements =
        GenerateFileExtensionsOption::IncludeExtensionsInFileImports;

    let config = CompilerConfig {
        config_location: tmp_root.join("isograph.config.json"),
        project_root: project_root.clone(),
        artifact_directory: absolute_and_relative_paths(
            db.get_current_working_directory(),
            artifact_dir.clone(),
        ),
        schema: absolute_and_relative_paths(db.get_current_working_directory(), schema_path),
        schema_extensions: vec![],
        options,
    };
    db.set(config);

    // Initialize and compile
    initialize_sources(&mut db).expect("initialize_sources should succeed");
    let (schema, unprocessed) =
        create_schema::<GraphQLNetworkProtocol>(&db).expect("create_schema should not error");
    let (schema, _stats) =
        process_iso_literals_for_schema::<GraphQLNetworkProtocol>(&db, schema, unprocessed)
            .expect("process_iso_literals_for_schema should not error");

    // Assert that __refetch client scalar is registered on Item
    let item: ServerObjectEntityName = "Item".intern().into();
    let refetch_name: ClientScalarSelectableName = "__refetch".intern().into();
    assert!(
        schema
            .client_scalar_selectables
            .contains_key(&(item, refetch_name)),
        "expected __refetch to be generated for Item"
    );

    // Also validate that Item's available selectables include __refetch
    let has_refetch_in_selectables = schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(&item)
        .and_then(|info| info.selectables.get(&SelectableName::from("__refetch".intern())))
        .is_some();
    assert!(
        has_refetch_in_selectables,
        "expected __refetch to appear in Item.selectables"
    );
}

