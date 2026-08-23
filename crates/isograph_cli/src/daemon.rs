use std::ops::ControlFlow;
use std::path::PathBuf;

use isograph_compiler::HostLanguage;
use prelude::Postfix;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use crate::state::{IsographState, handle, intern_config_directory};

pub fn run<THostLanguage: HostLanguage>(config_path: PathBuf, port_path: PathBuf) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(error = %e, "could not start the tokio runtime");
            return;
        }
    };
    runtime.block_on(serve::<THostLanguage>(config_path, port_path));
}

async fn serve<THostLanguage: HostLanguage>(config_path: PathBuf, port_path: PathBuf) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let listener = match tokio::net::TcpListener::bind(std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        0,
    )))
    .await
    {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, "could not bind the lsp socket");
            return;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            tracing::error!(error = %e, "could not read the lsp socket address");
            return;
        }
    };
    if let Err(e) = std::fs::write(port_path.reference(), format!("{port}\n")) {
        tracing::error!(
            error = %e,
            path = %port_path.display(),
            "could not write the event socket port"
        );
        return;
    }
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");

    // `isograph stop` sends SIGTERM. Route it into the event channel as Quit, so the
    // model turns it into Kill, the effect loop breaks, and serve returns.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                if term.recv().await.is_some() {
                    tracing::info!("SIGTERM: quitting");
                    let _ = event_tx.send(IsographEvent::Quit);
                }
            });
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "no SIGTERM handler; a terminated isograph will not run Kill"
            );
        }
    }

    // `select!` rather than `join!`: the effect loop ends on `Kill`, and the event
    // loop never does, because `_hold_events` holds a sender for as long as serve runs.
    let _hold_events = event_tx.clone();
    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
    std::process::exit(0);
}

pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: IsographState<THostLanguage>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    while let Some(event) = event_rx.recv().await {
        let effects = handle(&mut state, event);
        for effect in effects {
            let _ = effect_tx.send(effect);
        }
    }
}

pub(crate) async fn run_effect_loop(mut effect_rx: UnboundedReceiver<IsographEffect>) {
    while let Some(effect) = effect_rx.recv().await {
        if perform(effect).is_break() {
            break;
        }
    }
}

pub fn perform(effect: IsographEffect) -> ControlFlow<()> {
    match effect {
        IsographEffect::LogHelloWorld => {
            tracing::info!("hello world");
            ControlFlow::Continue(())
        }
        IsographEffect::Kill => {
            tracing::info!("kill: exiting");
            ControlFlow::Break(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{run_effect_loop, run_event_loop};
    use crate::effect::IsographEffect;
    use crate::event::IsographEvent;
    use crate::state::IsographState;
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use tokio::sync::mpsc::unbounded_channel;

    #[tokio::test]
    async fn sending_hello_world_emits_log_hello_world() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(IsographEvent::HelloWorld)
            .expect("the test sends HelloWorld");
        drop(event_tx);
        run_event_loop(
            IsographState::<TypeScriptHostLanguage>::default(),
            event_rx,
            effect_tx,
        )
        .await;
        let effect = effect_rx.recv().await.expect("handle sent one effect");
        assert_eq!(effect, IsographEffect::LogHelloWorld);
    }

    #[tokio::test]
    async fn sending_quit_emits_kill() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(IsographEvent::Quit)
            .expect("the test sends Quit");
        drop(event_tx);
        run_event_loop(
            IsographState::<TypeScriptHostLanguage>::default(),
            event_rx,
            effect_tx,
        )
        .await;
        let effect = effect_rx.recv().await.expect("handle sent one effect");
        assert_eq!(effect, IsographEffect::Kill);
    }

    #[tokio::test]
    async fn kill_ends_the_effect_loop() {
        let (effect_tx, effect_rx) = unbounded_channel();
        effect_tx
            .send(IsographEffect::Kill)
            .expect("the test sends Kill");
        run_effect_loop(effect_rx).await;
        let _hold = effect_tx;
    }
}
