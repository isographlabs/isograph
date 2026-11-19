use std::convert::Infallible;
use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use prelude::Postfix;
use thiserror::Error;

use crate::host_language::{
    CompilationProfile, HostLanguage, IncludeFile, IsoLiteralExtraction, NetworkProtocol,
};
use crate::{ClientId, IsographDb, ProtocolEntity, ProtocolSelectable};

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
    type HostLanguage = TestHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
