# LSP semantic tokens

Requires `HostLanguage` in `crates/isograph_compiler` (landed). Opening a JavaScript or TypeScript file in VS Code colors the contents of each `iso(\`...\`)` (and `iso\`...\``) according to the grammar: `field` / `entrypoint` / `to` as keywords, type names as classes, field and selection names as properties, and so on.

Changes 1–2 land: `file_literals` and concatenating their tokens into `lsp_semantic_tokens`. Change 3's standalone stdio `isograph lsp` loop is not the process in `docs-website/docs/design-docs/event-model.md`. The LSP adapter in the daemon calls these functions. `didOpen` / `didChange` / `didClose` become `EditorChanged`. `semanticTokens/full` is answered by the adapter reading `OpenFile` else `DiskFile`. The vscode-extension spawn of `isograph lsp` becomes the stdio proxy (`refactors/pending/event-model.md` item 6). Until that adapter exists, change 3 is not the next slice.

## What the user does

Open a `.ts` / `.tsx` / `.js` / `.jsx` file that contains:

```
export const fullName = iso(`
  field Pet.fullName {
    id
  }
`)(({ data }) => data.id);
```

The extension activates, starts `isograph lsp`, and VS Code requests semantic tokens. `field` is a keyword, `Pet` a class, `fullName` and `id` properties, `{` `}` operators.

## Types

Most important first.

```rust
// from crates/isograph_lsp/src/file_literals.rs
use isograph_compiler::{HostLanguage, IsoLiteralError};
use isograph_parser::{IsoLiteralParse, IsographSemanticToken, parse_iso_literal};
use span::WithSpan;

pub struct FileLiteral<'a, THostLanguage: HostLanguage> {
    pub extraction: WithSpan<(&'a str, THostLanguage::LiteralContext)>,
    pub parse: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<IsoLiteralError<THostLanguage>>>,
    pub tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub fn file_literals<THostLanguage: HostLanguage>(
    host: &THostLanguage,
    source: &str,
) -> Vec<FileLiteral<'_, THostLanguage>> {
    host.extract_iso_literals(source)
        .into_iter()
        .map(|extracted| {
            let extraction = extracted.item;
            let text = extraction.item.0;
            let parsed = parse_iso_literal(text);
            FileLiteral {
                extraction,
                parse: parsed.item,
                errors: extracted.errors,
                tokens: parsed.tokens,
            }
        })
        .collect()
}
```

`parse` is `None` when `ParsedIsoLiteral.item` is `None` (empty literal). `tokens` are literal-relative, consume order, whatever the grammar recorded. extra and extra_chunks leftover is leftover-semantic-tokens.md. The matcher's cut is not filled in. `errors` is `WithErrors.errors`: `IsoLiteralError` (`Host`, `Parse`). `Parse` is pipeline `ParseError` (`Ast`, `Bracket`, `Comma`), already file-absolute.

```rust
// from crates/isograph_lsp/src/lsp_state.rs
use std::collections::HashMap;

use isograph_compiler::HostLanguage;

pub struct LspState<THostLanguage: HostLanguage> {
    pub open_files: HashMap<String, String>,
    pub host: THostLanguage,
}
```

Key is `uri.as_str()`. Value is the full document text from `didOpen` / `didChange`. `host` is the extraction implementor the binary passed to `start`.

## Change 1: `file_literals`

encoding.md created `crates/isograph_lsp` with `lsp_semantic_tokens` and `semantic_token_legend`. This change adds `file_literals`. Workspace member via `./crates/*`. The `Cargo.toml` below is the crate after this change: encoding.md's deps plus `isograph_compiler`, `lsp-server`, `serde_json`, `tracing`, and the extract-typescript dev-dependency.

