use std::collections::BTreeMap;

use common_lang_types::ServerObjectEntityName;
use pico::MemoRef;
use pico_macros::legacy_memo;

use crate::{IsographDatabase, NetworkProtocol, RootOperationName};

/// This is a GraphQL-ism and this function should probably not exist.
#[legacy_memo]
pub fn fetchable_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    MemoRef<BTreeMap<ServerObjectEntityName, RootOperationName>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let (_items, fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    Ok(db.intern_ref(fetchable_types))
}
