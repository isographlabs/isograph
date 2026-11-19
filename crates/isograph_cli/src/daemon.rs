use std::ops::ControlFlow;
use std::path::PathBuf;

use freddie_timer::TimerEffect;
use isograph_compiler::CompilationProfile;
use isograph_config::IsographProjectConfig;
use prelude::Postfix;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot::error::TryRecvError;

use crate::Filesystem;
use crate::effect::{IsographEffect, PublishDiagnosticsEffect};
use crate::event::IsographEvent;
use crate::state::{IsographState, handle, intern_config_directory};
use crate::timer::TimerPayload;
use crate::watch::{ProjectGlobs, SourceFileEvent};

pub fn run<TProfile: CompilationProfile>(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
    config: IsographProjectConfig,
) {
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
    runtime.block_on(serve::<TProfile>(
        config_path,
        port_path,
        filesystem,
        config,
    ));
}

async fn serve<TProfile: CompilationProfile>(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
    config: IsographProjectConfig,
) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let (watch_tx, watch_rx) = unbounded_channel::<Vec<SourceFileEvent>>();
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

    let globs = match ProjectGlobs::parse(
        config.source_files.as_slice(),
        config.schema.as_slice(),
        config.output_directory.as_deref(),
    ) {
        Ok(globs) => globs,
        Err(e) => {
            tracing::error!(error = %e, "invalid config globs");
            return;
        }
    };
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory")
        .to_owned();
    let artifact_folder = config.output_directory.as_ref().map(|output_directory| {
        crate::state::ArtifactFolder(
            config_directory
                .join(output_directory)
                .join(isograph_config::ISOGRAPH_FOLDER),
        )
    });
    let mut state = IsographState::<TProfile>::new(globs.clone(), artifact_folder.clone());
    intern_config_directory(&mut state.db, config_path.reference());
    let _hold_watch = watch_tx.clone();
    let started = match crate::watch::start_if_watching::<TProfile>(
        filesystem,
        watch_tx,
        config_path.reference(),
        globs.clone(),
    ) {
        Ok(started) => started,
        Err(e) => {
            tracing::error!(error = %e, "could not start the watcher");
            return;
        }
    };
    let (_watcher, boot) = match started {
        Some(crate::watch::StartedWatcher { watcher, boot }) => (watcher.wrap_some(), boot),
        None => (None, Vec::new()),
    };
    // accept_loop is first polled inside select!, so every session's events queue behind these.
    crate::watch::apply::<TProfile>(
        event_tx.reference(),
        config_directory.reference(),
        boot,
        globs.reference(),
    );
    if let Some(folder) = artifact_folder.as_ref() {
        let _ = event_tx.send(
            crate::event::InternalEvent::ArtifactFilesRead(
                crate::write_artifacts::read_artifact_files(folder),
            )
            .to(),
        );
    }

    // `isograph stop` without `--force` is SIGTERM on unix, App::ask_to_quit on Windows
    // (InternalEvent::Quit on the socket). Route SIGTERM into the event channel as Quit,
    // so the model turns it into Kill, the effect loop breaks, and serve returns.
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
                    let _ = event_tx.send(crate::event::InternalEvent::Quit.to());
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
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx, event_tx.clone()) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx.clone()) => {}
        () = run_filesystem_watcher::<TProfile>(
            watch_rx,
            event_tx,
            config_directory,
            globs,
        ) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
    std::process::exit(0);
}

async fn run_filesystem_watcher<TProfile: CompilationProfile>(
    mut watch_rx: UnboundedReceiver<Vec<SourceFileEvent>>,
    event_tx: UnboundedSender<IsographEvent>,
    config_directory: PathBuf,
    globs: ProjectGlobs,
) {
    while let Some(events) = watch_rx.recv().await {
        crate::watch::apply::<TProfile>(
            event_tx.reference(),
            config_directory.reference(),
            events,
            globs.reference(),
        );
    }
}

