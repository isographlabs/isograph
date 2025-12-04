use isograph_lang_types::DefinitionLocation;
use pico::MemoRef;

use crate::{ClientScalarSelectable, ServerScalarSelectable};

pub type ScalarSelectable<TNetworkProtocol> = DefinitionLocation<
    MemoRef<ServerScalarSelectable<TNetworkProtocol>>,
    ClientScalarSelectable<TNetworkProtocol>,
>;
