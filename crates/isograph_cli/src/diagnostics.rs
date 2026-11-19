use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use common_lang_types::RelativePathToSourceFile;
use intern::Lookup;
use isograph_compiler::{ClientId, HostLanguage, IsographDb, SkipSourceFile};
use prelude::Postfix;

use crate::effect::{IsographEffect, PublishDiagnosticsEffect};
use crate::event::LiveSession;
use crate::state::IsographState;
use crate::timer::TimerPayload;
use crate::watch::SourceGlobs;

pub(crate) const DIAGNOSTICS_DEBOUNCE: Duration = Duration::from_millis(100);

/// What one client was last sent: every path whose last notification had a non-empty list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishedDiagnostics(pub HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>>);

#[derive(Debug, thiserror::Error)]
enum PublishDiagnosticsError {
    #[error("{0}")]
    Uri(#[from] crate::adapter::PathUriError),
    #[error("{0}")]
    Serialize(#[from] serde_json::Error),
}

/// Replaces the pending diagnostics timer with one that fires after `DIAGNOSTICS_DEBOUNCE`.
pub(crate) fn arm_diagnostics_debounce<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) = freddie_timer::timer_effect_and_guard(
        DIAGNOSTICS_DEBOUNCE,
        TimerPayload::DiagnosticsDebounce,
    );
    state.diagnostics_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

/// Changed diagnostics for every live client that accepts them, one effect per client.
pub(crate) fn publish_diagnostics<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let db = state.db.reference();
    let source_globs = state.source_globs.reference();
    state
        .live_sessions
        .0
        .iter_mut()
        .filter_map(|(client, session)| {
            publish_to_client(db, source_globs, client.dereference(), session)
        })
        .collect()
}

fn publish_to_client<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    source_globs: &SourceGlobs,
    client: ClientId,
    session: &mut LiveSession,
) -> Option<IsographEffect> {
    session
        .capabilities
        .text_document
        .as_ref()
        .and_then(|text_document| text_document.publish_diagnostics.as_ref())?;
    let previous = std::mem::take(&mut session.published_diagnostics);
    let mut current = client_diagnostics(db, source_globs, client);
    let outcomes: Vec<_> = changed_diagnostics(previous.reference(), current.reference())
        .map(|(path, diagnostics)| (path, publish_notification(db, path, diagnostics)))
        .collect();
    let notifications: Vec<lsp_server::Notification> = outcomes
        .into_iter()
        .filter_map(|(path, outcome)| {
            outcome
                .map_err(|error| {
                    tracing::warn!(%error, path = path.lookup(), "publishDiagnostics");
                    match previous.0.get(&path) {
                        Some(sent) => current.insert(path, sent.clone()),
                        None => current.remove(&path),
                    };
                })
                .ok()
        })
        .collect();
    session.published_diagnostics = PublishedDiagnostics(current);
    if notifications.is_empty() {
        return None;
    }
    IsographEffect::PublishDiagnostics(
        PublishDiagnosticsEffect {
            sender: session.sender.clone(),
            notifications,
        }
        .boxed(),
    )
    .wrap_some()
}

/// Non-empty diagnostics of every source file `client` has open.
fn client_diagnostics<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    source_globs: &SourceGlobs,
    client: ClientId,
) -> HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>> {
    db.open_paths_of_client(client)
        .into_iter()
        .filter(|path| {
            let relative = Path::new(path.lookup());
            source_globs.contains(relative)
                && matches!(
                    THostLanguage::should_skip_source_file(relative),
                    SkipSourceFile::Keep
                )
        })
        .filter_map(|path| {
            let diagnostics = isograph_lsp::lsp_diagnostics_for_file(db, client, path).as_ref()?;
            (!diagnostics.is_empty()).then(|| (path, diagnostics.clone()))
        })
        .collect()
}

/// Paths whose `current` list differs from `published`, and `vec![]` for published paths absent from `current`.
fn changed_diagnostics<'a>(
    published: &'a PublishedDiagnostics,
    current: &'a HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>>,
) -> impl Iterator<Item = (RelativePathToSourceFile, Vec<lsp_types::Diagnostic>)> + 'a {
    let changed = current
        .iter()
        .filter(|(path, diagnostics)| {
            published.0.get(*path) != diagnostics.dereference().wrap_some()
        })
        .map(|(path, diagnostics)| (path.dereference(), diagnostics.clone()));
    let cleared = published
        .0
        .keys()
        .filter(|path| !current.contains_key(*path))
        .map(|path| (path.dereference(), Vec::new()));
    changed.chain(cleared)
}

fn publish_notification<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<lsp_server::Notification, PublishDiagnosticsError> {
    let uri = crate::adapter::uri_for_path(db, path)?;
    let params = serde_json::to_value(lsp_types::PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    })?;
    lsp_server::Notification {
        method: <lsp_types::notification::PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_owned(),
        params,
    }
    .wrap_ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use prelude::Postfix;

    use super::{PublishedDiagnostics, changed_diagnostics};

    fn path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn diagnostic(message: &str) -> lsp_types::Diagnostic {
        lsp_types::Diagnostic {
            message: message.to_owned(),
            ..Default::default()
        }
    }

    fn changed(
        published: &PublishedDiagnostics,
        current: &HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>>,
    ) -> HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>> {
        changed_diagnostics(published, current).collect()
    }

    #[test]
    fn empty_published_and_empty_current_yield_nothing() {
        assert!(changed(&PublishedDiagnostics::default(), &HashMap::new()).is_empty());
    }

    #[test]
    fn a_new_path_yields_its_list() {
        let current = HashMap::from([(path("src/a.ts"), diagnostic("a").wrap_vec())]);
        assert_eq!(changed(&PublishedDiagnostics::default(), &current), current);
    }

    #[test]
    fn an_equal_list_yields_nothing() {
        let current = HashMap::from([(path("src/a.ts"), diagnostic("a").wrap_vec())]);
        let published = PublishedDiagnostics(current.clone());
        assert!(changed(&published, &current).is_empty());
    }

    #[test]
    fn a_different_list_yields_the_new_list() {
        let published = PublishedDiagnostics(HashMap::from([(
            path("src/a.ts"),
            diagnostic("a").wrap_vec(),
        )]));
        let current = HashMap::from([(path("src/a.ts"), diagnostic("b").wrap_vec())]);
        assert_eq!(changed(&published, &current), current);
    }

    #[test]
    fn a_published_path_absent_from_current_yields_an_empty_list() {
        let published = PublishedDiagnostics(HashMap::from([(
            path("src/a.ts"),
            diagnostic("a").wrap_vec(),
        )]));
        assert_eq!(
            changed(&published, &HashMap::new()),
            HashMap::from([(path("src/a.ts"), Vec::new())])
        );
    }
}
