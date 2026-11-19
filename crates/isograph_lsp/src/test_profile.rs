use isograph_compiler::{CompilationProfile, NetworkProtocol};
use isograph_extract_typescript::TypeScriptHostLanguage;

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
