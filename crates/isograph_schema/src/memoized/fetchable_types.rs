use std::collections::BTreeMap;

use common_lang_types::ServerObjectEntityName;
use intern::string_key::Intern;
use pico::MemoRef;
use pico_macros::memo;

use crate::{IsographDatabase, NetworkProtocol, RootOperationName};

/// This is a GraphQL-ism and this function should probably not exist.
#[memo]
pub fn fetchable_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    MemoRef<BTreeMap<ServerObjectEntityName, RootOperationName>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let (_items, fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    eprintln!(
        "fetchable types called, always has len 3 (proof: {})",
        fetchable_types.len()
    );
    Ok(db.intern_ref(fetchable_types))

    // things that don't break:

    // - db.intern(value)
    // Ok(db.intern({
    //     let mut map = BTreeMap::new();
    //     map.insert("foo".intern().into(), RootOperationName("Query"));
    //     map
    // }))

    // - to_owned, then intern_ref
    // let (_items, fetchable_types) = TNetworkProtocol::parse_type_system_documents(db).to_owned()?;
    // eprintln!("fetchable types called, len {}", fetchable_types.len());
    // Ok(db.intern_ref(&fetchable_types))
}

/// This is a GraphQL-ism and this function should probably not exist.
#[memo]
pub fn fetchable_types_2<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    BTreeMap<ServerObjectEntityName, RootOperationName>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
    let (_items, fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    eprintln!("fetchable types 2 {:?}", fetchable_types.len());
    Ok(fetchable_types.clone())
}
