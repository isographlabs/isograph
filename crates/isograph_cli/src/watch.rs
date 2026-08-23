use std::path::{Path, PathBuf};
use std::time::Duration;

use globset::Glob;
use isograph_compiler::{HostLanguage, SkipSourceFile};
use isograph_config::ISOGRAPH_FOLDER;
use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache, new_debouncer,
};
use prelude::Postfix;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info, warn};

use crate::Filesystem;
use crate::event::{DiskChanged, DiskFileChanged, FolderRemoved, IsographEvent, Presence};

const DEBOUNCE: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct WatchRoot {
    pub path: PathBuf,
    pub source: notify::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("could not watch {}: {}", .0.path.display(), .0.source)]
    Watch(WatchRoot),
    #[error("could not start the notify watcher: {0}")]
    Notify(notify::Error),
    #[error("invalid source_files glob {glob}: {source}")]
    Glob {
        glob: String,
        source: globset::Error,
    },
}

#[derive(Debug, Clone)]
pub enum SourceEventKind {
    CreateOrModify(PathBuf),
    Rename((PathBuf, PathBuf)),
    Remove(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangedFileKind {
    Config,
    SourceFile,
    SourceFolder,
}

pub type SourceFileEvent = (SourceEventKind, ChangedFileKind);

pub struct Watcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

pub fn start_if_watching<THostLanguage: HostLanguage>(
    filesystem: Filesystem,
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    source_files: &[String],
) -> Result<Option<Watcher>, WatchError> {
    match filesystem {
        Filesystem::Injected => None.wrap_ok(),
        Filesystem::Watch => start::<THostLanguage>(watch_tx, config_path, source_files)
            .map(|watcher| watcher.wrap_some()),
    }
}

#[expect(clippy::extra_unused_type_parameters)]
pub fn start<THostLanguage: HostLanguage>(
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    source_files: &[String],
) -> Result<Watcher, WatchError> {
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory")
        .to_owned();
    let globs = SourceGlobs::parse(source_files)?;
    let tx = watch_tx.clone();
    let globs_for_events = globs.clone();
    let config_path_for_events = config_path.to_owned();
    let mut debouncer =
        new_debouncer(
            DEBOUNCE,
            None,
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    if let Some(source_file_events) = categorize_and_filter_events(
                        events.as_slice(),
                        globs_for_events.reference(),
                        config_path_for_events.reference(),
                    ) {
                        let _ = tx.send(source_file_events);
                    }
                }
                Err(errors) => {
                    for e in errors {
                        warn!(error = %e, "notify");
                    }
                }
            },
        )
        .map_err(WatchError::Notify)?;
    debouncer
        .watch(config_directory.reference(), RecursiveMode::Recursive)
        .map_err(|source| {
            WatchError::Watch(WatchRoot {
                path: config_directory.clone(),
                source,
            })
        })?;
    let mut paths = Vec::new();
    visit_dirs_skipping_isograph(config_directory.reference(), &mut |entry| {
        paths.push(entry.path());
    });
    let boot: Vec<SourceFileEvent> = paths
        .into_iter()
        .filter_map(|path| {
            categorize_file(path.reference(), globs.reference(), config_path).and_then(|kind| {
                match kind {
                    ChangedFileKind::SourceFile => (
                        SourceEventKind::CreateOrModify(path),
                        ChangedFileKind::SourceFile,
                    )
                        .wrap_some(),
                    _ => None,
                }
            })
        })
        .collect();
    info!(n = boot.len(), "scan finished");
    if !boot.is_empty() {
        let _ = watch_tx.send(boot);
    }
    Watcher {
        _debouncer: debouncer,
    }
    .wrap_ok()
}

