use std::collections::BTreeSet;
use std::convert::Infallible;
use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use prelude::Postfix;
use thiserror::Error;

use crate::host_language::{
    CompilationProfile, HostLanguage, IncludeFile, IsoLiteralExtraction, IsoLiteralStartIndex,
    NetworkProtocol,
};
use crate::{
    ArtifactPath, ArtifactSource, ClientId, IsographDb, ProtocolEntity, ProtocolSelectable,
};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("test host")]
pub(crate) struct TestHostError;

pub(crate) struct TestHostLanguage;

impl HostLanguage for TestHostLanguage {
    type Error = TestHostError;
    type LiteralContext = ();

    /// The text between `` iso(` `` and the next `` ` ``, for each `` iso(` ``.
    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        const OPEN: &str = "iso(`";
        contents
            .match_indices(OPEN)
            .filter_map(|(open, _)| {
                let start = open + OPEN.len();
                let length = contents[start..].find('`')?;
                IsoLiteralExtraction {
                    iso_literal_text: contents[start..start + length].to_owned(),
                    iso_literal_start_index: IsoLiteralStartIndex(start),
                    context: (),
                }
                .wrap_some()
            })
            .collect()
    }

    fn should_include_source_file(_relative_path: &Path) -> IncludeFile {
        IncludeFile::Include
    }

    fn generate_artifacts(_sources: &BTreeSet<ArtifactSource>) -> Vec<(ArtifactPath, String)> {
        Vec::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("test protocol")]
pub(crate) struct TestProtocolError;

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {
    type EntityData = ();
    type SelectableData = ();
    type FetchPlan = ();
    type NotFetchable = TestProtocolError;
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

    fn fetch_plan<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        _entity: EntityName,
    ) -> Result<(), TestProtocolError> {
        TestProtocolError.wrap_err()
    }
}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TestHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
