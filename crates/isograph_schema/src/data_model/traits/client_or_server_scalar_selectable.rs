use isograph_lang_types::DefinitionLocation;

use crate::{ClientScalarSelectable, ServerScalarSelectable};

pub type ScalarSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    &'a ServerScalarSelectable<TNetworkProtocol>,
    &'a ClientScalarSelectable<TNetworkProtocol>,
>;
