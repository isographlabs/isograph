use lsp_server::Message;

#[derive(Debug)]
pub struct SendLspResponseEffect {
    pub reply: crossbeam::channel::Sender<Message>,
    pub response: lsp_server::Response,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponseEffect(Box<SendLspResponseEffect>),
}
