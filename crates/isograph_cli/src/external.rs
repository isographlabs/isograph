use tracing::warn;

use crate::event::IsographEvent;

pub(crate) fn on_message(text: &str, emit: impl FnOnce(IsographEvent)) {
    match serde_json::from_str::<IsographEvent>(text) {
        Ok(event) => emit(event),
        Err(e) => warn!(error = %e, frame = text, "undeserializable frame"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::SinkExt;
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
    use tokio_tungstenite::tungstenite::Message;

    use super::on_message;
    use crate::event::IsographEvent;

    const SETTLE: Duration = Duration::from_millis(250);

    fn listen_for_events() -> (
        freddie_event_socket::EventSocket,
        u16,
        UnboundedReceiver<IsographEvent>,
    ) {
        let (event_tx, event_rx) = unbounded_channel();
        let socket = freddie_event_socket::listen(0, move |text| {
            on_message(text, |event| {
                let _ = event_tx.send(event);
            });
        })
        .expect("binding port 0");
        let port = socket.local_addr().port();
        (socket, port, event_rx)
    }

    #[tokio::test]
    async fn a_hello_world_frame_arrives_as_an_event() {
        let (_socket, port, mut event_rx) = listen_for_events();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connecting");
        ws.send(Message::Text(r#"{"kind":"HelloWorld"}"#.to_owned()))
            .await
            .expect("sending");
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("an event arrived"),
            IsographEvent::HelloWorld
        ));
    }

    #[tokio::test]
    async fn an_unknown_frame_is_dropped_without_disturbing_the_connection() {
        let (_socket, port, mut event_rx) = listen_for_events();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connecting");
        for frame in [r#"{"kind":"Nope"}"#, "not json at all"] {
            ws.send(Message::Text(frame.to_owned()))
                .await
                .expect("sending");
        }
        tokio::time::sleep(SETTLE).await;
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
        ws.send(Message::Text(r#"{"kind":"HelloWorld"}"#.to_owned()))
            .await
            .expect("the connection survived two bad frames");
        tokio::time::sleep(SETTLE).await;
        assert!(
            event_rx.try_recv().is_ok(),
            "the next good frame still arrived"
        );
    }
}