```toml
# from crates/isograph_lsp/Cargo.toml
[package]
name = "isograph_lsp"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
common_lang_types = { path = "../common_lang_types" }
intern = { path = "../../relay-crates/intern" }
isograph_compiler = { path = "../isograph_compiler" }
isograph_parser = { path = "../isograph_parser" }
lsp-server = { workspace = true }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
serde_json = { workspace = true }
span = { path = "../span" }
tracing = { workspace = true }

[dev-dependencies]
isograph_extract_typescript = { path = "../isograph_extract_typescript" }

[lints]
workspace = true
```

No `typescript` feature. Production `isograph_lsp` does not depend on `isograph_extract_typescript`. Tests that open a TypeScript fixture take `TypeScriptHostLanguage` through the dev-dependency.

`src/lib.rs`:

```rust
// from crates/isograph_lsp/src/lib.rs
use isograph_compiler::HostLanguage;

mod file_literals;
mod lsp_notification_dispatch;
mod lsp_request_dispatch;
mod lsp_runtime_error;
mod lsp_state;
mod semantic_tokens;
mod server;
mod text_document;

pub fn start<THostLanguage: HostLanguage>(host: THostLanguage) -> std::process::ExitCode;
```

`file_literals.rs` is the types above. Tests in that module:

```rust
// from crates/isograph_lsp/src/file_literals.rs
#[cfg(test)]
mod tests {
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use isograph_parser::{IsoLiteralItem, IsographSemanticToken};
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpanPostfix};

    use super::file_literals;

    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    #[test]
    fn exported_field_is_one_file_literal() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let literals = file_literals(&TypeScriptHostLanguage, source);
        assert_eq!(literals.len(), 1);
        assert_eq!(
            literals[0].extraction.item.1.const_export_name,
            "fullName".intern().to::<common_lang_types::ConstExportName>().wrap_some()
        );
        assert!(matches!(
            literals[0]
                .parse
                .as_ref()
                .expect("the fixture is not an empty literal")
                .item
                .item
                .item
                .item
                .as_ref()
                .expect("the fixture parsed a declaration")
                .item,
            IsoLiteralItem::Selectable(_)
        ));
        assert_eq!(literals[0].errors, vec![]);
        assert!(
            literals[0]
                .tokens
                .contains(&IsographSemanticToken::Keyword.with_span(span_of(
                    literals[0].extraction.item.0,
                    "field"
                )))
        );
    }

    #[test]
    fn two_literals_in_one_file() {
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let literals = file_literals(&TypeScriptHostLanguage, source);
        assert_eq!(literals.len(), 2);
        assert!(matches!(
            literals[1]
                .parse
                .as_ref()
                .expect("the fixture is not an empty literal")
                .item
                .item
                .item
                .item
                .as_ref()
                .expect("the fixture parsed a declaration")
                .item,
            IsoLiteralItem::Entrypoint(_)
        ));
    }
}
```

`with_span` needs `WithSpanPostfix` in the test module.

## Change 2: concatenate literals, call `lsp_semantic_tokens`

encoding.md landed `semantic_token_legend` and `lsp_semantic_tokens`. This change concatenates each `FileLiteral`'s tokens rebased with `with_offset(extraction.location.start)` and calls `lsp_semantic_tokens`. Origin concatenated by building `AbsoluteToken` via `split_inclusive('\n')` and byte-length `delta_line_delta_start`. encoding.md replaced that encoding.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use isograph_compiler::HostLanguage;
use span::WithSpanPostfix;

use crate::file_literals::file_literals;

