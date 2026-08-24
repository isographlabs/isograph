use std::path::PathBuf;

use prelude::Postfix;

#[derive(Clone, Debug)]
pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum InternalEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(pub u64);

#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    #[cfg_attr(not(test), expect(dead_code))]
    pub capabilities: lsp_types::ClientCapabilities,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}

#[derive(Debug, Default)]
pub struct LiveSessions(pub std::collections::HashMap<ClientId, LiveSession>);

impl LiveSessions {
    pub fn take_reply(
        &mut self,
        client: ClientId,
        id: lsp_server::RequestId,
    ) -> Option<
        impl for<'a> FnOnce(&'a crate::effect::SideEffectToken, lsp_server::Message) + Send + use<>,
    > {
        let session = self.0.get_mut(&client)?;
        if !session.outstanding.remove(&id) {
            return None;
        }
        let sender = session.sender.clone();
        (move |_token: &crate::effect::SideEffectToken, message| {
            let _ = sender.send(message);
        })
        .wrap_some()
    }
}

#[derive(Clone, Debug)]
pub struct ClientStartedEvent {
    pub client: ClientId,
    pub session: LiveSession,
}

#[derive(Clone, Debug)]
#[expect(clippy::large_enum_variant)]
pub enum ClientConnectionEvent {
    Started(ClientStartedEvent),
    Gone(ClientId),
}

#[derive(Clone, Debug, derive_more::From)]
#[expect(clippy::large_enum_variant)]
pub enum IsographEvent {
    #[from]
    Lsp(LspEvent),
    #[from]
    Internal(InternalEvent),
    #[from]
    ClientConnection(ClientConnectionEvent),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum DiskChanged {
    File(DiskFileChanged),
    FolderRemoved(FolderRemoved),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct DiskFileChanged {
    pub path: PathBuf,
    pub presence: Presence,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct FolderRemoved {
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Presence {
    Present(String),
    Absent,
}
