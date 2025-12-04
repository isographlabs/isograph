use std::collections::HashMap;

use crate::{
    ClientFieldVariant, ClientScalarSelectable, IsographDatabase, LINK_FIELD_NAME, NetworkProtocol,
    server_object_entities,
};
use common_lang_types::{DiagnosticResult, EntityName, SelectableName, WithLocationPostfix};
use intern::string_key::Intern;
use isograph_lang_types::Description;
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

#[memo]
pub fn get_link_fields<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<Vec<MemoRef<ClientScalarSelectable<TNetworkProtocol>>>> {
    server_object_entities(db)
        .as_ref()
        .map_err(|e| e.clone())?
        .iter()
        .map(|object| {
            let field_name = *LINK_FIELD_NAME;
            let parent_object_entity_name = object.lookup(db).name;
            ClientScalarSelectable {
                description: Some(Description(
                    format!("A store Link for the {} type.", parent_object_entity_name)
                        .intern()
                        .into(),
                )),
                name: field_name.with_generated_location(),
                parent_object_entity_name,
                variable_definitions: vec![],
                variant: ClientFieldVariant::Link,
                network_protocol: std::marker::PhantomData,
            }
            .interned_value(db)
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn get_link_fields_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<
    HashMap<(EntityName, SelectableName), MemoRef<ClientScalarSelectable<TNetworkProtocol>>>,
> {
    get_link_fields(db)
        .to_owned()
        .note_todo("Do not clone. Use a MemoRef.")?
        .into_iter()
        .map(|link_selectable| {
            (
                (
                    link_selectable.lookup(db).parent_object_entity_name,
                    (*LINK_FIELD_NAME).into(),
                ),
                link_selectable,
            )
        })
        .collect::<HashMap<_, _>>()
        .wrap_ok()
}
