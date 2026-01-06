use common_lang_types::WithGenericLocation;
use isograph_lang_types::{DefinitionLocation, Description};

use crate::{CompilationProfile, IsographDatabase, MemoRefObjectSelectable};

pub fn description<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    definition_location: MemoRefObjectSelectable<TCompilationProfile>,
) -> Option<Description> {
    match definition_location {
        DefinitionLocation::Server(server_field) => server_field
            .lookup(db)
            .description
            .map(WithGenericLocation::item),
        DefinitionLocation::Client(client_field) => client_field
            .lookup(db)
            .description
            .map(WithGenericLocation::item),
    }
}
