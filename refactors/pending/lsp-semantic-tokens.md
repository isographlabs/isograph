# LSP semantic tokens

Requires extract-iso-literals.md. Opening a JavaScript or TypeScript file in VS Code colors the contents of each `iso(\`...\`)` (and `iso\`...\``) according to the grammar: `field` / `entrypoint` / `to` as keywords, type names as classes, field and selection names as properties, and so on.

The VS Code extension already starts `isograph lsp` on those languages (`vscode-extension/src/languageClient.ts`). This doc makes that process a language server that answers `textDocument/semanticTokens/full`. The extension is unchanged.

The server is isograph's LSP: stdio, `lsp-server` + `lsp-types`, full text-document sync, a semantic-tokens legend, `didOpen` / `didChange` / `didClose`, and the same absolutize-then-delta-encode walk as `crates/isograph_lsp/src/semantic_tokens.rs` in isograph. It has no pico, no schema, no file watcher. Open-file text is a `HashMap` keyed by URI string. `--config` is accepted so the extension's flag does not fail clap, and is unused.

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
use isograph_parser::{
    HostLanguage, IsoLiteralError, IsoLiteralExtraction, IsoLiteralParse, SemanticToken,
    parse_iso_literal,
};
use span::WithSpan;

pub struct FileLiteral<'a, THostLanguage: HostLanguage> {
    pub extraction: WithSpan<IsoLiteralExtraction<'a, THostLanguage>>,
    pub parse: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<IsoLiteralError<THostLanguage>>>,
    pub tokens: Vec<WithSpan<SemanticToken>>,
}

