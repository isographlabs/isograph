use std::collections::BTreeMap;

use common_lang_types::{DiagnosticResult, EntityName};
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{IsographDatabase, NetworkProtocol, RootOperationName};

/// This is a GraphQL-ism and this function should probably not exist.
#[memo]
pub fn fetchable_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<MemoRef<BTreeMap<EntityName, RootOperationName>>> {
    let (_items, fetchable_types) =
        TNetworkProtocol::parse_type_system_documents(db).clone_err()?;

    fetchable_types.interned_ref(db).wrap_ok()
}
