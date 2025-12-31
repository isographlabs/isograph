use std::ops::ControlFlow;

use isograph_schema::CompilationProfile;
use lsp_types::request::{ExecuteCommand, Request};
use serde_json::Value;

use crate::{
    commands::IsographLspCommand,
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    lsp_state::LspState,
};

pub(crate) struct LspIsographCommandDispatch<'state, TCompilationProfile: CompilationProfile> {
    params: <ExecuteCommand as Request>::Params,
    // TODO make this generic over TState like the other dispatch structs
    state: &'state LspState<'state, TCompilationProfile>,
}

impl<'state, TCompilationProfile: CompilationProfile>
    LspIsographCommandDispatch<'state, TCompilationProfile>
{
    pub fn new(
        params: <ExecuteCommand as Request>::Params,
        state: &'state LspState<TCompilationProfile>,
    ) -> Self {
        LspIsographCommandDispatch { params, state }
    }

    /// Calls handler if the LSPRequestDispatch's request's method matches the method
    /// of TRequest. Returns a ControlFlow which will be Break if the handler was called,
    /// or Continue otherwise.
    /// Thus, multiple calls to `on_request_sync(...)?` can be chained. Doing so will
    /// cause LSPRequestDispatch to execute the first matching handler, if any.
    pub fn on_command_sync<TCommand: IsographLspCommand>(
        self,
    ) -> ControlFlow<LSPRuntimeResult<Option<Value>>, Self> {
        if self.params.command == TCommand::METHOD {
            let result: LSPRuntimeResult<_> = (|| {
                let first_param = self.params.arguments.into_iter().next().ok_or_else(|| {
                    LSPRuntimeError::UnexpectedError(
                        "Expected one param. \
                        This is indicative of a bug in Isograph."
                            .to_string(),
                    )
                })?;

                let converted_param = serde_json::from_value(first_param).map_err(|_| {
                    LSPRuntimeError::UnexpectedError(
                        "Unable to deserialize param. \
                        This is indicative of a bug in Isograph."
                            .to_string(),
                    )
                })?;

                TCommand::handler(self.state, converted_param)
            })();

            return ControlFlow::Break(result);
        }

        ControlFlow::Continue(self)
    }

    pub fn params(self) -> <ExecuteCommand as Request>::Params {
        self.params
    }
}
