use isograph_config::IsographProjectConfig;
use isograph_config::{self, ConfigFileOptions};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use swc_common::plugin::metadata::TransformPluginMetadataContextKind;
use swc_core::{
    ecma::{ast::Program, visit::FoldWith},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use swc_isograph;
use tracing::debug;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WasmConfig {
    /// Unlike native env,  in WASM we can't use env::current_dir
    /// as well as `/cwd` alias. current_dir cannot resolve to actual path,
    /// `/cwd` alias won't expand to `real` path but only gives ACCESS to the cwd as
    /// mounted path, which we can't use in this case.
    /// Must be an absolute path
    pub root_dir: PathBuf,
    /// From here is the same as isograph_config::IsographProjectConfig
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    #[allow(dead_code)]
    pub json_schema: Option<String>,
    /// The relative path to the folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The relative path to the folder where the compiler should create artifacts
    /// Defaults to the project_root directory.
    pub artifact_directory: Option<PathBuf>,
    /// The relative path to the GraphQL schema
    pub schema: PathBuf,
    /// The relative path to schema extensions
    #[serde(default)]
    pub schema_extensions: Vec<PathBuf>,

    /// Various options of less importance
    #[serde(default)]
    pub options: ConfigFileOptions,
}

#[plugin_transform]
fn isograph_plugin_transform(
    program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    let config: WasmConfig = serde_json::from_str(
        &metadata
            .get_transform_plugin_config()
            .expect("Failed to get plugin config for isograph"),
    )
    .unwrap_or_else(|e| panic!("Error parsing plugin config. Error: {}", e));

    let root_dir = config.root_dir;

    let isograph_config = IsographProjectConfig {
        json_schema: config.json_schema,
        project_root: config.project_root,
        artifact_directory: config.artifact_directory,
        schema: config.schema,
        schema_extensions: config.schema_extensions,
        options: config.options,
    };

    debug!("Config: {:?}", isograph_config);
    let file_name = metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap_or_default();
    let path = Path::new(&file_name);

    let mut isograph = swc_isograph::isograph(
        &isograph_config,
        path,
        root_dir.as_path(),
        Some(metadata.unresolved_mark),
    );

    program.fold_with(&mut isograph)
}