pub fn lsp_tokens_for_file<THostLanguage: HostLanguage>(
    host: &THostLanguage,
    source: &str,
) -> Vec<lsp_types::SemanticToken> {
    let literals = file_literals(host, source);
    let mut tokens = Vec::new();
    for literal in &literals {
        tokens.extend(literal.tokens.iter().map(|token| {
            token
                .item
                .with_span(token.location.with_offset(literal.extraction.location.start))
        }));
    }
    lsp_semantic_tokens(&tokens, source)
}
```

`tokens` after the loop are offsets into `source`, in extraction order. `lsp_semantic_tokens` asserts they are ordered, exclusive, and have text. Extraction order is source order.

Tests:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
#[cfg(test)]
mod file_tests {
    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{CLASS, KEYWORD, PROPERTY, lsp_tokens_for_file};

    #[test]
    fn field_keyword_is_the_first_lsp_token() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let lsp = lsp_tokens_for_file(&TypeScriptHostLanguage, source);
        let field_at = source.find("field").expect("the fixture contains field") as u32;
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, field_at);
        assert_eq!(lsp[0].length, 5);
        assert_eq!(lsp[0].token_modifiers_bitset, 0);
    }

    #[test]
    fn pet_is_class_and_id_is_property() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let lsp = lsp_tokens_for_file(&TypeScriptHostLanguage, source);
        assert_eq!(lsp[1].token_type, CLASS);
        assert_eq!(lsp[5].token_type, PROPERTY);
    }

    #[test]
    fn two_literals_tokens_are_in_file_order() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let lsp = lsp_tokens_for_file(&TypeScriptHostLanguage, source);
        assert_eq!(lsp.len(), 8);
        assert_eq!(lsp[4].delta_line, 1);
        assert_eq!(lsp[4].delta_start, 5);
        assert_eq!(lsp[4].token_type, KEYWORD);
    }
}
```

`file_tests` is a second `#[cfg(test)]` module in `semantic_tokens.rs` so encoding.md's `tests` module stays as written. `KEYWORD` / `CLASS` / `PROPERTY` are the legend indices encoding.md already defines.

`field_keyword_is_the_first_lsp_token`: `field` is the first parser token in the extracted literal. `delta_start` is its file offset.

`pet_is_class_and_id_is_property`: `Pet` is index 1, `id` is index 5 (`field`, `Pet`, `.`, `fullName`, `{`, `id`).

`two_literals_tokens_are_in_file_order`: eight tokens. Index 4 is the second `entrypoint`. Previous piece is `A` on the previous line; `delta_line` 1, `delta_start` 5 (column of `entrypoint` after `iso(\``).

## Change 3: the server and `isograph lsp`

Dispatch types origin: isograph `lsp_request_dispatch.rs`, `lsp_notification_dispatch.rs`, `lsp_runtime_error.rs`. Delta: `extract` is `match` on the `Result`, not `expect` / `catch_unwind`. A failed extract is `LSPRuntimeError::ExpectedError`.

```rust
// from crates/isograph_lsp/src/lsp_runtime_error.rs
use lsp_server::{ErrorCode, ResponseError};

pub type LSPRuntimeResult<T> = Result<T, LSPRuntimeError>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LSPRuntimeError {
    ExpectedError,
    UnexpectedError(String),
}

impl From<LSPRuntimeError> for Option<ResponseError> {
    fn from(err: LSPRuntimeError) -> Self {
        match err {
            LSPRuntimeError::ExpectedError => None,
            LSPRuntimeError::UnexpectedError(message) => ResponseError {
                code: ErrorCode::UnknownErrorCode as i32,
                message,
                data: None,
            }
            .wrap_some(),
        }
    }
}
```