pub(crate) async fn run_event_loop<TProfile: CompilationProfile>(
    mut state: crate::state::IsographState<TProfile>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    while let Some(event) = event_rx.recv().await {
        tracing::debug!(?event, "event");
        let effects = handle(&mut state, event);
        for effect in effects {
            let _ = effect_tx.send(effect);
        }
    }
}

pub(crate) async fn run_effect_loop(
    mut effect_rx: UnboundedReceiver<IsographEffect>,
    event_tx: UnboundedSender<IsographEvent>,
) {
    let token = crate::effect::SideEffectToken::mint();
    while let Some(effect) = effect_rx.recv().await {
        tracing::debug!(?effect, "effect");
        if perform(token.reference(), effect, event_tx.reference()).is_break() {
            break;
        }
    }
}

pub fn perform(
    token: &crate::effect::SideEffectToken,
    effect: IsographEffect,
    event_tx: &UnboundedSender<IsographEvent>,
) -> ControlFlow<()> {
    match effect {
        IsographEffect::LogHelloWorld => {
            tracing::info!("hello world");
            ControlFlow::Continue(())
        }
        IsographEffect::Kill => {
            tracing::info!("kill: exiting");
            ControlFlow::Break(())
        }
        IsographEffect::SendLspResponse(respond) => {
            (respond.send)(token);
            ControlFlow::Continue(())
        }
        IsographEffect::PublishDiagnostics(publish) => {
            send_notifications(*publish);
            ControlFlow::Continue(())
        }
        IsographEffect::AssignClientId(assign) => {
            let _ = assign.assign.send(assign.client);
            ControlFlow::Continue(())
        }
        IsographEffect::WriteArtifacts(write) => {
            let folder = write.folder.clone();
            crate::write_artifacts::write_artifacts(*write).unwrap_or_else(|error| {
                tracing::error!(%error, "could not write the artifacts");
                let _ = event_tx.send(
                    crate::event::InternalEvent::ArtifactFilesRead(
                        crate::write_artifacts::read_artifact_files(folder.reference()),
                    )
                    .to(),
                );
            });
            ControlFlow::Continue(())
        }
        IsographEffect::Timer(timer) => {
            schedule_timer(timer, event_tx);
            ControlFlow::Continue(())
        }
    }
}

/// Sends each notification in order; a disconnected session drops the rest.
fn send_notifications(publish: PublishDiagnosticsEffect) {
    let PublishDiagnosticsEffect {
        sender,
        notifications,
    } = publish;
    notifications
        .into_iter()
        .map(lsp_server::Message::Notification)
        .try_for_each(|message| sender.send(message).map_err(drop))
        .unwrap_or_else(|()| tracing::debug!("publishDiagnostics to a closed session"));
}

