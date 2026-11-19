use std::ops::ControlFlow;

use isograph_compiler::ClientId;
use lsp_types::notification::Notification;
use tracing::warn;

use crate::effect::{IsographEffect, no_effects};
use crate::event::LspNotification;

pub struct LSPNotificationDispatch<'state, TState> {
    client: ClientId,
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LSPNotificationDispatch<'state, TState> {
    pub fn new(incoming: LspNotification, state: &'state mut TState) -> Self {
        Self {
            client: incoming.client,
            notification: incoming.notification,
            state,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, ClientId, TNotification::Params) -> Vec<IsographEffect>,
    ) -> ControlFlow<Vec<IsographEffect>, Self> {
        if self.notification.method != TNotification::METHOD {
            return ControlFlow::Continue(self);
        }
        let lsp_server::Notification { method, params } = self.notification;
        ControlFlow::Break(
            serde_json::from_value::<TNotification::Params>(params).map_or_else(
                |error| {
                    warn!(method = method.as_str(), error = %error, "notification params");
                    no_effects()
                },
                |params| handler(self.state, self.client, params),
            ),
        )
    }

    pub fn notification(self) -> lsp_server::Notification {
        self.notification
    }
}
