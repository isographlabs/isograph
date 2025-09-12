use std::{fs, path::PathBuf};

use common_lang_types::{CurrentWorkingDirectory, GeneratedFileHeader};
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
fn refetch_node_absent_does_not_error() {
    // Create a temp workspace
    let tmp_root = std::env::temp_dir().join("isograph-refetch-node-test");
    // Ensure a fresh directory for this test invocation
    let _ = fs::remove_dir_all(&tmp_root);
    fs::create_dir_all(&tmp_root).expect("failed to create tmp root");

    // Project structure
    let project_root = tmp_root.join("project");
    let schema_path = project_root.join("schema.graphql");
    let artifact_dir = project_root.join("__isograph");
    fs::create_dir_all(&artifact_dir).expect("failed to create artifact dir");

    // Minimal schema with Node implementer, but WITHOUT Query.node
    let schema = r#"
schema { query: Query }

interface Node { id: ID! }

type Item implements Node { id: ID!, name: String }

type ItemEdge { cursor: String!, node: Item }
type ItemConnection { edges: [ItemEdge], pageInfo: PageInfo! }

type PageInfo {
  endCursor: String
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
}

type Query {
  items(first: Int): ItemConnection
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
    options.generated_file_header = Some(GeneratedFileHeader("test".to_string()));

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

    // Initialize sources (reads schema from disk)
    initialize_sources(&mut db).expect("initialize_sources should succeed");

    // Run schema creation; this is where @exposeField(refetch via Query.node) is handled.
    let (schema, unprocessed) =
        create_schema::<GraphQLNetworkProtocol>(&db).expect("create_schema should not error");

    // Finish processing (there should be no errors here either)
    let _ = process_iso_literals_for_schema::<GraphQLNetworkProtocol>(&db, schema, unprocessed)
        .expect("process_iso_literals_for_schema should not error");
}

