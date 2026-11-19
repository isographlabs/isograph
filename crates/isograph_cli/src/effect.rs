use freddie_timer::TimerEffect;

use crate::timer::TimerPayload;

pub struct SideEffectToken {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl SideEffectToken {
    pub(crate) fn mint() -> Self {
        Self {
            _not_send: std::marker::PhantomData,
        }
    }
}

pub struct SendLspResponseEffect {
    pub send: Box<dyn FnOnce(&SideEffectToken) + Send>,
}

impl std::fmt::Debug for SendLspResponseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SendLspResponseEffect")
    }
}

/// The `ClientId` `handle` minted for one connection, and the channel its session waits on.
#[derive(Debug)]
pub struct AssignClientIdEffect {
    pub assign: crossbeam::channel::Sender<isograph_compiler::ClientId>,
    pub client: isograph_compiler::ClientId,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    PublishDiagnostics(Box<PublishDiagnosticsEffect>),
    AssignClientId(AssignClientIdEffect),
    Timer(TimerEffect<TimerPayload>),
}

/// `textDocument/publishDiagnostics` notifications for one client.
#[derive(Debug)]
pub struct PublishDiagnosticsEffect {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub notifications: Vec<lsp_server::Notification>,
}

pub fn no_effects() -> Vec<IsographEffect> {
    Vec::new()
}