pub fn apply<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    events: Vec<SourceFileEvent>,
    source_files: &[String],
) {
    let globs = match SourceGlobs::parse(source_files) {
        Ok(globs) => globs,
        Err(e) => {
            tracing::error!(error = %e, "invalid source_files");
            return;
        }
    };
    for (kind, changed) in events {
        match changed {
            ChangedFileKind::Config => {}
            ChangedFileKind::SourceFile => match kind {
                SourceEventKind::CreateOrModify(path) => {
                    post_file::<THostLanguage>(
                        event_tx,
                        config_directory,
                        globs.reference(),
                        path.reference(),
                    );
                }
                SourceEventKind::Rename((from, to)) => {
                    post_file_absent(event_tx, from.reference());
                    post_file::<THostLanguage>(
                        event_tx,
                        config_directory,
                        globs.reference(),
                        to.reference(),
                    );
                }
                SourceEventKind::Remove(path) => {
                    post_file_absent(event_tx, path.reference());
                }
            },
            ChangedFileKind::SourceFolder => match kind {
                SourceEventKind::CreateOrModify(folder) => {
                    scan_folder::<THostLanguage>(
                        event_tx,
                        config_directory,
                        globs.reference(),
                        folder.reference(),
                    );
                }
                SourceEventKind::Rename((from, to)) => {
                    post_folder_removed(event_tx, from.reference());
                    scan_folder::<THostLanguage>(
                        event_tx,
                        config_directory,
                        globs.reference(),
                        to.reference(),
                    );
                }
                SourceEventKind::Remove(path) => {
                    post_folder_removed(event_tx, path.reference());
                }
            },
        }
    }
}

#[derive(Clone)]
struct SourceGlobs {
    patterns: Vec<GlobPattern>,
}

#[derive(Clone)]
struct GlobPattern {
    exclude: ExcludeGlob,
    matcher: globset::GlobMatcher,
}

#[derive(Clone, Copy)]
enum ExcludeGlob {
    Include,
    Exclude,
}

impl SourceGlobs {
    fn parse(source_files: &[String]) -> Result<Self, WatchError> {
        let patterns = source_files
            .iter()
            .map(|raw| {
                let (exclude, glob_str) = match raw.strip_prefix('!') {
                    Some(rest) => (ExcludeGlob::Exclude, rest),
                    None => (ExcludeGlob::Include, raw.as_str()),
                };
                let matcher = Glob::new(glob_str)
                    .map_err(|source| WatchError::Glob {
                        glob: raw.clone(),
                        source,
                    })?
                    .compile_matcher();
                GlobPattern { exclude, matcher }.wrap_ok()
            })
            .collect::<Result<Vec<_>, _>>()?;
        SourceGlobs { patterns }.wrap_ok()
    }

    fn contains(&self, relative: &Path) -> bool {
        self.patterns.iter().fold(false, |allowed, pattern| {
            if pattern.matcher.is_match(relative) {
                match pattern.exclude {
                    ExcludeGlob::Include => true,
                    ExcludeGlob::Exclude => false,
                }
            } else {
                allowed
            }
        })
    }
}

