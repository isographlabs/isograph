use isograph_lang_types::DefinitionLocation;

use crate::{ClientScalarSelectable, ServerScalarSelectable};

pub type ScalarSelectable<'a, TNetworkProtocol> = DefinitionLocation<
    // HACK: Note the owned server scalar selectable
    // This is fixable when memoized functions can return references with 'db lifetime
    ServerScalarSelectable<TNetworkProtocol>,
    &'a ClientScalarSelectable<TNetworkProtocol>,
>;
