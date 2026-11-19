use std::ops::ControlFlow;

use lsp_server::{ErrorCode, ResponseError};
use lsp_types::request::Request;
use prelude::Postfix;
use serde_json::Value;

use crate::event::LspRequest;
use isograph_compiler::ClientId;

use isograph_lsp::lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult};

/// The `result` or the `error` of a response.
pub type ResponseBody = Result<Value, ResponseError>;

pub struct LSPRequestDispatch<'state, TState> {
    client: ClientId,
    request: lsp_server::Request,
    state: &'state TState,
}

impl<'state, TState> LSPRequestDispatch<'state, TState> {
    pub fn new(incoming: LspRequest, state: &'state TState) -> Self {
        LSPRequestDispatch {
            client: incoming.client,
            request: incoming.request,
            state,
        }
    }

    /// Calls handler if the LSPRequestDispatch's request's method matches the method
    /// of TRequest. Returns a ControlFlow which will be Break if the handler was called,
    /// or Continue otherwise.
    /// Thus, multiple calls to `on_request_sync(...)?` can be chained. Doing so will
    /// cause LSPRequestDispatch to execute the first matching handler, if any.
    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, ClientId, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<ResponseBody, Self> {
        if self.request.method != TRequest::METHOD {
            return ControlFlow::Continue(self);
        }
        ControlFlow::Break(
            extract_request_params::<TRequest>(self.request)
                .and_then(|params| handler(self.state, self.client, params))
                .and_then(|handler_result| {
                    serde_json::to_value(handler_result).map_err(|_err| {
                        LSPRuntimeError::UnexpectedError(
                            "Unable to serialize request response".to_string(),
                        )
                    })
                })
                .map_or_else(runtime_error_body, Value::wrap_ok),
        )
    }

    pub fn request(self) -> lsp_server::Request {
        self.request
    }
}

fn runtime_error_body(error: LSPRuntimeError) -> ResponseBody {
    match error {
        LSPRuntimeError::ExpectedError => Value::Null.wrap_ok(),
        LSPRuntimeError::UnexpectedError(message) => ResponseError {
            code: ErrorCode::UnknownErrorCode as i32,
            message,
            data: None,
        }
        .wrap_err(),
    }
}

fn extract_request_params<R>(req: lsp_server::Request) -> LSPRuntimeResult<R::Params>
where
    R: Request,
{
    std::panic::catch_unwind(|| {
        req.extract(R::METHOD)
            .expect("extract_request_params: could not extract request params")
    })
    .map(|(_id, params)| params)
    .map_err(|err| {
        LSPRuntimeError::UnexpectedError(format!("panic in the `extract_request_params`: {err:?}"))
    })
}

#[cfg(test)]
mod test {
    use std::{
        ops::ControlFlow,
        str::FromStr,
        sync::atomic::{AtomicI32, Ordering},
    };

    use lsp_types::{
        Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
        request::{GotoDefinition, HoverRequest, Request},
    };

    use super::LSPRequestDispatch;
    use crate::event::LspRequest;
    use isograph_compiler::ClientId;
    use isograph_lsp::lsp_runtime_error::LSPRuntimeResult;

    #[test]
    fn calls_first_matching_request_handler() {
        let state: AtomicI32 = AtomicI32::new(0);
        let dispatch = LSPRequestDispatch::new(
            LspRequest {
                client: ClientId(1),
                request: lsp_server::Request {
                    id: "id".to_string().into(),
                    method: "textDocument/definition".to_string(),
                    params: serde_json::to_value(TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier {
                            uri: Uri::from_str("https://example.net").unwrap(),
                        },
                        position: Position {
                            line: 0,
                            character: 0,
                        },
                    })
                    .unwrap(),
                },
            },
            &state,
        );
        let dispatch = || {
            dispatch
                .on_request_sync::<HoverRequest>(hover_handler)?
                .on_request_sync::<GotoDefinition>(goto_definition_handler)?;
            ControlFlow::Continue(())
        };
        let result = dispatch();
        assert!(result.is_break());
        assert_eq!(state.load(Ordering::Relaxed), 2);
    }

    fn hover_handler(
        state: &AtomicI32,
        _client: ClientId,
        _params: <HoverRequest as Request>::Params,
    ) -> LSPRuntimeResult<<HoverRequest as Request>::Result> {
        state.store(1, Ordering::Relaxed);

        Ok(None)
    }

    fn goto_definition_handler(
        state: &AtomicI32,
        _client: ClientId,
        _params: <GotoDefinition as Request>::Params,
    ) -> LSPRuntimeResult<<GotoDefinition as Request>::Result> {
        state.store(2, Ordering::Relaxed);

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::request::CodeActionRequest;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_extract_request_params_error() {
        let request_id = lsp_server::RequestId::from("test-id".to_string());
        let request = lsp_server::Request {
            id: request_id,
            method: "textDocument/codeAction".to_string(),
            params: json!({
                "invalid_key": "invalid_value",
            }),
        };
        let result = extract_request_params::<CodeActionRequest>(request);
        // it returns an error, but not panic
        assert!(result.is_err());
    }
}
