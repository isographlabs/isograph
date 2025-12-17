use colored::Colorize;
use common_lang_types::{CurrentWorkingDirectory, Diagnostic, DiagnosticVecResult};
use isograph_config::{CompilerConfig, create_config};
use isograph_schema::NetworkProtocol;
use notify::{
    Error, EventKind, RecommendedWatcher, RecursiveMode,
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache, new_debouncer,
};
use prelude::Postfix;
use std::{path::PathBuf, time::Duration};
use tokio::{runtime::Handle, sync::mpsc::Receiver};
use tracing::{info, warn};

use crate::{
    batch_compile::{compile, print_result},
    compiler_state::CompilerState,
    source_files::update_sources,
    with_duration::WithDuration,
};

pub async fn handle_watch_command<TNetworkProtocol: NetworkProtocol>(
    config: CompilerConfig,
    current_working_directory: CurrentWorkingDirectory,
) -> DiagnosticVecResult<()> {
    let mut state = CompilerState::new(config, current_working_directory)?;

    let config = state.db.get_isograph_config().clone();

    info!("{}", "Starting to compile.".cyan());
    let _ = print_result(WithDuration::new(|| {
        compile::<TNetworkProtocol>(&mut state)
    }));

    let (mut file_system_receiver, mut file_system_watcher) =
        create_debounced_file_watcher(&config);
    while let Some(res) = file_system_receiver.recv().await {
        match res {
            Ok(changes) => {
                if has_config_changes(&changes) {
                    info!(
                        "{}",
                        "Config change detected. Starting a full compilation.".cyan()
                    );
                    let config = create_config(&config.config_location, current_working_directory);
                    state = CompilerState::new(config.clone(), current_working_directory)?;
                    file_system_watcher.stop();
                    // TODO is this a bug? Will we continue to watch the old folders? I think so.
                    (file_system_receiver, file_system_watcher) =
                        create_debounced_file_watcher(&config);
                } else {
                    info!("{}", "File changes detected. Starting to compile.".cyan());
                    update_sources(&mut state.db, &changes)?;
                };
                let result = WithDuration::new(|| compile::<TNetworkProtocol>(&mut state));
                let _ = print_result(result);
                state.run_garbage_collection();
            }
            Err(errors) => {
                return errors
                    .into_iter()
                    .map(|e| Diagnostic::from_error(e, None))
                    .collect::<Vec<_>>()
                    .wrap_err();
            }
        }
    }
    Ok(())
}

pub fn has_config_changes(changes: &[SourceFileEvent]) -> bool {
    changes
        .iter()
        .any(|(_, changed_file_kind)| matches!(changed_file_kind, ChangedFileKind::Config))
}

fn categorize_and_filter_events(
    events: &[DebouncedEvent],
    config: &CompilerConfig,
) -> Option<Vec<SourceFileEvent>> {
    let mut source_file_events = vec![];
    for event in events {
        if let Some(source_file_event) = match event.kind {
            EventKind::Create(create_kind) => {
                process_create_event(config, create_kind, &event.paths)
            }
            EventKind::Modify(modify_kind) => {
                process_modify_event(config, modify_kind, &event.paths)
            }
            EventKind::Remove(remove_kind) => {
                process_remove_event(config, remove_kind, &event.paths)
            }
            _ => None,
        } {
            source_file_events.push(source_file_event);
        }
    }
    if source_file_events.is_empty() {
        None
    } else {
        source_file_events.wrap_some()
    }
}

fn process_create_event(
    config: &CompilerConfig,
    create_kind: CreateKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match create_kind {
        // Note: maybe we should add CreateKind::Folder as well. Need a confirmation
        // that move folder from outside a watch directory could fire a create event.
        // Now it's always Modify(Name(Any)) i.e. Rename
        CreateKind::File => {
            if paths.len() != 1 {
                panic!(
                    "File create event should contain exactly one file. \
                    This is indicative of a bug in Isograph."
                )
            }
            categorize_changed_file_and_filter_changes_in_artifact_directory(config, &paths[0])
                .map(|file_kind| (SourceEventKind::CreateOrModify(paths[0].clone()), file_kind))
        }
        // TODO: handle symlinks
        CreateKind::Other => None,
        _ => None,
    }
}