Request dispatch origin: isograph `lsp_request_dispatch.rs`. Delta: `extract_request_params` is:

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
fn extract_request_params<R>(
    req: lsp_server::Request,
) -> LSPRuntimeResult<(lsp_server::RequestId, R::Params)>
where
    R: lsp_types::request::Request,
{
    req.extract(R::METHOD)
        .map_err(|_| LSPRuntimeError::ExpectedError)
}
```

Origin of `LSPRequestDispatch::on_request_sync`: isograph `lsp_request_dispatch.rs`. Delta: `extract_request_params` as above; `Err(error)` passed into `convert_to_lsp_response` is `error.wrap_err()`.

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
    pub fn on_request_sync<TRequest: lsp_types::request::Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
        if self.request.method == TRequest::METHOD {
            match extract_request_params::<TRequest>(self.request) {
                Ok((request_id, params)) => {
                    let response = handler(self.state, params).and_then(|handler_result| {
                        serde_json::to_value(handler_result).map_err(|_err| {
                            LSPRuntimeError::UnexpectedError(
                                "Unable to serialize request response".to_string(),
                            )
                        })
                    });
                    let server_response = convert_to_lsp_response(request_id, response);
                    return ControlFlow::Break(server_response);
                }
                Err(error) => {
                    return ControlFlow::Break(convert_to_lsp_response(
                        lsp_server::RequestId::from("default-lsp-id".to_string()),
                        error.wrap_err(),
                    ));
                }
            }
        }
        ControlFlow::Continue(self)
    }

pub(crate) fn convert_to_lsp_response(
    id: lsp_server::RequestId,
    result: LSPRuntimeResult<serde_json::Value>,
) -> Response {
    match result {
        Ok(value) => Response {
            id,
            result: value.wrap_some(),
            error: None,
        },
        Err(LSPRuntimeError::ExpectedError) => Response {
            id,
            result: serde_json::Value::Null.wrap_some(),
            error: None,
        },
        Err(runtime_error) => {
            let response_error: Option<ResponseError> = runtime_error.to();
            let response_error = response_error.unwrap_or_else(|| ResponseError {
                code: ErrorCode::UnknownErrorCode as i32,
                message: "Request Canceled".to_string(),
                data: None,
            });
            Response {
                id,
                result: None,
                error: response_error.wrap_some(),
            }
        }
    }
}
```

Notification dispatch origin: isograph `lsp_notification_dispatch.rs`. Delta: extract is:

```rust
// from crates/isograph_lsp/src/lsp_notification_dispatch.rs
    pub fn on_notification_sync<TNotification: lsp_types::notification::Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params) -> LSPRuntimeResult<()>,
    ) -> ControlFlow<Option<LSPRuntimeError>, Self> {
        if self.notification.method == TNotification::METHOD {
            let params = match self.notification.extract(TNotification::METHOD) {
                Ok(params) => params,
                Err(_) => return ControlFlow::Break(None),
            };
            let response = handler(self.state, params);
            return ControlFlow::Break(response.err());
        }
        ControlFlow::Continue(self)
    }
```

Open-file notifications:

```rust
// from crates/isograph_lsp/src/text_document.rs
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, TextDocumentItem,
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification},
};
use prelude::Postfix;

use isograph_compiler::HostLanguage;

use crate::{lsp_runtime_error::LSPRuntimeResult, lsp_state::LspState};

pub fn on_did_open_text_document<THostLanguage: HostLanguage>(
    state: &mut LspState<THostLanguage>,
    params: <DidOpenTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidOpenTextDocumentParams { text_document } = params;
    let TextDocumentItem { text, uri, .. } = text_document;
    state.open_files.insert(uri.as_str().to_owned(), text);
    ().wrap_ok()
}

pub fn on_did_close_text_document<THostLanguage: HostLanguage>(
    state: &mut LspState<THostLanguage>,
    params: <DidCloseTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    state
        .open_files
        .remove(params.text_document.uri.as_str());
    ().wrap_ok()
}

pub fn on_did_change_text_document<THostLanguage: HostLanguage>(
    state: &mut LspState<THostLanguage>,
    params: <DidChangeTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidChangeTextDocumentParams {
        content_changes,
        text_document,
    } = params;
    let Some(content_changed) = content_changes.first() else {
        return ().wrap_ok();
    };
    state.open_files.insert(
        text_document.uri.as_str().to_owned(),
        content_changed.text.to_owned(),
    );
    ().wrap_ok()
}
```

Origin: isograph `text_document.rs`. Delta: `HashMap` by URI string, no pico, no relative path, no `expect` on `to_file_path` or `content_changes.first()`.

