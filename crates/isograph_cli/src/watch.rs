use colored::Colorize;
use notify::{Error, FsEventWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use std::time::Duration;
use tokio::{runtime::Handle, sync::mpsc::Receiver, task::JoinError};

use crate::{batch_compile::compile_and_print, config::CompilerConfig};

pub(crate) async fn handle_watch_command(
    config: CompilerConfig,
) -> Result<Result<(), Vec<Error>>, JoinError> {
    let _ = compile_and_print(&config);

    let (mut rx, mut watcher) = create_debounced_file_watcher();

    // We need to watch a few things: the schema, extensions, and project root
    watcher
        .watcher()
        .watch(&config.project_root, RecursiveMode::Recursive)
        .expect("Failure when watching project root");
    watcher
        .watcher()
        .watch(&config.schema, RecursiveMode::Recursive)
        .expect("Failing when watching schema");
    for extension in &config.schema_extensions {
        watcher
            .watcher()
            .watch(&extension, RecursiveMode::Recursive)
            .expect("Failing when watching schema extension");
    }

    tokio::spawn(async move {
        while let Some(res) = rx.recv().await {
            match res {
                Ok(_events) => {
                    eprintln!("{}", "File changes detected.".cyan());
                    let _ = compile_and_print(&config);
                }
                Err(errors) => return Err(errors),
            }
        }
        Ok(())
    })
    .await
}

fn create_debounced_file_watcher() -> (
    Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>,
    Debouncer<FsEventWatcher, FileIdMap>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let rt = Handle::current();

    let debounced_watcher = new_debouncer(
        // TODO control this with config
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| {
            let tx = tx.clone();

            rt.spawn(async move {
                if let Err(e) = tx.send(result).await {
                    println!("Error sending event result: {:?}", e);
                }
            });
        },
    );

    (
        rx,
        debounced_watcher.expect("Expected to be able to create debouncer"),
    )
}
