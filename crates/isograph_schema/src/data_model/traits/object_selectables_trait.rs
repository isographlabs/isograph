use isograph_lang_types::DefinitionLocation;
use pico::MemoRef;

use crate::{ClientObjectSelectable, ServerObjectSelectable};

pub type BorrowedObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerObjectSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

pub type MemoRefObjectSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRef<ServerObjectSelectable<TNetworkProtocol>>,
    MemoRef<ClientObjectSelectable<TNetworkProtocol>>,
>;
