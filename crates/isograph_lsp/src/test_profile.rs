use std::convert::Infallible;
use std::path::Path;

use common_lang_types::{EntityName, SelectableName};
use isograph_compiler::{
    ClientId, CompilationProfile, EntityKind, IncludeFile, IsographDb, NetworkProtocol,
    ProtocolEntity, ProtocolSelectable,
};
use isograph_extract_typescript::TypeScriptHostLanguage;
use prelude::Postfix;

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

    fn entity_shape(_data: &()) -> isograph_compiler::EntityShape {
        isograph_compiler::EntityShape::Leaf
    }

    fn selectable_shape(_data: &()) -> isograph_compiler::SelectableShape {
        isograph_compiler::SelectableShape::Direct
    }

    fn discriminator() -> SelectableName {
        intern::string_key::Intern::intern("__typename").to()
    }
}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;

    fn custom_scalar_types() -> std::collections::BTreeMap<
        common_lang_types::EntityName,
        <Self::HostLanguage as isograph_compiler::HostLanguage>::CustomScalarType,
    > {
        std::collections::BTreeMap::new()
    }
}
