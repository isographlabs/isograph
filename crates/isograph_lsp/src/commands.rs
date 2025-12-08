use std::{ops::ControlFlow, str::FromStr};

use isograph_schema::NetworkProtocol;
use lsp_server::{Message, RequestId};
use lsp_types::{
    Command, ShowDocumentParams, Uri,
    request::{ExecuteCommand, Request, ShowDocument},
};
use prelude::Postfix;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    lsp_command_dispatch::LspIsographCommandDispatch,
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    lsp_state::LspState,
};

pub fn all_commands() -> Vec<String> {
    vec![OpenFileIsographLspCommand::METHOD.to_string()]
}

pub(crate) trait IsographLspCommand {
    const TITLE: &'static str;
    const METHOD: &'static str;
    type Params: Serialize + DeserializeOwned;
    fn command(params: Self::Params) -> Command {
        Command {
            title: Self::TITLE.to_string(),
            command: Self::METHOD.to_string(),
            arguments: serde_json::to_value(params)
                .expect("Expected params to be serializable")
                .wrap_vec()
                .wrap_some(),
        }
    }
    fn handler<TNetworkProtocol: NetworkProtocol>(
        state: &LspState<TNetworkProtocol>,
        params: Self::Params,
    ) -> LSPRuntimeResult<Option<Value>>;
}

pub(crate) struct OpenFileIsographLspCommand {}

impl IsographLspCommand for OpenFileIsographLspCommand {
    const METHOD: &'static str = "iso_open_file";
    const TITLE: &'static str = "Isograph: Open file";
    // the uri of the file
    type Params = String;

    fn handler<TNetworkProtocol: NetworkProtocol>(
        state: &LspState<TNetworkProtocol>,
        params: <OpenFileIsographLspCommand as IsographLspCommand>::Params,
    ) -> LSPRuntimeResult<Option<Value>> {
        let params = ShowDocumentParams {
            uri: Uri::from_str(&params).map_err(|_| {
                LSPRuntimeError::UnexpectedError(format!(
                    "Unable to convert to uri: `{params}`. \
                        This is indicative of a bug in Isograph."
                ))
            })?,
            external: None,
            take_focus: true.wrap_some(),
            selection: None,
        };

        let request = lsp_server::Request {
            id: RequestId::from("lsp-default-id".to_string()),
            method: ShowDocument::METHOD.to_string(),
            params: serde_json::to_value(params).map_err(|_| {
                LSPRuntimeError::UnexpectedError(
                    "Unable to serialize. This is indicative of a bug in Isograph.".to_string(),
                )
            })?,
        };

        state.sender.send(Message::Request(request)).map_err(|_| {
            LSPRuntimeError::UnexpectedError("Unable to send message to server.".to_string())
        })?;

        None.wrap_ok()
    }
}

pub fn on_command<TNetworkProtocol: NetworkProtocol>(
    lsp_state: &LspState<TNetworkProtocol>,
    params: <ExecuteCommand as Request>::Params,
) -> LSPRuntimeResult<<ExecuteCommand as Request>::Result> {
    let get_response = || {
        let retrieved_params = LspIsographCommandDispatch::new(params, lsp_state)
            .on_command_sync::<OpenFileIsographLspCommand>()?
            .params();

        ControlFlow::Continue(retrieved_params)
    };

    match get_response() {
        ControlFlow::Break(result) => result,
        ControlFlow::Continue(params) => LSPRuntimeError::UnexpectedError(format!(
            "No command handler registered for method '{}'",
            params.command
        ))
        .wrap_err(),
    }
}
