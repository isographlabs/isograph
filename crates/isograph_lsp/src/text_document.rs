use common_lang_types::relative_path_from_absolute_and_working_directory;
use isograph_compiler::{get_current_working_directory, get_open_file_map, CompilerState};
use isograph_schema::OpenFileSource;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, TextDocumentItem,
};
use pico::Database;

use crate::lsp_runtime_error::LSPRuntimeResult;

pub fn on_did_open_text_document(
    compiler_state: &mut CompilerState,
    params: <DidOpenTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidOpenTextDocumentParams { text_document } = params;
    let TextDocumentItem { text, uri, .. } = text_document;

    let db = &mut compiler_state.db;
    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    let source_id = db.set(OpenFileSource {
        relative_path: relative_path_to_source_file,
        content: text,
    });

    let mut open_file_map = get_open_file_map(db).clone();

    open_file_map
        .0
        .insert(relative_path_to_source_file, source_id);

    db.set(open_file_map);

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
pub fn on_did_close_text_document(
    compiler_state: &mut CompilerState,
    params: <DidCloseTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let uri = params.text_document.uri;
    let db = &mut compiler_state.db;

    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    let mut open_file_map = get_open_file_map(db).clone();

    let deleted_entry = open_file_map
        .0
        .remove(&relative_path_to_source_file)
        .expect("Expected file to exist in OpenFileMap");

    db.remove(deleted_entry);

    db.set(open_file_map);

    Ok(())
}

pub fn on_did_change_text_document(
    compiler_state: &mut CompilerState,
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

    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    db.set(OpenFileSource {
        relative_path: relative_path_to_source_file,
        content: content_changed.text.to_owned(),
    });

    Ok(())
}
