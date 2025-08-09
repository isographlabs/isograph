use isograph_compiler::CompilerState;
use lsp_types::request::{GotoDefinition, Request};

use crate::lsp_runtime_error::LSPRuntimeResult;

pub fn on_goto_definition(
    compiler_state: &CompilerState,
    params: <GotoDefinition as Request>::Params,
) -> LSPRuntimeResult<<GotoDefinition as Request>::Result> {
    todo!()
}
