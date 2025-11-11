use std::collections::HashMap;

use common_lang_types::{ClientSelectableName, ServerObjectEntityName};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{ClientFieldDeclaration, ClientPointerDeclaration, SelectionType};
use isograph_schema::{IsographDatabase, NetworkProtocol};
use pico_macros::legacy_memo;

use crate::parse_iso_literal_in_source;

#[legacy_memo]
pub fn client_selectable_declaration_map<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    Vec<SelectionType<ClientFieldDeclaration, ClientPointerDeclaration>>,
> {
    let mut out: HashMap<(_, ClientSelectableName), Vec<_>> = HashMap::new();

    for (_relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        for extraction in parse_iso_literal_in_source(db, *iso_literals_source_id).to_owned() {
            match extraction {
                Ok((extraction_result, _)) => match extraction_result {
                    IsoLiteralExtractionResult::ClientPointerDeclaration(
                        client_pointer_declaration,
                    ) => {
                        out.entry((
                            client_pointer_declaration.item.parent_type.item.0,
                            client_pointer_declaration
                                .item
                                .client_pointer_name
                                .item
                                .0
                                .into(),
                        ))
                        .or_default()
                        .push(SelectionType::Object(client_pointer_declaration.item));
                    }
                    IsoLiteralExtractionResult::ClientFieldDeclaration(
                        client_field_declaration,
                    ) => {
                        out.entry((
                            client_field_declaration.item.parent_type.item.0,
                            client_field_declaration
                                .item
                                .client_field_name
                                .item
                                .0
                                .into(),
                        ))
                        .or_default()
                        .push(SelectionType::Scalar(client_field_declaration.item));
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => {
                        // Intentionally ignored. TODO reconsider
                    }
                },

                Err(_) => {
                    // For now, we can only ignore these errors! We don't know a parent entity name
                    // and a selectable name. But. we should restructure this so that we get both,
                    // even if the rest fails to parse.
                }
            }
        }
    }

    out
}
