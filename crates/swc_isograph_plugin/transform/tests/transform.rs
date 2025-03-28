use common_lang_types::CurrentWorkingDirectory;
use intern::string_key::Intern;
use isograph_config::CompilerConfig;
use std::path::{Path, PathBuf};
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
    std::env::set_current_dir(dir)
        .expect("Expected current working directory to be able to be set.");
    let config: CompilerConfig = isograph_config::create_config(
        dir.join("isograph.config.json").into(),
        current_working_directory(),
    );

    let output = dir.join("output.js");
    let filename = format!("{}/src/components/Home/Header/File.ts", dir.display());

    test_fixture(
        Syntax::Es(EsConfig {
            jsx: true,
            ..Default::default()
        }),
        &|_| isograph(&config, Path::new(&filename), None),
        &input,
        &output,
        Default::default(),
    );
}