pub fn file_literals<THostLanguage: HostLanguage>(
    host: &THostLanguage,
    source: &str,
) -> Vec<FileLiteral<'_, THostLanguage>> {
    host.extract_iso_literals(source)
        .into_iter()
        .map(|extracted| {
            let extraction = extracted.item;
            let text = extraction.item.iso_literal_text;
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

`parse` is `None` when `ParsedIsoLiteral.item` is `None` (empty literal). `tokens` are literal-relative, consume order, whatever the grammar recorded. extra and extra_chunks leftover is leftover-semantic-tokens.md. The matcher's cut is not filled in. `errors` is `WithErrors.errors`: `IsoLiteralError` (`Host`, `Parse`, `Bracket`, `Comma`), already file-absolute.

```rust
// from crates/isograph_lsp/src/lsp_state.rs
use std::collections::HashMap;

pub struct LspState {
    pub open_files: HashMap<String, String>,
}
```

Key is `uri.as_str()`. Value is the full document text from `didOpen` / `didChange`.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
pub struct AbsoluteToken {
    pub absolute_char_start: u32,
    pub len: u32,
    pub semantic_token: SemanticToken,
}
```

Origin: `AbsoluteIsographSemanticToken` in isograph's `semantic_tokens.rs`. Delta: `IsographSemanticToken` is i2's `SemanticToken`.

## Change 1: `file_literals`

New crate `crates/isograph_lsp`. Workspace member via `./crates/*`.

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
isograph_extract_typescript = { path = "../isograph_extract_typescript", optional = true }
isograph_parser = { path = "../isograph_parser" }
lsp-server = { workspace = true }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
serde_json = { workspace = true }
span = { path = "../span" }
tracing = { workspace = true }

[features]
default = ["typescript"]
typescript = ["dep:isograph_extract_typescript"]

[lints]
workspace = true
```

`src/lib.rs`:

```rust
// from crates/isograph_lsp/src/lib.rs
mod file_literals;
mod lsp_notification_dispatch;
mod lsp_request_dispatch;
mod lsp_runtime_error;
mod lsp_state;
mod semantic_tokens;
mod server;
mod text_document;

#[cfg(feature = "typescript")]
pub use isograph_extract_typescript::TypeScriptHostLanguage;

pub fn start() -> std::process::ExitCode;
```

`file_literals.rs` is the types above. Tests in that module:

```rust
// from crates/isograph_lsp/src/file_literals.rs
#[cfg(test)]
mod tests {
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use isograph_parser::{IsoLiteralItem, SemanticToken};
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
            literals[0].extraction.item.context.const_export_name,
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
                .contains(&SemanticToken::Keyword.with_span(span_of(
                    literals[0].extraction.item.iso_literal_text,
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

## Change 2: legend, absolutize, LSP encoding

Origin: isograph `crates/isograph_lsp/src/semantic_tokens.rs` and `crates/isograph_lang_types/src/semantic_token_legend/mod.rs`. The legend token-type list is identical, in the same order, so the indices match isograph's `LspSemanticToken(n)` constants.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use isograph_parser::SemanticToken;
use lsp_types::{
    SemanticToken as LspSemanticToken, SemanticTokenModifier, SemanticTokenType,
    SemanticTokensLegend,
};

pub fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::DECORATOR,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
        ],
    }
}

const LSP_ST_TYPE: u32 = 1;
const LSP_ST_CLASS: u32 = 2;
const LSP_ST_PARAMETER: u32 = 7;
const LSP_ST_VARIABLE: u32 = 8;
const LSP_ST_PROPERTY: u32 = 9;
const LSP_ST_KEYWORD: u32 = 15;
const LSP_ST_COMMENT: u32 = 17;
const LSP_ST_STRING: u32 = 18;
const LSP_ST_NUMBER: u32 = 19;
const LSP_ST_OPERATOR: u32 = 21;
const LSP_ST_DECORATOR: u32 = 22;

pub fn lsp_type_index(token: SemanticToken) -> u32 {
    match token {
        SemanticToken::Keyword => LSP_ST_KEYWORD,
        SemanticToken::Type => LSP_ST_CLASS,
        SemanticToken::FieldName => LSP_ST_PROPERTY,
        SemanticToken::ObjectKey => LSP_ST_PROPERTY,
        SemanticToken::GraphQLTypeName => LSP_ST_TYPE,
        SemanticToken::DirectiveName => LSP_ST_DECORATOR,
        SemanticToken::Variable => LSP_ST_VARIABLE,
        SemanticToken::Argument => LSP_ST_PARAMETER,
        SemanticToken::Integer => LSP_ST_NUMBER,
        SemanticToken::String => LSP_ST_STRING,
        SemanticToken::BooleanOrNull => LSP_ST_VARIABLE,
        SemanticToken::Period
        | SemanticToken::Colon
        | SemanticToken::Equals
        | SemanticToken::Parenthesis
        | SemanticToken::Brace
        | SemanticToken::Content
        | SemanticToken::Bracket => LSP_ST_OPERATOR,
        SemanticToken::Error => LSP_ST_COMMENT,
    }
}
```

Delta from isograph's per-constant `lsp_semantic_token`: i2 has one `SemanticToken` enum. `FieldName` is PROPERTY (isograph uses METHOD for `Type.name` and PROPERTY for selections). `Error` is COMMENT. `Content` is OPERATOR.

Absolutize and encode, origin isograph `semantic_tokens.rs`. Delta: `text_source.span` is `extraction.location`; `IsographSemanticToken` is `SemanticToken`; no pico; no `uri_is_project_file`; `page_content` is the open file text.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use isograph_extract_typescript::TypeScriptHostLanguage;
use isograph_parser::HostLanguage;

use crate::file_literals::{FileLiteral, file_literals};

pub fn lsp_tokens_for_file(source: &str) -> Vec<LspSemanticToken> {
    let literals = file_literals(&TypeScriptHostLanguage, source);
    let absolute = concatenate_and_absolutize(literals.iter(), source);
    convert_absolute_token_to_lsp_token(absolute, source).collect()
}

fn concatenate_and_absolutize<'a, THostLanguage: HostLanguage>(
    literals: impl Iterator<Item = &'a FileLiteral<'a, THostLanguage>> + 'a,
    page_content: &'a str,
) -> impl Iterator<Item = AbsoluteToken> + 'a {
    literals.flat_map(move |literal| {
        let iso_literal_extraction_span = literal.extraction.location;
        literal.tokens.iter().flat_map(move |relative_token| {
            absolutize_relative_token(page_content, iso_literal_extraction_span, relative_token)
        })
    })
}

fn absolutize_relative_token<'a>(
    page_content: &'a str,
    iso_literal_extraction_span: span::Span,
    relative_token: &'a WithSpan<SemanticToken>,
) -> impl Iterator<Item = AbsoluteToken> + 'a {
    let start = iso_literal_extraction_span.start as usize
        + relative_token.location.start as usize;
    let end = iso_literal_extraction_span.start as usize
        + relative_token.location.end as usize;
    let span_content = &page_content[start..end];
    span_content
        .split_inclusive('\n')
        .scan(0, move |iterated_so_far_within_token, line_text| {
            let token = AbsoluteToken {
                absolute_char_start: iso_literal_extraction_span.start
                    + relative_token.location.start
                    + *iterated_so_far_within_token,
                len: line_text.len() as u32,
                semantic_token: relative_token.item,
            };
            *iterated_so_far_within_token += line_text.len() as u32;
            token.wrap_some()
        })
}

fn convert_absolute_token_to_lsp_token<'a>(
    absolute_tokens: impl Iterator<Item = AbsoluteToken> + 'a,
    page_content: &'a str,
) -> impl Iterator<Item = LspSemanticToken> + 'a {
    absolute_tokens.scan(0, |last_token_start, absolute_token| {
        let new_token_start = absolute_token.absolute_char_start;
        let in_between_content =
            &page_content[(*last_token_start as usize)..(new_token_start as usize)];
        let (delta_line, delta_start) = delta_line_delta_start(in_between_content);
        let token = LspSemanticToken {
            delta_line,
            delta_start,
            length: absolute_token.len,
            token_type: lsp_type_index(absolute_token.semantic_token),
            token_modifiers_bitset: 0,
        };
        *last_token_start = absolute_token.absolute_char_start;
        token.wrap_some()
    })
}

pub fn delta_line_delta_start(text: &str) -> (u32, u32) {
    let mut last_line_break_index = 0;
    let mut line_break_count = 0;
    for (index, char) in text.chars().enumerate() {
        if char == '\n' {
            line_break_count += 1;
            last_line_break_index = index as u32 + 1;
        }
    }
    (line_break_count, text.len() as u32 - last_line_break_index)
}
```

Origin of `absolutize_relative_token`, `convert_absolute_token_to_lsp_token`, `delta_line_delta_start`: isograph `semantic_tokens.rs`, verbatim except the type names above.

Tests:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
#[cfg(test)]
mod tests {
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use isograph_parser::SemanticToken;
    use prelude::Postfix;

    use super::{
        AbsoluteToken, concatenate_and_absolutize, delta_line_delta_start, lsp_type_index,
        lsp_tokens_for_file,
    };
    use crate::file_literals::file_literals;

    fn absolute_for(source: &str) -> Vec<AbsoluteToken> {
        let literals = file_literals(&TypeScriptHostLanguage, source);
        concatenate_and_absolutize(literals.iter(), source).collect()
    }

    fn covering<'a>(
        tokens: &'a [AbsoluteToken],
        source: &str,
        pattern: &str,
    ) -> &'a AbsoluteToken {
        let mut occurrences = source.match_indices(pattern);
        let (start, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the file");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the file"
        );
        let start = start as u32;
        let end = start + pattern.len() as u32;
        tokens
            .iter()
            .find(|token| {
                token.absolute_char_start <= start
                    && token.absolute_char_start + token.len >= end
            })
            .expect("a token covers the pattern")
    }

    #[test]
    fn field_keyword_is_keyword_at_its_file_offset() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let tokens = absolute_for(source);
        let token = covering(&tokens, source, "field");
        assert_eq!(token.semantic_token, SemanticToken::Keyword);
        assert_eq!(lsp_type_index(token.semantic_token), 15);
    }

    #[test]
    fn pet_is_class_and_id_is_property() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let tokens = absolute_for(source);
        assert_eq!(
            covering(&tokens, source, "Pet").semantic_token,
            SemanticToken::Type
        );
        assert_eq!(
            covering(&tokens, source, "id").semantic_token,
            SemanticToken::FieldName
        );
    }

    #[test]
    fn two_literals_tokens_are_file_absolute() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let tokens = absolute_for(source);
        let a = covering(&tokens, source, "A");
        let b = covering(&tokens, source, "B");
        assert!(b.absolute_char_start > a.absolute_char_start);
        assert_eq!(&source[a.absolute_char_start as usize..][..1], "A");
        assert_eq!(&source[b.absolute_char_start as usize..][..1], "B");
    }

    #[test]
    fn delta_line_delta_start_same_line() {
        assert_eq!(delta_line_delta_start("   "), (0, 3));
    }

    #[test]
    fn delta_line_delta_start_newline() {
        assert_eq!(delta_line_delta_start("\n  "), (1, 2));
    }

    #[test]
    fn lsp_tokens_for_a_file_are_nonempty() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let lsp = lsp_tokens_for_file(source);
        assert!(!lsp.is_empty());
        assert_eq!(lsp[0].token_modifiers_bitset, 0);
    }
}
```

`concatenate_and_absolutize` is `pub(crate)` so the tests in this module can call it. If it stays private, the tests live in the same module and call it.

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

use crate::{lsp_runtime_error::LSPRuntimeResult, lsp_state::LspState};

pub fn on_did_open_text_document(
    state: &mut LspState,
    params: <DidOpenTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidOpenTextDocumentParams { text_document } = params;
    let TextDocumentItem { text, uri, .. } = text_document;
    state.open_files.insert(uri.as_str().to_owned(), text);
    ().wrap_ok()
}

pub fn on_did_close_text_document(
    state: &mut LspState,
    params: <DidCloseTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    state
        .open_files
        .remove(params.text_document.uri.as_str());
    ().wrap_ok()
}

pub fn on_did_change_text_document(
    state: &mut LspState,
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

use crate::{lsp_runtime_error::LSPRuntimeResult, lsp_state::LspState};

pub fn on_semantic_token_full_request(
    state: &LspState,
    params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    let uri = params.text_document.uri;
    let Some(source) = state.open_files.get(uri.as_str()) else {
        return None.wrap_ok();
    };
    LspSemanticTokensResult::Tokens(LspSemanticTokens {
        result_id: None,
        data: lsp_tokens_for_file(source),
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

pub fn start() -> std::process::ExitCode {
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
    run(connection);
    if let Err(err) = io_threads.join() {
        tracing::error!("language server io threads: {err}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run(connection: Connection) {
    let mut state = LspState {
        open_files: HashMap::new(),
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

fn dispatch_notification(
    notification: lsp_server::Notification,
    state: &mut LspState,
) -> ControlFlow<Option<LSPRuntimeError>, ()> {
    LSPNotificationDispatch::new(notification, state)
        .on_notification_sync::<DidOpenTextDocument>(on_did_open_text_document)?
        .on_notification_sync::<DidCloseTextDocument>(on_did_close_text_document)?
        .on_notification_sync::<DidChangeTextDocument>(on_did_change_text_document)?
        .notification();
    ControlFlow::Continue(())
}

fn dispatch_request(request: lsp_server::Request, state: &LspState) -> Response {
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

`server.rs` imports `std::collections::HashMap`. `run` holds `let mut state = LspState { open_files: HashMap::new() }`, passes `&state` to `dispatch_request` and `&mut state` to `dispatch_notification`. Prefix `&` / `&mut` stay: `reference_mut` does not exist.

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
    use crate::lsp_state::LspState;

    fn uri() -> lsp_types::Uri {
        "file:///tmp/Pet.tsx".parse().expect("the fixture URI parses")
    }

    #[test]
    fn did_open_stores_the_text() {
        let mut state = LspState {
            open_files: Default::default(),
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

CLI. `crates/isograph_cli/Cargo.toml` gains:

```toml
isograph_lsp = { path = "../isograph_lsp" }
```

`main.rs`:

```rust
// from crates/isograph_cli/src/main.rs
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser};
use freddie_cli::{App, Instance, NoArgs};
use prelude::Postfix;

#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct IsographCli {
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

fn main() -> ExitCode {
    let matches = IsographCli::command().get_matches();
    let cli = IsographCli::from_arg_matches(matches.reference())
        .expect("the derived type matches the command it derived");

    match cli.command {
        Some(Command::Lsp(_)) => isograph_lsp::start(),
        Some(Command::Lifecycle(verb)) => {
            freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference())
        }
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            matches.reference(),
        ),
    }
}
```

Before: `verb: Option<freddie_cli::Verb<Isograph>>` only. `Isograph`, `IsographArgs`, and `impl App` stay.

`LspCommand.config` is unread. The extension in `languageClient.ts` already pushes `--config` when `pathToConfig` is set.

## Order

1. Change 1; crate, `file_literals`, its tests.
2. Change 2; legend, absolutize, encoding, its tests.
3. Change 3; dispatch, open files, server loop, `isograph lsp`, handler tests.