fn process_modify_event(
    config: &CompilerConfig,
    modify_kind: ModifyKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match modify_kind {
        ModifyKind::Data(_) => {
            if paths.len() != 1 {
                panic!(
                    "File modify event should contain exactly one file. \
                    This is indicative of a bug in Isograph."
                )
            }
            if paths[0].is_file() {
                categorize_changed_file_and_filter_changes_in_artifact_directory(config, &paths[0])
                    .map(|file_kind| (SourceEventKind::CreateOrModify(paths[0].clone()), file_kind))
            } else {
                None
            }
        }
        ModifyKind::Name(rename_mode) => {
            match rename_mode {
                // This event could be fired once on delete or twice on rename
                RenameMode::Any => {
                    if paths.len() != 1 {
                        panic!(
                            "File rename event should contain exactly one file. \
                            This is indicative of a bug in Isograph."
                        )
                    }
                    categorize_changed_file_and_filter_changes_in_artifact_directory(
                        config, &paths[0],
                    )
                    .map(|file_kind| {
                        if paths[0].exists() {
                            (SourceEventKind::CreateOrModify(paths[0].clone()), file_kind)
                        } else {
                            (SourceEventKind::Remove(paths[0].clone()), file_kind)
                        }
                    })
                }
                RenameMode::Both => {
                    if paths.len() != 2 {
                        panic!(
                            "Rename event should contain exactly two paths. \
                            This is indicative of a bug in Isograph."
                        )
                    }
                    categorize_changed_file_and_filter_changes_in_artifact_directory(
                        config, &paths[1],
                    )
                    .map(|file_kind| {
                        (
                            SourceEventKind::Rename((paths[0].clone(), paths[1].clone())),
                            file_kind,
                        )
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn process_remove_event(
    config: &CompilerConfig,
    remove_kind: RemoveKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match remove_kind {
        RemoveKind::File | RemoveKind::Folder | RemoveKind::Any => {
            if paths.len() != 1 {
                panic!(
                    "Remove event should contain exactly one path. \
                    This is indicative of a bug in Isograph."
                )
            }
            categorize_changed_file_and_filter_changes_in_artifact_directory(config, &paths[0])
                .map(|file_kind| (SourceEventKind::Remove(paths[0].clone()), file_kind))
        }
        // TODO: handle symlinks
        RemoveKind::Other => None,
    }
}

fn categorize_changed_file_and_filter_changes_in_artifact_directory(
    config: &CompilerConfig,
    path: &PathBuf,
) -> Option<ChangedFileKind> {
    if !path.starts_with(&config.artifact_directory.absolute_path) {
        if path.starts_with(&config.project_root) {
            if path.is_file() {
                return ChangedFileKind::JavaScriptSourceFile.wrap_some();
            } else {
                return ChangedFileKind::JavaScriptSourceFolder.wrap_some();
            }
        } else if path == &config.schema.absolute_path {
            return ChangedFileKind::Schema.wrap_some();
        } else if config
            .schema_extensions
            .iter()
            .any(|x| x.absolute_path == *path)
        {
            return ChangedFileKind::SchemaExtension.wrap_some();
        } else if path == &config.config_location {
            return ChangedFileKind::Config.wrap_some();
        }
    }
    None
}

// TODO reimplement this as create_debounced_file_watcher.map(...)
#[expect(clippy::complexity)]
pub fn create_debounced_file_watcher(
    config: &CompilerConfig,
) -> (
    Receiver<Result<Vec<(SourceEventKind, ChangedFileKind)>, Vec<Error>>>,
    Debouncer<RecommendedWatcher, RecommendedCache>,
) {
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    let current_runtime = Handle::current();
    let config_for_watcher = config.clone();

    let mut watcher = new_debouncer(
        // TODO control this with config
        Duration::from_millis(100),
        None,
        move |result: DebounceEventResult| {
            let events = result
                .map(|events| categorize_and_filter_events(&events, &config_for_watcher))
                .transpose();

            if let Some(events) = events {
                let sender = sender.clone();
                current_runtime.spawn(async move {
                    if let Err(e) = sender.send(events).await {
                        warn!("Error sending event result: {e:?}");
                    }
                });
            }
        },
    )
    .expect("Expected to be able to create debouncer");

    watcher
        .watch(&config.config_location, RecursiveMode::NonRecursive)
        .expect("Failure when watching project root");
    watcher
        .watch(&config.project_root, RecursiveMode::Recursive)
        .expect("Failure when watching project root");
    watcher
        .watch(&config.schema.absolute_path, RecursiveMode::NonRecursive)
        .expect("Failing when watching schema");
    for extension in &config.schema_extensions {
        watcher
            .watch(&extension.absolute_path, RecursiveMode::NonRecursive)
            .expect("Failing when watching schema extension");
    }

    (receiver, watcher)
}

#[derive(Debug, Clone)]
pub enum SourceEventKind {
    CreateOrModify(PathBuf),
    Rename((PathBuf, PathBuf)),
    Remove(PathBuf),
}

pub enum ChangedFileKind {
    Config,
    Schema,
    SchemaExtension,
    JavaScriptSourceFile,
    JavaScriptSourceFolder,
}

pub type SourceFileEvent = (SourceEventKind, ChangedFileKind);
