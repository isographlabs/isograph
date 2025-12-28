use isograph_lang_types::DefinitionLocation;
use pico::MemoRef;

use crate::{
    ClientObjectSelectable, MemoRefClientSelectable, MemoRefServerSelectable, ServerSelectable,
};

pub type MemoRefSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRefServerSelectable<TNetworkProtocol>,
    MemoRefClientSelectable<TNetworkProtocol>,
>;

pub type BorrowedObjectSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

pub type MemoRefObjectSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRef<ServerSelectable<TNetworkProtocol>>,
    MemoRef<ClientObjectSelectable<TNetworkProtocol>>,
>;
