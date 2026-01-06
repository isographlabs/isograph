use common_lang_types::WithGenericLocation;
use isograph_lang_types::{DefinitionLocation, Description, TypeAnnotationDeclaration};
use prelude::Postfix;

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

pub fn output_type_annotation<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    definition_location: MemoRefObjectSelectable<TCompilationProfile>,
) -> &TypeAnnotationDeclaration {
    match definition_location {
        DefinitionLocation::Client(client_pointer) => {
            client_pointer.lookup(db).target_entity_name.reference()
        }
        DefinitionLocation::Server(server_field) => {
            server_field.lookup(db).target_entity.reference()
        }
    }
}
