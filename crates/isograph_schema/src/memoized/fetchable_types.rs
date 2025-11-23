use std::collections::BTreeMap;

use common_lang_types::{DiagnosticResult, ServerObjectEntityName};
use pico_macros::memo;

use crate::{IsographDatabase, NetworkProtocol, RootOperationName};

/// This is a GraphQL-ism and this function should probably not exist.
#[memo]
pub fn fetchable_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<BTreeMap<ServerObjectEntityName, RootOperationName>> {
    let (_items, fetchable_types) = TNetworkProtocol::parse_type_system_documents(db).to_owned()?;

    Ok(fetchable_types)
}
