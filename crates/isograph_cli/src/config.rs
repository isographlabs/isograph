use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug)]
pub(crate) struct CompilerConfig {
    /// The folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The folder where the compiler should create artifacts
    pub artifact_directory: PathBuf,
    /// The path to the GraphQL schema
    pub schema: PathBuf,
}

#[derive(Deserialize)]
pub(crate) struct ConfigFile {
    /// The relative path to the folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The relative path to the folder where the compiler should create artifacts
    pub artifact_directory: PathBuf,
    /// The relative path to the GraphQL schema
    pub schema: PathBuf,
}

impl CompilerConfig {
    // TODO this should return a Result
    pub(crate) fn create(config_location: Option<PathBuf>) -> Self {
        let mut config_location = config_location.expect("--config must be provided for now.");
        let config_contents =
            std::fs::read_to_string(&config_location).expect("Expected config to be found");

        let config_parsed: ConfigFile = serde_json::from_str(&config_contents)
            .unwrap_or_else(|e| panic!("Error parsing config. Error: {}", e));

        config_location.pop();
        let config_dir = config_location;

        std::fs::create_dir_all(config_dir.join(&config_parsed.artifact_directory))
            .expect("Unable to create artifact directory");

        CompilerConfig {
            project_root: config_dir
                .join(&config_parsed.project_root)
                .canonicalize()
                .expect("Unable to canonicalize project root"),
            artifact_directory: config_dir
                .join(&config_parsed.artifact_directory)
                .canonicalize()
                .expect("Unable to canonicalize artifact directory."),
            schema: config_dir
                .join(&config_parsed.schema)
                .canonicalize()
                .expect("Unable to canonicalize schema"),
        }
    }
}
