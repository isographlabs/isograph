use lsp_server::Message;

#[derive(Debug)]
pub struct SendLspResponse {
    pub reply: crossbeam::channel::Sender<Message>,
    pub response: lsp_server::Response,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponse>),
}