Semantic tokens request:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use lsp_types::{
    SemanticTokens as LspSemanticTokens, SemanticTokensResult as LspSemanticTokensResult,
    request::{Request, SemanticTokensFullRequest},
};

use isograph_compiler::HostLanguage;

use crate::{lsp_runtime_error::LSPRuntimeResult, lsp_state::LspState};

pub fn on_semantic_token_full_request<THostLanguage: HostLanguage>(
    state: &LspState<THostLanguage>,
    params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    let uri = params.text_document.uri;
    let Some(source) = state.open_files.get(uri.as_str()) else {
        return None.wrap_ok();
    };
    LspSemanticTokensResult::Tokens(LspSemanticTokens {
        result_id: None,
        data: lsp_tokens_for_file(&state.host, source),
    })
    .wrap_some()
    .wrap_ok()
}
```

Server:

```rust
// from crates/isograph_lsp/src/server.rs
use std::ops::ControlFlow;

use std::collections::HashMap;

use lsp_server::{Connection, ErrorCode, Response, ResponseError};
use lsp_types::{
    InitializeParams, SemanticTokensFullOptions, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
    request::SemanticTokensFullRequest,
};
use isograph_compiler::HostLanguage;
use prelude::Postfix;

use crate::{
    lsp_notification_dispatch::LSPNotificationDispatch,
    lsp_request_dispatch::LSPRequestDispatch,
    lsp_runtime_error::LSPRuntimeError,
    lsp_state::LspState,
    semantic_tokens::{on_semantic_token_full_request, semantic_token_legend},
    text_document::{
        on_did_change_text_document, on_did_close_text_document, on_did_open_text_document,
    },
};

