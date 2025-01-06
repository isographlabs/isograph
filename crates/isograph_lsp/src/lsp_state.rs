use std::collections::HashMap;

use crossbeam::channel::Sender;
use isograph_config::CompilerConfig;
use lsp_server::Message;
use lsp_types::Url;

use crate::lsp_runtime_error::LSPRuntimeResult;

#[derive(Debug)]
pub struct LSPState {
    open_docs: HashMap<Url, String>,
    sender: Sender<Message>,
    pub config: CompilerConfig,
}

impl LSPState {
    pub fn new(sender: Sender<Message>, config: CompilerConfig) -> Self {
        LSPState {
            open_docs: HashMap::new(),
            sender,
            config,
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

    pub fn text_for(&self, uri: &Url) -> Option<&str> {
        self.open_docs.get(uri).map(|s| s.as_str())
    }

    pub fn send_message(&self, message: Message) {
        self.sender.send(message).unwrap();
    }
}
