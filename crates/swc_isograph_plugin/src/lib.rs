use common_lang_types::CurrentWorkingDirectory;
use intern::string_key::Intern;
use isograph_config;
use serde::Deserialize;
use std::path::Path;
use std::path::PathBuf;
use swc_common::plugin::metadata::TransformPluginMetadataContextKind;
use swc_core::{
    ecma::{ast::Program, visit::FoldWith},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use swc_isograph;
use tracing::debug;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WasmConfig {
    root_dir: PathBuf,
}

fn current_working_directory() -> CurrentWorkingDirectory {
    std::env::current_dir()
        .expect("Expected current working to exist")
        .to_str()
        .expect("Expected current working directory to be able to be stringified.")
        .intern()
        .into()
}

#[plugin_transform]
fn isograph_plugin_transform(
    program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    debug!("Isograph plugin transform called");
    let config = isograph_config::create_config(
        "/cwd/isograph.config.json".into(),
        current_working_directory(),
    );
    debug!("Config: {:?}", config);
    let file_name = metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap_or_default();
    let path = Path::new(&file_name);
    debug!("File name: {:?}", path);
    let mut isograph = swc_isograph::isograph(&config, path, Some(metadata.unresolved_mark));

    program.fold_with(&mut isograph)
}
