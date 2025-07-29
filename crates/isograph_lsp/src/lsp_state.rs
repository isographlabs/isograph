use std::collections::HashMap;

use common_lang_types::CurrentWorkingDirectory;
use crossbeam::channel::Sender;
use isograph_config::CompilerConfig;
use lsp_server::Message;
use lsp_types::Url;
use pico::Database;

use crate::lsp_runtime_error::LSPRuntimeResult;

#[derive(Debug)]
pub struct LSPState {
    pub db: Database,
    open_docs: HashMap<Url, String>,
    sender: Sender<Message>,
}

impl LSPState {
    pub fn new(
        sender: Sender<Message>,
        config: CompilerConfig,
        current_working_directory: CurrentWorkingDirectory,
    ) -> Self {
        let mut db = Database::new();

        db.set(current_working_directory);
        db.set(config);

        LSPState {
            db,
            open_docs: HashMap::new(),
            sender,
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
