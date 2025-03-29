use isograph_config::IsographProjectConfig;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};
use swc_ecma_parser::{EsConfig, Syntax};
use swc_ecma_transforms_testing::test_fixture;
use swc_isograph::isograph;

#[testing::fixture("tests/fixtures/base/**/*/input.js")]
fn fixture(input: PathBuf) {
    let root_dir = input.parent().unwrap();
    let isograph_config =
        read_to_string(root_dir.join("isograph.config.json")).expect("failed to read config.json");
    println!("---- Config -----\n{}", isograph_config);

    let config: IsographProjectConfig = serde_json::from_str(&isograph_config).unwrap();

    let output = root_dir.join("output.js");
    let filename = format!("{}/src/components/HomeRoute.tsx", root_dir.display());

    test_fixture(
        Syntax::Es(EsConfig {
            jsx: true,
            ..Default::default()
        }),
        &|_| isograph(&config, Path::new(&filename), Path::new(root_dir), None),
        &input,
        &output,
        Default::default(),
    );
}
