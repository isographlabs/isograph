use crossbeam::channel::Sender;
use isograph_compiler::CompilerState;
use isograph_schema::NetworkProtocol;

pub struct LspState<'a, TNetworkProtocol: NetworkProtocol> {
    pub compiler_state: CompilerState<TNetworkProtocol>,
    pub sender: &'a Sender<lsp_server::Message>,
}

impl<'a, TNetworkProtocol: NetworkProtocol> LspState<'a, TNetworkProtocol> {
    pub fn new(
        compiler_state: CompilerState<TNetworkProtocol>,
        sender: &'a Sender<lsp_server::Message>,
    ) -> Self {
        LspState {
            compiler_state,
            sender,
        }
    }
}
