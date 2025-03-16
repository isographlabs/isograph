use common_lang_types::CurrentWorkingDirectory;
use intern::string_key::Intern;
use isograph_config;
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

    let mut isograph = swc_isograph::isograph(&config, filename, Some(metadata.unresolved_mark));

    program.fold_with(&mut isograph)
}
