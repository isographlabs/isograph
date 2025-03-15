use common_lang_types::CurrentWorkingDirectory;
use intern::string_key::Intern;
use isograph_config;
use serde_json::Value;
use std::path::PathBuf;
use swc_common::{plugin::metadata::TransformPluginMetadataContextKind, FileName};
use swc_core::{
    ecma::{ast::Program, visit::FoldWith},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use swc_isograph;

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
    let config = isograph_config::create_config(
        "./isograph.config.json".into(),
        current_working_directory(),
    );
    let filename = if let Some(filename) =
        metadata.get_context(&TransformPluginMetadataContextKind::Filename)
    {
        FileName::Real(PathBuf::from(filename))
    } else {
        FileName::Anon
    };

    // let plugin_config: Value = serde_json::from_str(
    //     &metadata
    //         .get_transform_plugin_config()
    //         .expect("failed to get plugin config for isograph"),
    // )
    // .expect("Should provide plugin config");

    // let artifact_directory = plugin_config["artifact_directory"]
    //     .as_str()
    //     .map(PathBuf::from);
    // let project_root = plugin_config["project_root"].as_str().map(PathBuf::from);

    // let language = plugin_config["language"]
    //     .as_str()
    //     .map_or(RelayLanguageConfig::TypeScript, |v| v.try_into().unwrap());
    // let output_file_extension = plugin_config["outputFileExtension"]
    //     .as_str()
    //     .map_or(OutputFileExtension::Undefined, |v| v.try_into().unwrap());
    // let eager_es_modules = plugin_config["eagerEsModules"]
    //     .as_bool()
    //     .unwrap_or_default();

    let mut isograph = swc_isograph::isograph(&config, filename, Some(metadata.unresolved_mark));

    program.fold_with(&mut isograph)
}
