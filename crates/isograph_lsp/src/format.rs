use isograph_compiler::CompilerState;
use lsp_types::{
    request::{Formatting, Request},
    Position, Range, TextEdit,
};

use crate::lsp_runtime_error::LSPRuntimeResult;

pub fn on_format(
    _compiler_state: &CompilerState,
    _params: <Formatting as Request>::Params,
) -> LSPRuntimeResult<<Formatting as Request>::Result> {
    Ok(Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        new_text: "".to_string(),
    }]))
}
