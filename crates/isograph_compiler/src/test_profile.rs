use std::collections::BTreeMap;
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
    ArtifactPath, ClientId, EntityKind, GenerationOptions, IsographDb, ProjectArtifacts,
    ProtocolEntity, ProtocolSelectable,
};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("test host")]
pub(crate) struct TestHostError;

pub(crate) struct TestHostLanguage;

impl HostLanguage for TestHostLanguage {
    type Error = TestHostError;
    type LiteralContext = ();
    type GenerationError = TestHostError;
    type CustomScalarType = ();

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

    fn literal_errors(
        _extraction: &IsoLiteralExtraction<Self>,
        _parsed: &isograph_parser::ParsedIsoLiteral,
    ) -> Vec<span::WithSpan<Self::Error>> {
        Vec::new()
    }

    fn generate_artifacts(
        _project: &ProjectArtifacts<()>,
        _options: &GenerationOptions<()>,
    ) -> Result<Vec<(ArtifactPath, String)>, TestHostError> {
        Vec::new().wrap_ok()
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
    type SchemaFileError = Infallible;
    type Options = ();
    type ArgumentError = Infallible;

    fn should_include_schema_file(_relative_path: &Path) -> IncludeFile {
        IncludeFile::Skip
    }

    fn schema_file_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        db: &IsographDb<TProfile>,
        client: ClientId,
        path: common_lang_types::RelativePathToSourceFile,
    ) -> Option<Vec<span::WithSpan<Infallible>>> {
        crate::file_contents(db, client, path)
            .as_ref()
            .map(|_| Vec::new())
    }

    fn selection_argument_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        _selectable: crate::SelectableKey,
        _selection_name: span::Span,
        _arguments: Option<&span::WithSpan<isograph_parser::ArgumentList>>,
        _variables: Option<&span::WithSpan<isograph_parser::VariableDeclarationList>>,
    ) -> Vec<span::WithSpan<Infallible>> {
        Vec::new()
    }

    fn field_argument_errors<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        _field_variables: Option<&span::WithSpan<isograph_parser::VariableDeclarationList>>,
        _arguments: Option<&span::WithSpan<isograph_parser::ArgumentList>>,
        _variables: Option<&span::WithSpan<isograph_parser::VariableDeclarationList>>,
    ) -> Vec<span::WithSpan<Infallible>> {
        Vec::new()
    }

    fn entity_kind(_data: &()) -> EntityKind {
        EntityKind::Object
    }

    fn concrete_types<TProfile: CompilationProfile<NetworkProtocol = Self>>(
        _db: &IsographDb<TProfile>,
        _client: ClientId,
        entity: EntityName,
        _data: &(),
    ) -> Vec<EntityName> {
        entity.wrap_vec()
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

    fn entity_shape(_data: &()) -> crate::EntityShape {
        crate::EntityShape::Leaf
    }

    fn selectable_shape(_data: &()) -> crate::SelectableShape {
        crate::SelectableShape::Direct
    }

    fn discriminator() -> SelectableName {
        intern::string_key::Intern::intern("__typename").to()
    }

    fn entrypoint_wrapper(_plan: &()) -> crate::EntrypointWrapper {
        crate::EntrypointWrapper::Unwrapped(crate::RootEntities {
            operation: intern::string_key::Intern::intern("Query").to(),
            others: std::collections::BTreeSet::new(),
        })
    }

    fn options(_config: &isograph_config::IsographProjectConfig) {}

    fn query_text(
        _options: &(),
        _operation: &crate::EntrypointOperation<'_, Self>,
    ) -> crate::ProtocolQueryText {
        crate::ProtocolQueryText {
            text: common_lang_types::QueryText(String::new()),
            file_name: None,
        }
    }
}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TestHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;

    fn custom_scalar_types() -> BTreeMap<EntityName, ()> {
        BTreeMap::new()
    }
}
