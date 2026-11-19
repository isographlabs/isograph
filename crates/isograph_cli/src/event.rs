use std::path::PathBuf;

use prelude::Postfix;

use crate::timer::TimerPayload;

#[derive(Clone, Debug)]
pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
}

#[derive(Clone, Debug)]
pub struct LspNotification {
    pub client: ClientId,
    pub notification: lsp_server::Notification,
}

#[derive(Clone, Debug)]
pub struct LspResponse {
    pub client: ClientId,
    pub response: lsp_server::Response,
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(LspNotification),
    Response(LspResponse),
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

pub struct BeforeCompute {
    client: ClientId,
    id: lsp_server::RequestId,
}

pub struct AfterCompute {
    client: ClientId,
    id: lsp_server::RequestId,
}

impl LiveSessions {
    pub fn reply(&mut self, client: ClientId, id: lsp_server::RequestId) -> Option<BeforeCompute> {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id.clone());
        BeforeCompute { client, id }.wrap_some()
    }

    pub fn before_compute(&mut self, before: BeforeCompute) -> Option<AfterCompute> {
        let session = self.0.get_mut(&before.client)?;
        if !session.outstanding.contains(&before.id) {
            return None;
        }
        AfterCompute {
            client: before.client,
            id: before.id,
        }
        .wrap_some()
    }

    #[expect(clippy::type_complexity)]
    pub fn after_compute(
        &mut self,
        after: AfterCompute,
        body: Result<serde_json::Value, lsp_server::ResponseError>,
    ) -> Option<Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>> {
        let session = self.0.get_mut(&after.client)?;
        if !session.outstanding.remove(&after.id) {
            return None;
        }
        let sender = session.sender.clone();
        let (result, error) = match body {
            Ok(value) => (value.wrap_some(), None),
            Err(error) => (None, error.wrap_some()),
        };
        let message = lsp_server::Message::Response(lsp_server::Response {
            id: after.id,
            result,
            error,
        });
        let send: Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send> =
            (move |_token: &crate::effect::SideEffectToken| {
                let _ = sender.send(message);
            })
            .boxed();
        send.wrap_some()
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
    #[from]
    Timer(freddie_timer::TimerFired<TimerPayload>),
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
