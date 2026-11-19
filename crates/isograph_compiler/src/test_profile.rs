use std::path::Path;

use thiserror::Error;

use crate::host_language::{
    CompilationProfile, HostLanguage, IncludeFile, IsoLiteralExtraction, NetworkProtocol,
};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("test host")]
pub(crate) struct TestHostError;

pub(crate) struct TestHostLanguage;

impl HostLanguage for TestHostLanguage {
    type Error = TestHostError;
    type LiteralContext = ();

    fn extract_iso_literals_from_text(_contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        Vec::new()
    }

    fn should_include_source_file(_relative_path: &Path) -> IncludeFile {
        IncludeFile::Include
    }
}

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {
    type EntityData = ();
    type SelectableData = ();

    fn should_include_schema_file(_relative_path: &Path) -> IncludeFile {
        IncludeFile::Skip
    }
}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TestHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
