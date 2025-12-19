use isograph_lang_types::DefinitionLocation;

use crate::{MemoRefClientSelectable, MemoRefServerSelectable};

pub type MemoRefSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRefServerSelectable<TNetworkProtocol>,
    MemoRefClientSelectable<TNetworkProtocol>,
>;
