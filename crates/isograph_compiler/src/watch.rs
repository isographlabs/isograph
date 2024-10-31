use colored::Colorize;
use isograph_config::CompilerConfig;
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    Error, EventKind, RecommendedWatcher, RecursiveMode,
};
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache,
};
use std::{path::PathBuf, time::Duration};
use tokio::{runtime::Handle, sync::mpsc::Receiver, task::JoinError};

use crate::{
    batch_compile::print_result, compiler_state::CompilerState, with_duration::WithDuration,
};

const MAX_CHANGED_FILES: usize = 100;

pub async fn handle_watch_command(
    config_location: PathBuf,
) -> Result<Result<(), Vec<Error>>, JoinError> {
    let mut state = CompilerState::new(config_location);
    let (mut rx, mut watcher) = create_debounced_file_watcher(&state.config);

    eprintln!("{}", "Starting to compile.".cyan());
    let _ = print_result(WithDuration::new(|| state.compile()));

    tokio::spawn(async move {
        while let Some(res) = rx.recv().await {
            match res {
                Ok(events) => {
                    if let Some(changes) = categorize_and_filter_events(&events, &state.config) {
                        let result = if has_config_changes(&changes) {
                            eprintln!(
                                "{}",
                                "Config change detected. Starting a full compilation.".cyan()
                            );
                            state = CompilerState::new(state.config.config_location);
                            watcher.stop();
                            (rx, watcher) = create_debounced_file_watcher(&state.config);
                            WithDuration::new(|| state.compile())
                        } else if changes.len() < MAX_CHANGED_FILES {
                            eprintln!("{}", "File changes detected. Starting to compile.".cyan());
                            WithDuration::new(|| state.update(&changes))
                        } else {
                            eprintln!(
                                "{}",
                                "Too many changes. Starting a full compilation.".cyan()
                            );
                            WithDuration::new(|| state.compile())
                        };
                        let _ = print_result(result);
                    }
                }
                Err(errors) => return Err(errors),
            }
        }
        Ok(())
    })
    .await
}

fn has_config_changes(changes: &[SourceFileEvent]) -> bool {
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
        Some(source_file_events)
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
                panic!("File create event should contain exactly one file. This is indicative of a bug in Isograph.")
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
                panic!("File modify event should contain exactly one file. This is indicative of a bug in Isograph.")
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
                        panic!("File rename event should contain exactly one file. This is indicative of a bug in Isograph.")
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
                        panic!("Rename event should contain exactly two paths. This is indicative of a bug in Isograph.")
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
                panic!("Remove event should contain exactly one path. This is indicative of a bug in Isograph.")
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
    if !path.starts_with(&config.artifact_directory) {
        if path.starts_with(&config.project_root) {
            if path.is_file() {
                return Some(ChangedFileKind::JavaScriptSourceFile);
            } else {
                return Some(ChangedFileKind::JavaScriptSourceFolder);
            }
        } else if path == &config.schema {
            return Some(ChangedFileKind::Schema);
        } else if config.schema_extensions.contains(path) {
            return Some(ChangedFileKind::SchemaExtension);
        } else if path == &config.config_location {
            return Some(ChangedFileKind::Config);
        }
    }
    None
}

#[allow(clippy::complexity)]
fn create_debounced_file_watcher(
    config: &CompilerConfig,
) -> (
    Receiver<Result<Vec<DebouncedEvent>, Vec<Error>>>,
    Debouncer<RecommendedWatcher, RecommendedCache>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let rt = Handle::current();

    let mut watcher = new_debouncer(
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
    )
    .expect("Expected to be able to create debouncer");

    watcher
        .watch(&config.config_location, RecursiveMode::NonRecursive)
        .expect("Failure when watching project root");
    watcher
        .watch(&config.project_root, RecursiveMode::Recursive)
        .expect("Failure when watching project root");
    watcher
        .watch(&config.schema, RecursiveMode::NonRecursive)
        .expect("Failing when watching schema");
    for extension in &config.schema_extensions {
        watcher
            .watch(extension, RecursiveMode::NonRecursive)
            .expect("Failing when watching schema extension");
    }

    (rx, watcher)
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