fn categorize_and_filter_events(
    events: &[DebouncedEvent],
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<Vec<SourceFileEvent>> {
    if events.iter().any(|event| event.need_rescan()) {
        let config_directory = config_path.parent()?;
        return (
            SourceEventKind::CreateOrModify(config_directory.to_owned()),
            ChangedFileKind::SourceFolder,
        )
            .wrap_vec()
            .wrap_some();
    }
    let source_file_events: Vec<_> = events
        .iter()
        .filter_map(|event| match event.kind {
            EventKind::Create(create_kind) => {
                process_create_event(globs, config_path, create_kind, event.paths.as_slice())
            }
            EventKind::Modify(modify_kind) => {
                process_modify_event(globs, config_path, modify_kind, event.paths.as_slice())
            }
            EventKind::Remove(remove_kind) => {
                process_remove_event(globs, config_path, remove_kind, event.paths.as_slice())
            }
            EventKind::Any => event.paths.first().and_then(|path| {
                if path.exists() {
                    process_create_or_modify(globs, config_path, path)
                } else {
                    process_remove_path(globs, config_path, path)
                }
            }),
            _ => None,
        })
        .collect();
    if source_file_events.is_empty() {
        None
    } else {
        source_file_events.wrap_some()
    }
}

fn process_create_event(
    globs: &SourceGlobs,
    config_path: &Path,
    create_kind: CreateKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match create_kind {
        CreateKind::File => paths
            .first()
            .and_then(|path| process_create_or_modify(globs, config_path, path)),
        CreateKind::Folder => paths.first().and_then(|path| {
            categorize_path(path, globs, config_path)
                .map(|kind| (SourceEventKind::CreateOrModify(path.clone()), kind))
        }),
        _ => None,
    }
}

fn process_modify_event(
    globs: &SourceGlobs,
    config_path: &Path,
    modify_kind: ModifyKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match modify_kind {
        ModifyKind::Data(_) => {
            let path = paths.first()?;
            if path.is_file() {
                process_create_or_modify(globs, config_path, path)
            } else {
                None
            }
        }
        ModifyKind::Any => {
            let path = paths.first()?;
            if path.exists() {
                process_create_or_modify(globs, config_path, path)
            } else {
                process_remove_path(globs, config_path, path)
            }
        }
        ModifyKind::Name(RenameMode::Any) => {
            let path = paths.first()?;
            if path.exists() {
                process_create_or_modify(globs, config_path, path)
            } else {
                process_remove_path(globs, config_path, path)
            }
        }
        ModifyKind::Name(RenameMode::Both) => {
            let from = paths.first()?;
            let to = paths.get(1)?;
            categorize_path(to, globs, config_path)
                .map(|kind| (SourceEventKind::Rename((from.clone(), to.clone())), kind))
        }
        ModifyKind::Name(RenameMode::From) => paths
            .first()
            .and_then(|path| process_remove_path(globs, config_path, path)),
        ModifyKind::Name(RenameMode::To) => paths
            .first()
            .and_then(|path| process_create_or_modify(globs, config_path, path)),
        _ => None,
    }
}

fn process_remove_event(
    globs: &SourceGlobs,
    config_path: &Path,
    remove_kind: RemoveKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    let path = paths.first()?;
    let kind = match remove_kind {
        RemoveKind::File => categorize_file(path, globs, config_path)?,
        RemoveKind::Folder => categorize_folder(path, globs, config_path)?,
        RemoveKind::Any => categorize_path(path, globs, config_path)?,
        RemoveKind::Other => return None,
    };
    (SourceEventKind::Remove(path.clone()), kind).wrap_some()
}

fn process_create_or_modify(
    globs: &SourceGlobs,
    config_path: &Path,
    path: &Path,
) -> Option<SourceFileEvent> {
    categorize_path(path, globs, config_path)
        .map(|kind| (SourceEventKind::CreateOrModify(path.to_owned()), kind))
}

fn process_remove_path(
    globs: &SourceGlobs,
    config_path: &Path,
    path: &Path,
) -> Option<SourceFileEvent> {
    categorize_path(path, globs, config_path)
        .map(|kind| (SourceEventKind::Remove(path.to_owned()), kind))
}

fn categorize_path(
    path: &Path,
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<ChangedFileKind> {
    if path.is_file() {
        categorize_file(path, globs, config_path)
    } else {
        categorize_folder(path, globs, config_path)
    }
}

fn categorize_file(
    path: &Path,
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<ChangedFileKind> {
    if path == config_path {
        return ChangedFileKind::Config.wrap_some();
    }
    let config_directory = config_path.parent()?;
    if is_skipped_isograph(path, config_directory) {
        return None;
    }
    let relative = pathdiff::diff_paths(path, config_directory)?;
    if globs.contains(relative.reference()) {
        ChangedFileKind::SourceFile.wrap_some()
    } else {
        None
    }
}

fn categorize_folder(
    path: &Path,
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<ChangedFileKind> {
    let _ = globs;
    if path == config_path {
        return ChangedFileKind::Config.wrap_some();
    }
    let config_directory = config_path.parent()?;
    if is_skipped_isograph(path, config_directory) {
        return None;
    }
    if path.starts_with(config_directory) {
        ChangedFileKind::SourceFolder.wrap_some()
    } else {
        None
    }
}

fn is_skipped_isograph(path: &Path, config_directory: &Path) -> bool {
    pathdiff::diff_paths(path, config_directory)
        .is_some_and(|relative| relative.starts_with(ISOGRAPH_FOLDER))
}

fn visit_dirs_skipping_isograph(dir: &Path, cb: &mut dyn FnMut(&std::fs::DirEntry)) {
    let config_directory = dir;
    visit_dirs_skipping_isograph_from(dir, config_directory, cb);
}

fn visit_dirs_skipping_isograph_from(
    dir: &Path,
    config_directory: &Path,
    cb: &mut dyn FnMut(&std::fs::DirEntry),
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(error = %e, path = %dir.display(), "walk");
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!(error = %e, path = %dir.display(), "walk");
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            if is_skipped_isograph(path.reference(), config_directory) {
                continue;
            }
            visit_dirs_skipping_isograph_from(path.reference(), config_directory, cb);
        } else {
            cb(&entry);
        }
    }
}

fn scan_folder<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    globs: &SourceGlobs,
    folder: &Path,
) {
    let mut paths = Vec::new();
    visit_dirs_skipping_isograph_from(folder, config_directory, &mut |entry| {
        paths.push(entry.path());
    });
    for path in paths {
        post_file::<THostLanguage>(event_tx, config_directory, globs, path.reference());
    }
}

fn post_file<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    globs: &SourceGlobs,
    path: &Path,
) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not canonicalize");
            post_file_absent(event_tx, path);
            return;
        }
    };
    let metadata = match std::fs::metadata(path.reference()) {
        Ok(metadata) => metadata,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not stat");
            post_file_absent(event_tx, path.reference());
            return;
        }
    };
    if !metadata.is_file() {
        return;
    }
    if path.to_str().is_none() {
        warn!(path = %path.display(), "skipping non-UTF8 path");
        return;
    }
    if is_skipped_isograph(path.reference(), config_directory) {
        return;
    }
    let relative = match pathdiff::diff_paths(path.reference(), config_directory) {
        Some(relative) => relative,
        None => return,
    };
    if !globs.contains(relative.reference()) {
        return;
    }
    match THostLanguage::should_skip_source_file(relative.reference()) {
        SkipSourceFile::Skip => return,
        SkipSourceFile::Keep => {}
    }
    let contents = match std::fs::read_to_string(path.reference()) {
        Ok(contents) => contents,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not read");
            post_file_absent(event_tx, path.reference());
            return;
        }
    };
    debug!(path = %path.display(), "disk present");
    post(
        event_tx,
        DiskChanged::File(DiskFileChanged {
            path,
            presence: Presence::Present(contents),
        }),
    );
}

