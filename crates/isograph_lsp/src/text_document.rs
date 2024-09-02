use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::Notification;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::TextDocumentItem;

use crate::lsp_runtime_error::LSPRuntimeResult;
use crate::lsp_state::LSPState;

pub fn on_did_open_text_document(
    lsp_state: &mut LSPState,
    params: <DidOpenTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidOpenTextDocumentParams { text_document } = params;
    let TextDocumentItem { text, uri, .. } = text_document;

    lsp_state.document_opened(&uri, &text)
}

#[allow(clippy::unnecessary_wraps)]
pub fn on_did_close_text_document(
    lsp_state: &mut LSPState,
    params: <DidCloseTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let uri = params.text_document.uri;
    lsp_state.document_closed(&uri)
}

pub fn on_did_change_text_document(
    lsp_state: &mut LSPState,
    params: <DidChangeTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidChangeTextDocumentParams {
        content_changes,
        text_document,
    } = params;
    let uri = text_document.uri;

    // We do full text document syncing, so the new text will be in the first content change event.
    let content_change = content_changes
        .first()
        .expect("content_changes should always be non-empty");

    lsp_state.document_changed(&uri, &content_change.text)
}
