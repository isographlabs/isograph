use std::path::Path;

use isograph_compiler::{CompilationProfile, IncludeFile, NetworkProtocol};
use isograph_extract_typescript::TypeScriptHostLanguage;

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {
    type EntityData = ();
    type SelectableData = ();

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }
}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