pub fn start<THostLanguage: HostLanguage>(host: THostLanguage) -> std::process::ExitCode {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        text_document_sync: TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)
            .wrap_some(),
        semantic_tokens_provider: SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: semantic_token_legend(),
                range: None,
                full: SemanticTokensFullOptions::Bool(true).wrap_some(),
            },
        )
        .wrap_some(),
        ..Default::default()
    };
    let capabilities = serde_json::to_value(&capabilities)
        .expect("ServerCapabilities serializes");
    if let Err(err) = connection.initialize(capabilities) {
        tracing::error!("failed to initialize the language server: {err}");
        return std::process::ExitCode::FAILURE;
    }
    run(connection, host);
    if let Err(err) = io_threads.join() {
        tracing::error!("language server io threads: {err}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run<THostLanguage: HostLanguage>(connection: Connection, host: THostLanguage) {
    let mut state = LspState {
        open_files: HashMap::new(),
        host,
    };
    for msg in &connection.receiver {
        match msg {
            lsp_server::Message::Request(request) => {
                if connection.handle_shutdown(&request).unwrap_or(false) {
                    break;
                }
                let response = dispatch_request(request, &state);
                if connection.sender.send(response.to()).is_err() {
                    break;
                }
            }
            lsp_server::Message::Notification(notification) => {
                let _ = dispatch_notification(notification, &mut state);
            }
            lsp_server::Message::Response(_) => {}
        }
    }
}

fn dispatch_notification<THostLanguage: HostLanguage>(
    notification: lsp_server::Notification,
    state: &mut LspState<THostLanguage>,
) -> ControlFlow<Option<LSPRuntimeError>, ()> {
    LSPNotificationDispatch::new(notification, state)
        .on_notification_sync::<DidOpenTextDocument>(on_did_open_text_document)?
        .on_notification_sync::<DidCloseTextDocument>(on_did_close_text_document)?
        .on_notification_sync::<DidChangeTextDocument>(on_did_change_text_document)?
        .notification();
    ControlFlow::Continue(())
}

fn dispatch_request<THostLanguage: HostLanguage>(
    request: lsp_server::Request,
    state: &LspState<THostLanguage>,
) -> Response {
    let get_response = || {
        let request = LSPRequestDispatch::new(request, state)
            .on_request_sync::<SemanticTokensFullRequest>(on_semantic_token_full_request)?
            .request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => response,
        ControlFlow::Continue(request) => Response {
            id: request.id,
            result: None,
            error: ResponseError {
                code: ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!("No handler registered for method '{}'", request.method),
            }
            .wrap_some(),
        },
    }
}
```

`handle_shutdown` returns `Result<bool, ProtocolError>`. `unwrap_or(false)` continues if the check itself fails. Origin isograph uses a tokio loop; this is a blocking `for` over `connection.receiver`. `Connection::handle_shutdown` is the stdio shutdown handshake.

`server.rs` imports `std::collections::HashMap`. `run` holds `let mut state = LspState { open_files: HashMap::new(), host }`, passes `&state` to `dispatch_request` and `&mut state` to `dispatch_notification`. Prefix `&` / `&mut` stay: `reference_mut` does not exist.

`expect` on `ServerCapabilities` serialize: the value is a struct literal in this file. `serde_json::to_value` fails only if a type in `lsp_types` refuses to serialize; the type system cannot check that.

Tests for open-file state:

```rust
// from crates/isograph_lsp/src/text_document.rs
#[cfg(test)]
mod tests {
    use lsp_types::{
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, TextDocumentContentChangeEvent,
        TextDocumentItem, VersionedTextDocumentIdentifier,
        notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification},
    };
    use prelude::Postfix;

    use super::{
        on_did_change_text_document, on_did_close_text_document, on_did_open_text_document,
    };
    use isograph_extract_typescript::TypeScriptHostLanguage;

    use crate::lsp_state::LspState;

    fn uri() -> lsp_types::Uri {
        "file:///tmp/Pet.tsx".parse().expect("the fixture URI parses")
    }

    #[test]
    fn did_open_stores_the_text() {
        let mut state = LspState {
            open_files: Default::default(),
            host: TypeScriptHostLanguage,
        };
        on_did_open_text_document(
            &mut state,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri(),
                    language_id: "typescript".to_owned(),
                    version: 1,
                    text: "iso(`entrypoint Query.HomeRoute`)".to_owned(),
                },
            },
        )
        .expect("didOpen succeeds");
        assert_eq!(
            state.open_files.get(uri().as_str()).map(String::as_str),
            "iso(`entrypoint Query.HomeRoute`)".wrap_some()
        );
    }

    #[test]
    fn did_change_replaces_the_text() {
        let mut state = LspState {
            open_files: Default::default(),
            host: TypeScriptHostLanguage,
        };
        state
            .open_files
            .insert(uri().as_str().to_owned(), "old".to_owned());
        on_did_change_text_document(
            &mut state,
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri(),
                    version: 2,
                },
                content_changes: TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "iso(`field Pet.fullName { id }`)".to_owned(),
                }
                .wrap_vec(),
            },
        )
        .expect("didChange succeeds");
        assert_eq!(
            state.open_files.get(uri().as_str()).map(String::as_str),
            "iso(`field Pet.fullName { id }`)".wrap_some()
        );
    }

    #[test]
    fn did_close_removes_the_text() {
        let mut state = LspState {
            open_files: Default::default(),
            host: TypeScriptHostLanguage,
        };
        state
            .open_files
            .insert(uri().as_str().to_owned(), "iso(`entrypoint Query.A`)".to_owned());
        on_did_close_text_document(
            &mut state,
            lsp_types::DidCloseTextDocumentParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: uri() },
            },
        )
        .expect("didClose succeeds");
        assert!(state.open_files.get(uri().as_str()).is_none());
    }
}
```

`on_semantic_token_full_request` test: insert a file, request tokens, assert nonempty and the first token's `token_type` is KEYWORD when the literal starts with `field` after only whitespace.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
    #[test]
    fn request_uses_the_open_file_text() {
        let mut state = LspState {
            open_files: Default::default(),
            host: TypeScriptHostLanguage,
        };
        let uri: lsp_types::Uri = "file:///tmp/Pet.tsx"
            .parse()
            .expect("the fixture URI parses");
        state.open_files.insert(
            uri.as_str().to_owned(),
            "iso(`field Pet.fullName { id }`)".to_owned(),
        );
        let result = on_semantic_token_full_request(
            &state,
            lsp_types::SemanticTokensParams {
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                text_document: lsp_types::TextDocumentIdentifier { uri },
            },
        )
        .expect("the request succeeds")
        .expect("the file is open");
        match result {
            LspSemanticTokensResult::Tokens(tokens) => {
                assert!(!tokens.data.is_empty());
                assert_eq!(tokens.data[0].token_type, 15);
            }
            LspSemanticTokensResult::Partial(_) => {
                assert!(false, "the fixture returns Tokens, not Partial");
            }
        }
    }
```