/// Fire-and-forget on the runtime, so a pending sleep runs off the effect loop.
fn schedule_timer(timer: TimerEffect<TimerPayload>, event_tx: &UnboundedSender<IsographEvent>) {
    let TimerEffect {
        delay,
        event,
        mut cancel,
    } = timer;
    if matches!(cancel.try_recv(), Err(TryRecvError::Closed)) {
        return;
    }
    let event_tx = event_tx.clone();
    tokio::spawn(async move {
        tokio::select! {
            () = tokio::time::sleep(delay) => { let _ = event_tx.send(event.to()); }
            _ = cancel => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{perform, run_effect_loop, run_event_loop};
    use crate::effect::IsographEffect;
    use crate::event::{InternalEvent, IsographEvent};
    use crate::state::IsographState;
    use crate::test_profile::TestProfile;
    use crate::timer::TimerPayload;
    use prelude::Postfix;
    use tokio::sync::mpsc::unbounded_channel;

    fn empty_state() -> IsographState<TestProfile> {
        IsographState::new(
            crate::watch::ProjectGlobs::parse(&["src/**".to_owned()], &[], None)
                .expect("the test glob is valid"),
            None,
        )
    }

    #[tokio::test]
    async fn sending_hello_world_emits_log_hello_world() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(InternalEvent::HelloWorld.to())
            .expect("the test sends HelloWorld");
        drop(event_tx);
        run_event_loop(empty_state(), event_rx, effect_tx).await;
        let effect = effect_rx.recv().await.expect("handle sent one effect");
        assert!(matches!(effect, IsographEffect::LogHelloWorld));
    }

    #[tokio::test]
    async fn a_timer_effect_fires_its_event_after_a_zero_delay() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::ZERO,
            TimerPayload::DiagnosticsDebounce,
        );
        assert!(
            perform(
                &crate::effect::SideEffectToken::mint(),
                IsographEffect::Timer(timer),
                event_tx.reference(),
            )
            .is_continue()
        );
        let event = event_rx.recv().await.expect("the timer fired");
        let IsographEvent::Timer(fired) = event else {
            panic!("the timer posts TimerFired");
        };
        assert!(matches!(
            fired.trigger_if_matching(guard.reference().wrap_some()),
            Some(&TimerPayload::DiagnosticsDebounce)
        ));
    }

    #[tokio::test]
    async fn dropping_the_guard_before_perform_does_not_fire() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::ZERO,
            TimerPayload::DiagnosticsDebounce,
        );
        drop(guard);
        let _ = perform(
            &crate::effect::SideEffectToken::mint(),
            IsographEffect::Timer(timer),
            event_tx.reference(),
        );
        event_tx
            .send(InternalEvent::Quit.to())
            .expect("the test sends Quit");
        let event = event_rx.recv().await.expect("Quit arrives");
        assert!(matches!(
            event,
            IsographEvent::Internal(InternalEvent::Quit)
        ));
    }

    #[tokio::test]
    async fn dropping_the_guard_after_perform_does_not_fire() {
        tokio::time::pause();
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::from_secs(60),
            TimerPayload::DiagnosticsDebounce,
        );
        let _ = perform(
            &crate::effect::SideEffectToken::mint(),
            IsographEffect::Timer(timer),
            event_tx.reference(),
        );
        drop(guard);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(60)).await;
        event_tx
            .send(InternalEvent::Quit.to())
            .expect("the test sends Quit");
        let event = event_rx.recv().await.expect("Quit arrives");
        assert!(matches!(
            event,
            IsographEvent::Internal(InternalEvent::Quit)
        ));
    }

    #[tokio::test]
    async fn hello_world_through_event_and_effect_loops_returns_log_hello_world() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(InternalEvent::HelloWorld.to())
            .expect("the test sends HelloWorld");
        let token = crate::effect::SideEffectToken::mint();
        let event_loop = run_event_loop(empty_state(), event_rx, effect_tx);
        let effect_loop = async {
            loop {
                let effect = effect_rx.recv().await.expect("effects until LogHelloWorld");
                if matches!(effect, IsographEffect::LogHelloWorld) {
                    break;
                }
                assert!(perform(token.reference(), effect, event_tx.reference(),).is_continue());
            }
        };
        tokio::select! {
            () = event_loop => panic!("event loop ended before LogHelloWorld"),
            () = effect_loop => {}
        }
    }

    #[tokio::test]
    async fn sending_quit_emits_kill() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(InternalEvent::Quit.to())
            .expect("the test sends Quit");
        drop(event_tx);
        run_event_loop(empty_state(), event_rx, effect_tx).await;
        let effect = effect_rx.recv().await.expect("handle sent one effect");
        assert!(matches!(effect, IsographEffect::Kill));
    }

    #[tokio::test]
    async fn kill_ends_the_effect_loop() {
        let (effect_tx, effect_rx) = unbounded_channel();
        effect_tx
            .send(IsographEffect::Kill)
            .expect("the test sends Kill");
        let (event_tx, _event_rx) = unbounded_channel();
        run_effect_loop(effect_rx, event_tx).await;
        let _hold = effect_tx;
    }
}
