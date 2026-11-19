use std::convert::Infallible;
use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use isograph_compiler::{
    ClientId, CompilationProfile, IncludeFile, IsographDb, NetworkProtocol, ProtocolEntity,
    ProtocolSelectable,
};
use isograph_extract_typescript::TypeScriptHostLanguage;
use prelude::Postfix;

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {
    type EntityData = ();
    type SelectableData = ();
    type LookupError = Infallible;

    fn should_include_schema_file(_relative_path: &Path) -> IncludeFile {
        IncludeFile::Skip
    }

    fn entity<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        _name: EntityName,
    ) -> Result<Option<ProtocolEntity<()>>, Infallible> {
        None.wrap_ok()
    }

    fn selectable<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        _entity: EntityName,
        _selectable: SelectableName,
    ) -> Result<Option<ProtocolSelectable<()>>, Infallible> {
        None.wrap_ok()
    }

    fn selectables<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        _entity: EntityName,
    ) -> Vec<(SelectableName, ProtocolSelectable<()>)> {
        Vec::new()
    }
}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