`expect` in tests names an invariant the test established.

CLI. ts-graphql-react-isograph-cli.md already split the library and the binary. This change makes `run` take a host and adds `isograph lsp`. `ts_graphql_react_isograph_cli` is the crate that names `TypeScriptHostLanguage`.

```toml
# from crates/isograph_cli/Cargo.toml
isograph_compiler = { path = "../isograph_compiler" }
isograph_lsp = { path = "../isograph_lsp" }
```

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml
isograph_extract_typescript = { path = "../isograph_extract_typescript" }
```

```rust
// from crates/isograph_cli/src/lib.rs
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser};
use freddie_cli::{App, Instance, NoArgs};
use isograph_compiler::HostLanguage;
use prelude::Postfix;

pub struct IsographCli<THostLanguage: HostLanguage> {
    pub host: THostLanguage,
}

#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Language server for the VS Code extension.
    Lsp(LspCommand),
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph>),
}

#[derive(clap::Args, Debug)]
struct LspCommand {
    /// Accepted so the VS Code extension's --config flag does not fail clap.
    #[arg(long)]
    config: Option<PathBuf>,
}

impl<THostLanguage: HostLanguage> IsographCli<THostLanguage> {
    pub fn run(self) -> ExitCode {
        let matches = Args::command().get_matches();
        let cli = Args::from_arg_matches(matches.reference())
            .expect("the derived type matches the command it derived");

        match cli.command {
            Some(Command::Lsp(_)) => isograph_lsp::start(self.host),
            Some(Command::Lifecycle(verb)) => {
                freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference())
            }
            None => freddie_cli::run_lifecycle_verb::<Isograph>(
                freddie_cli::verb_for_bare_invocation::<Isograph>(),
                matches.reference(),
            ),
        }
    }
}
```

`Isograph`, `IsographArgs`, and `impl App` stay in `isograph_cli`. Clap type is `Args` (before, ts-graphql-react-isograph-cli.md: `Cli` with `verb` only). `run` is a method on `IsographCli<THostLanguage>` (before: `pub fn run()`).

```rust
// from crates/ts_graphql_react_isograph_cli/src/main.rs
use std::process::ExitCode;

use isograph_cli::IsographCli;
use isograph_extract_typescript::TypeScriptHostLanguage;

fn main() -> ExitCode {
    IsographCli {
        host: TypeScriptHostLanguage,
    }
    .run()
}
```

Before (ts-graphql-react-isograph-cli.md): `fn main() -> ExitCode { isograph_cli::run() }`.

`LspCommand.config` is unread. The extension in `languageClient.ts` already pushes `--config` when `pathToConfig` is set.

## Order

1. Change 1; crate, `file_literals`, its tests.
2. Change 2; legend, absolutize, encoding, its tests.
3. Change 3; dispatch, open files, server loop, `isograph lsp`, `IsographCli<THostLanguage>`, handler tests.
