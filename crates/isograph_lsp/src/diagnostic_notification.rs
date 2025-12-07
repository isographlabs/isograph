#![allow(clippy::mutable_key_type)]

use std::collections::{BTreeMap, BTreeSet};

use common_lang_types::Diagnostic;
use isograph_schema::{IsographDatabase, NetworkProtocol, read_iso_literals_source};
use lsp_types::{
    PublishDiagnosticsParams, Uri,
    notification::{Notification, PublishDiagnostics},
};

use crate::location_utils::isograph_location_to_lsp_location;

pub(crate) fn publish_new_diagnostics_and_clear_old_diagnostics<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    new_diagnostics: &[Diagnostic],
    sender: &crossbeam::channel::Sender<lsp_server::Message>,
    old_uris_with_diagnostics: BTreeSet<Uri>,
) -> BTreeSet<Uri> {
    let (diagnostic_params, new_uris_with_diagnostics) =
        iso_diagnostics_to_params(db, new_diagnostics, old_uris_with_diagnostics);
    for diagnostic_param in diagnostic_params {
        let notif =
            lsp_server::Notification::new(PublishDiagnostics::METHOD.into(), diagnostic_param);
        sender
            .send(lsp_server::Message::Notification(notif))
            .unwrap_or(());
    }
    new_uris_with_diagnostics
}

// TODO clean this up
fn iso_diagnostics_to_params<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    diagnostics: &[Diagnostic],
    old_uris_with_diagnostics: BTreeSet<Uri>,
) -> (
    impl Iterator<Item = PublishDiagnosticsParams>,
    BTreeSet<Uri>,
) {
    let mut map: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for diagnostic in diagnostics {
        let location = match diagnostic.location().and_then(|l| l.as_embedded_location()) {
            Some(l) => l,
            // TODO don't do this
            None => continue,
        };

        let path = location.text_source.relative_path_to_source_file;

        // TODO diagnostics with GraphQL locations get dropped here
        let iso_literal_map = db.get_iso_literal_map();
        let source_id = match iso_literal_map.tracked().0.get(&path) {
            Some(source_id) => source_id,
            // TODO don't do this
            None => continue,
        };
        let source = read_iso_literals_source(db, *source_id);

        let location = match isograph_location_to_lsp_location(db, location, &source.content) {
            Some(l) => l,
            // TODO don't do this
            None => continue,
        };

        map.entry(location.uri)
            .or_default()
            .push(lsp_types::Diagnostic {
                range: location.range,
                message: diagnostic.to_string(),
                ..Default::default()
            })
    }

    for uri in old_uris_with_diagnostics {
        // Explicitly send a message saying there are no diagnostics for files where we
        // previously sent diagnostics, but no longer have any
        map.entry(uri).or_default();
    }

    let paths = map.keys().cloned().collect();

    (
        map.into_iter()
            .map(|(uri, diagnostics)| PublishDiagnosticsParams {
                uri,
                diagnostics,
                version: None,
            }),
        paths,
    )
}
