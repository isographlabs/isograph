use isograph_lang_types::DefinitionLocation;
use pico::MemoRef;

use crate::{
    ClientObjectSelectable, MemoRefClientSelectable, MemoRefServerSelectable,
    ServerObjectSelectable,
};

pub type MemoRefSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRefServerSelectable<TNetworkProtocol>,
    MemoRefClientSelectable<TNetworkProtocol>,
>;

pub type BorrowedObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerObjectSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

pub type MemoRefObjectSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRef<ServerObjectSelectable<TNetworkProtocol>>,
    MemoRef<ClientObjectSelectable<TNetworkProtocol>>,
>;
