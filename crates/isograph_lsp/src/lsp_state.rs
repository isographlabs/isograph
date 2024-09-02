use std::collections::HashMap;

use lsp_types::Url;

use crate::lsp_runtime_error::LSPRuntimeResult;

#[derive(Debug)]
pub struct LSPState {
    open_docs: HashMap<Url, String>,
}

impl Default for LSPState {
    fn default() -> Self {
        Self::new()
    }
}

impl LSPState {
    pub fn new() -> Self {
        LSPState {
            open_docs: HashMap::new(),
        }
    }

    pub fn document_opened(&mut self, uri: &Url, text: &str) -> LSPRuntimeResult<()> {
        self.open_docs.insert(uri.to_owned(), text.to_owned());
        Ok(())
    }

    pub fn document_changed(&mut self, uri: &Url, text: &str) -> LSPRuntimeResult<()> {
        self.open_docs.insert(uri.to_owned(), text.to_owned());
        Ok(())
    }

    pub fn document_closed(&mut self, uri: &Url) -> LSPRuntimeResult<()> {
        self.open_docs.remove(uri);
        Ok(())
    }
}
