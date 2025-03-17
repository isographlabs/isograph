use common_lang_types::CurrentWorkingDirectory;
use intern::string_key::Intern;
use isograph_config::CompilerConfig;
use std::path::PathBuf;
use swc_common::FileName;
use swc_ecma_parser::{EsConfig, Syntax};
use swc_ecma_transforms_testing::test_fixture;
use swc_isograph::isograph;

fn current_working_directory() -> CurrentWorkingDirectory {
    std::env::current_dir()
        .expect("Expected current working to exist")
        .to_str()
        .expect("Expected current working directory to be able to be stringified.")
        .intern()
        .into()
}

#[testing::fixture("tests/fixtures/**/*/input.js")]
fn fixture(input: PathBuf) {
    let dir = input.parent().unwrap();

    let config: CompilerConfig = isograph_config::create_config(
        dir.join("isograph.config.json").into(),
        current_working_directory(),
    );

    let output = dir.join("output.js");
    // let file_name = input.clone().to_string_lossy().to_string();

    test_fixture(
        Syntax::Es(EsConfig {
            jsx: true,
            ..Default::default()
        }),
        &|_| isograph(&config, FileName::Real("file.js".parse().unwrap()), None),
        &input,
        &output,
        Default::default(),
    );
}
