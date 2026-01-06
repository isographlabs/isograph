use std::collections::HashMap;

use crate::{
    ClientFieldVariant, ClientScalarSelectable, CompilationProfile, IsographDatabase,
    LINK_FIELD_NAME, flattened_server_object_entities,
};
use common_lang_types::{EntityName, SelectableName, WithLocationPostfix};
use intern::string_key::Intern;
use isograph_lang_types::Description;
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

#[memo]
pub fn get_link_fields<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<MemoRef<ClientScalarSelectable<TCompilationProfile>>> {
    flattened_server_object_entities(db)
        .iter()
        .map(|object| {
            let field_name = *LINK_FIELD_NAME;
            let parent_entity_name = object.lookup(db).name;
            ClientScalarSelectable {
                description: Description(
                    format!("A store Link for the {} type.", parent_entity_name)
                        .intern()
                        .into(),
                )
                .with_no_location()
                .wrap_some(),
                name: field_name,
                parent_entity_name: parent_entity_name.item,
                variable_definitions: vec![],
                variant: ClientFieldVariant::Link,
                phantom_data: std::marker::PhantomData,
            }
            .interned_value(db)
        })
        .collect::<Vec<_>>()
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn get_link_fields_map<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> HashMap<(EntityName, SelectableName), MemoRef<ClientScalarSelectable<TCompilationProfile>>> {
    get_link_fields(db)
        .clone()
        .into_iter()
        .map(|link_selectable| {
            (
                (
                    link_selectable.lookup(db).parent_entity_name,
                    (*LINK_FIELD_NAME),
                ),
                link_selectable,
            )
        })
        .collect::<HashMap<_, _>>()
}
