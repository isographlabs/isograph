use isograph_config::IsographProjectConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use swc_common::plugin::metadata::TransformPluginMetadataContextKind;
use swc_core::{
    ecma::{ast::Program, visit::FoldWith},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
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
    #[serde(flatten)]
    pub isograph_project_config: IsographProjectConfig,
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

    let isograph_config = config.isograph_project_config;

    debug!("Config: {:?}", isograph_config);
    let file_name = metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap_or_default();
    let path = Path::new(&file_name);

    let mut isograph = swc_isograph::compile_iso_literal_visitor(
        &isograph_config,
        path,
        root_dir.as_path(),
        Some(metadata.unresolved_mark),
    );

    program.fold_with(&mut isograph)
}
