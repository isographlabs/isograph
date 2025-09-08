use isograph_config::IsographProjectConfig;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};
use swc_ecma_parser::{EsSyntax, Syntax};
use swc_ecma_transforms_testing::{FixtureTestConfig, test_fixture};
use swc_isograph_plugin::compile_iso_literal_visitor;

#[testing::fixture("tests/fixtures/base/*/input.js")]
fn run(input: PathBuf) {
    let root_dir = input.parent().unwrap();
    let isograph_config =
        read_to_string(root_dir.join("isograph.config.json")).expect("failed to read config.json");
    let config: IsographProjectConfig = serde_json::from_str(&isograph_config).unwrap();
    let output = root_dir.join("output.js");
    let filename = format!("{}/src/components/HomeRoute.tsx", root_dir.display());

    test_fixture(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        &|_| compile_iso_literal_visitor(&config, Path::new(&filename), Path::new(root_dir), None),
        &input,
        &output,
        FixtureTestConfig {
            module: Some(true),
            ..Default::default()
        },
    );
}

#[testing::fixture("tests/fixtures/errors/*/input.js")]
fn isograph_errors(input: PathBuf) {
    let root_dir = input.parent().unwrap();
    let isograph_config =
        read_to_string(root_dir.join("isograph.config.json")).expect("failed to read config.json");
    let config: IsographProjectConfig = serde_json::from_str(&isograph_config).unwrap();
    let output = root_dir.join("output.js");
    let filename = format!("{}/src/components/HomeRoute.tsx", root_dir.display());

    test_fixture(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        &|_| compile_iso_literal_visitor(&config, Path::new(&filename), Path::new(root_dir), None),
        &input,
        &output,
        FixtureTestConfig {
            allow_error: true,
            module: Some(true),
            ..Default::default()
        },
    );
}
