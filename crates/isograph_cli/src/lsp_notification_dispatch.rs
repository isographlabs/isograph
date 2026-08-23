use std::ops::ControlFlow;

use lsp_types::notification::Notification;
use tracing::warn;

use crate::effect::IsographEffect;

pub struct LSPNotificationDispatch<'state, TState> {
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LSPNotificationDispatch<'state, TState> {
    pub fn new(notification: lsp_server::Notification, state: &'state mut TState) -> Self {
        Self {
            notification,
            state,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params) -> Vec<IsographEffect>,
    ) -> ControlFlow<Vec<IsographEffect>, Self> {
        if self.notification.method != TNotification::METHOD {
            return ControlFlow::Continue(self);
        }
        match self
            .notification
            .extract::<TNotification::Params>(TNotification::METHOD)
        {
            Ok(params) => ControlFlow::Break(handler(self.state, params)),
            Err(lsp_server::ExtractError::MethodMismatch(notification)) => {
                ControlFlow::Continue(Self {
                    notification,
                    state: self.state,
                })
            }
            Err(lsp_server::ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "notification params");
                ControlFlow::Break(Vec::new())
            }
        }
    }

    pub fn notification(self) -> lsp_server::Notification {
        self.notification
    }
}
