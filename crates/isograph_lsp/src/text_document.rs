use common_lang_types::relative_path_from_absolute_and_working_directory;
use isograph_compiler::CompilerState;
use isograph_schema::NetworkProtocol;
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, TextDocumentItem,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
};

use crate::{lsp_runtime_error::LSPRuntimeResult, uri_file_path_ext::UriFilePathExt};

pub fn on_did_open_text_document<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &mut CompilerState<TNetworkProtocol>,
    params: <DidOpenTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidOpenTextDocumentParams { text_document } = params;
    let TextDocumentItem { text, uri, .. } = text_document;

    let db = &mut compiler_state.db;
    let current_working_directory = db.get_current_working_directory();

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );
    db.insert_open_file(relative_path_to_source_file, text);

    Ok(())
}

pub fn on_did_close_text_document<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &mut CompilerState<TNetworkProtocol>,
    params: <DidCloseTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let uri = params.text_document.uri;
    let db = &mut compiler_state.db;

    let current_working_directory = db.get_current_working_directory();

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    db.remove_open_file(relative_path_to_source_file);

    Ok(())
}

pub fn on_did_change_text_document<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &mut CompilerState<TNetworkProtocol>,
    params: <DidChangeTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidChangeTextDocumentParams {
        content_changes,
        text_document,
    } = params;
    let uri = text_document.uri;
    let db = &mut compiler_state.db;

    // We do full text document syncing, so the new text will be in the first content change event.
    let content_changed = content_changes
        .first()
        .expect("content_changes should always be non-empty");

    let current_working_directory = db.get_current_working_directory();

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    db.insert_open_file(
        relative_path_to_source_file,
        content_changed.text.to_owned(),
    );

    Ok(())
}