fn gone_path(path: &Path) -> Option<PathBuf> {
    match path.canonicalize() {
        Ok(path) => path.wrap_some(),
        Err(_) => {
            if path.is_absolute() {
                path.to_owned().wrap_some()
            } else {
                None
            }
        }
    }
    .and_then(|path| {
        if path.to_str().is_none() {
            warn!(path = %path.display(), "skipping non-UTF8 path");
            None
        } else {
            path.wrap_some()
        }
    })
}

fn post_file_absent(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let Some(path) = gone_path(path) else {
        return;
    };
    post(
        event_tx,
        DiskChanged::File(DiskFileChanged {
            path,
            presence: Presence::Absent,
        }),
    );
}

fn post_folder_removed(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let Some(path) = gone_path(path) else {
        return;
    };
    post(event_tx, DiskChanged::FolderRemoved(FolderRemoved { path }));
}

fn post(event_tx: &UnboundedSender<IsographEvent>, change: DiskChanged) {
    let _ = event_tx.send(crate::event::Internal::DiskChanged(change).to());
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use intern::string_key::Intern;
    use isograph_compiler::{HostLanguage, IsoLiteralExtraction, IsographState, SkipSourceFile};
    use isograph_config::ISOGRAPH_FOLDER;
    use prelude::Postfix;
    use thiserror::Error;
    use tokio::sync::mpsc::unbounded_channel;

    use super::{
        ChangedFileKind, SourceEventKind, SourceGlobs, apply, categorize_and_filter_events,
        categorize_folder, start,
    };
    use crate::event::IsographEvent;
    use crate::state::{handle, intern_config_directory};

    #[derive(Clone, Debug, PartialEq, Eq, Error)]
    #[error("in host")]
    struct InHostError;

    struct InHost;

    impl HostLanguage for InHost {
        type Error = InHostError;
        type LiteralContext = ();

        fn extract_iso_literals(
            _db: &IsographState<Self>,
            _path: common_lang_types::RelativePathToSourceFile,
        ) -> &Option<Vec<IsoLiteralExtraction<Self>>> {
            const NONE: Option<Vec<IsoLiteralExtraction<InHost>>> = None;
            &NONE
        }

        fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile {
            match relative_path.extension().and_then(|e| e.to_str()) {
                Some("in") => SkipSourceFile::Keep,
                _ => SkipSourceFile::Skip,
            }
        }
    }

    fn interned(s: &str) -> common_lang_types::RelativePathToSourceFile {
        s.intern().to()
    }

    fn contents(state: &IsographState<InHost>, path: &str) -> Option<String> {
        state
            .disk_file(interned(path))
            .map(|file| file.contents.clone())
    }

    struct Harness {
        _dir: tempfile::TempDir,
        config_path: PathBuf,
        config_directory: PathBuf,
        state: IsographState<InHost>,
        event_tx: tokio::sync::mpsc::UnboundedSender<IsographEvent>,
        event_rx: tokio::sync::mpsc::UnboundedReceiver<IsographEvent>,
        source_files: Vec<String>,
    }

    impl Harness {
        fn new(source_files: &[&str]) -> Self {
            let dir = tempfile::tempdir().expect("a test can create a temp directory");
            let config_path = dir.path().join("isograph.config.json");
            std::fs::write(config_path.reference(), "{}\n").expect("a test can write a config");
            let config_path = config_path.canonicalize().expect("the config exists");
            let mut state = IsographState::<InHost>::default();
            intern_config_directory(&mut state, config_path.reference());
            let config_directory = config_path
                .parent()
                .expect("a config file path has a parent directory")
                .to_owned();
            let (event_tx, event_rx) = unbounded_channel();
            Self {
                _dir: dir,
                config_path,
                config_directory,
                state,
                event_tx,
                event_rx,
                source_files: source_files.iter().map(|s| (*s).to_owned()).collect(),
            }
        }

        fn abs(&self, relative: &str) -> PathBuf {
            self.config_directory.join(relative)
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.abs(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("a test can create directories");
            }
            std::fs::write(path.reference(), contents).expect("a test can write a file");
        }

        fn ingest(&mut self, events: Vec<super::SourceFileEvent>) {
            apply::<InHost>(
                self.event_tx.reference(),
                self.config_directory.reference(),
                events,
                self.source_files.as_slice(),
            );
            while let Ok(event) = self.event_rx.try_recv() {
                handle(&mut self.state, event);
            }
        }
    }

    #[test]
    fn globs_last_match_wins_and_empty_contains_nothing() {
        let ts = SourceGlobs::parse(&["src/**/*.ts".to_owned()]).expect("valid glob");
        assert!(ts.contains(Path::new("src/a.ts")));
        assert!(!ts.contains(Path::new("src/a.tsx")));
        assert!(!ts.contains(Path::new("lib/a.ts")));
        let with_exclude =
            SourceGlobs::parse(&["src/**/*.ts".to_owned(), "!src/**/*.test.ts".to_owned()])
                .expect("valid glob");
        assert!(with_exclude.contains(Path::new("src/a.ts")));
        assert!(!with_exclude.contains(Path::new("src/a.test.ts")));
        let empty = SourceGlobs::parse(&[]).expect("empty is valid");
        assert!(!empty.contains(Path::new("src/a.ts")));
        let inn = SourceGlobs::parse(&["src/**/*.in".to_owned()]).expect("valid glob");
        assert!(inn.contains(Path::new("src/a.in")));
        assert!(!inn.contains(Path::new("src")));
    }

    #[test]
    fn folder_src_under_the_config_directory_is_a_source_folder() {
        let globs = SourceGlobs::parse(&["src/**/*.in".to_owned()]).expect("valid glob");
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let config_path = dir.path().join("isograph.config.json");
        std::fs::write(config_path.reference(), "{}\n").expect("a test can write a config");
        let src = dir.path().join("src");
        std::fs::create_dir_all(src.reference()).expect("a test can create src");
        assert_eq!(
            categorize_folder(src.reference(), globs.reference(), config_path.reference()),
            ChangedFileKind::SourceFolder.wrap_some()
        );
    }

    #[test]
    fn create_of_a_matching_file_interns_it_and_a_non_matching_file_does_not() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        h.write("src/b.rs", "b");
        let a = h.abs("src/a.in");
        let b = h.abs("src/b.rs");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(a),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        h.ingest(
            (
                SourceEventKind::CreateOrModify(b),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        assert_eq!(
            contents(h.state.reference(), "src/a.in").as_deref(),
            "a".wrap_some()
        );
        assert!(contents(h.state.reference(), "src/b.rs").is_none());
    }

    #[test]
    fn create_of_the_config_path_is_not_a_disk_changed() {
        let mut h = Harness::new(&["**/*.in"]);
        h.ingest(
            (
                SourceEventKind::CreateOrModify(h.config_path.clone()),
                ChangedFileKind::Config,
            )
                .wrap_vec(),
        );
        assert!(h.state.get_disk_file_map().untracked().0.is_empty());
    }

    #[test]
    fn create_of_folder_src_interns_matching_files_inside() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        h.write("src/b.rs", "b");
        let src = h.abs("src");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(src),
                ChangedFileKind::SourceFolder,
            )
                .wrap_vec(),
        );
        assert_eq!(
            contents(h.state.reference(), "src/a.in").as_deref(),
            "a".wrap_some()
        );
        assert!(contents(h.state.reference(), "src/b.rs").is_none());
    }

    #[test]
    fn remove_of_a_file_is_file_absent() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        let a = h.abs("src/a.in");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(a.clone()),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        h.ingest((SourceEventKind::Remove(a), ChangedFileKind::SourceFile).wrap_vec());
        assert!(contents(h.state.reference(), "src/a.in").is_none());
    }

    #[test]
    fn remove_of_folder_src_is_folder_removed() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        let a = h.abs("src/a.in");
        let src = h.abs("src");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(a),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        h.ingest((SourceEventKind::Remove(src), ChangedFileKind::SourceFolder).wrap_vec());
        assert!(contents(h.state.reference(), "src/a.in").is_none());
    }

    #[test]
    fn rename_file_is_absent_of_from_then_present_of_to() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        h.write("src/b.in", "b");
        let from = h.abs("src/a.in");
        let to = h.abs("src/b.in");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(from.clone()),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        h.ingest(
            (
                SourceEventKind::Rename((from, to)),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        assert!(contents(h.state.reference(), "src/a.in").is_none());
        assert_eq!(
            contents(h.state.reference(), "src/b.in").as_deref(),
            "b".wrap_some()
        );
    }

    #[test]
    fn rename_folder_is_folder_removed_then_present_of_files_under_to() {
        let mut h = Harness::new(&["**/*.in"]);
        h.write("src/a.in", "a");
        h.write("dst/a.in", "a");
        let from = h.abs("src");
        let to = h.abs("dst");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(h.abs("src/a.in")),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        h.ingest(
            (
                SourceEventKind::Rename((from, to)),
                ChangedFileKind::SourceFolder,
            )
                .wrap_vec(),
        );
        assert!(contents(h.state.reference(), "src/a.in").is_none());
        assert_eq!(
            contents(h.state.reference(), "dst/a.in").as_deref(),
            "a".wrap_some()
        );
    }

    #[test]
    fn empty_source_files_boot_posts_nothing() {
        let h = Harness::new(&[]);
        h.write("src/a.in", "a");
        let (watch_tx, mut watch_rx) = unbounded_channel();
        let _watcher = start::<InHost>(
            watch_tx,
            h.config_path.reference(),
            h.source_files.as_slice(),
        )
        .expect("the watcher starts");
        assert!(watch_rx.try_recv().is_err());
        assert!(contents(h.state.reference(), "src/a.in").is_none());
    }

    #[test]
    fn root_isograph_is_out_of_scope_and_nested_isograph_is_not() {
        let mut h = Harness::new(&["**/*.in"]);
        h.write(&format!("{ISOGRAPH_FOLDER}/a.in"), "skip");
        h.write("src/__isograph/b.in", "keep");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(h.abs(&format!("{ISOGRAPH_FOLDER}/a.in"))),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        h.ingest(
            (
                SourceEventKind::CreateOrModify(h.abs("src/__isograph/b.in")),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        assert!(contents(h.state.reference(), &format!("{ISOGRAPH_FOLDER}/a.in")).is_none());
        assert_eq!(
            contents(h.state.reference(), "src/__isograph/b.in").as_deref(),
            "keep".wrap_some()
        );
    }

    #[test]
    fn failed_read_after_a_successful_present_posts_file_absent() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        let a = h.abs("src/a.in");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(a.clone()),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        assert_eq!(
            contents(h.state.reference(), "src/a.in").as_deref(),
            "a".wrap_some()
        );
        std::fs::remove_file(a.reference()).expect("the test deletes the file");
        h.ingest(
            (
                SourceEventKind::CreateOrModify(a),
                ChangedFileKind::SourceFile,
            )
                .wrap_vec(),
        );
        assert!(contents(h.state.reference(), "src/a.in").is_none());
    }

    #[test]
    fn need_rescan_rewalks_the_config_directory() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        let event =
            notify::Event::new(notify::EventKind::Other).set_flag(notify::event::Flag::Rescan);
        let debounced = notify_debouncer_full::DebouncedEvent::new(event, Instant::now());
        let globs = SourceGlobs::parse(h.source_files.as_slice()).expect("valid glob");
        let events = categorize_and_filter_events(
            &[debounced],
            globs.reference(),
            h.config_path.reference(),
        )
        .expect("rescan produces a walk");
        h.ingest(events);
        assert_eq!(
            contents(h.state.reference(), "src/a.in").as_deref(),
            "a".wrap_some()
        );
    }

    #[test]
    fn boot_interns_a_file_written_before_start() {
        let mut h = Harness::new(&["src/**/*.in"]);
        h.write("src/a.in", "a");
        let (watch_tx, mut watch_rx) = unbounded_channel();
        let _watcher = start::<InHost>(
            watch_tx,
            h.config_path.reference(),
            h.source_files.as_slice(),
        )
        .expect("the watcher starts");
        let events = watch_rx.try_recv().expect("boot posted matching files");
        h.ingest(events);
        assert_eq!(
            contents(h.state.reference(), "src/a.in").as_deref(),
            "a".wrap_some()
        );
    }

    #[test]
    fn notify_interns_a_file_written_after_start() {
        let mut h = Harness::new(&["src/**/*.in"]);
        std::fs::create_dir_all(h.abs("src")).expect("a test can create src");
        let (watch_tx, mut watch_rx) = unbounded_channel();
        let _watcher = start::<InHost>(
            watch_tx,
            h.config_path.reference(),
            h.source_files.as_slice(),
        )
        .expect("the watcher starts");
        let _ = watch_rx.try_recv();
        h.write("src/a.in", "a");
        let deadline = Instant::now() + Duration::from_secs(10);
        let events = loop {
            if let Ok(events) = watch_rx.try_recv() {
                break events;
            }
            assert!(Instant::now() < deadline, "notify did not deliver src/a.in");
            std::thread::sleep(Duration::from_millis(50));
        };
        h.ingest(events);
        assert_eq!(
            contents(h.state.reference(), "src/a.in").as_deref(),
            "a".wrap_some()
        );
    }
}
