use std::path::{Path, PathBuf};
use std::time::Duration;

use common_lang_types::RelativePathToSourceFile;
use intern::Lookup;
use isograph_compiler::{
    ClientId, CompilationProfile, IncludeFile, IsographDb, compiler_errors_in_file,
};
use prelude::Postfix;

use crate::adapter::MissingWorkingDirectory;
use crate::effect::{IsographEffect, no_effects};
use crate::state::IsographState;
use crate::timer::TimerPayload;
use crate::watch::ProjectGlobs;

pub(crate) const COMPILE_DEBOUNCE: Duration = Duration::from_millis(100);

/// One compiler error, positioned for a terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerErrorReport {
    /// Absolute: the config directory joined with the source path.
    pub path: PathBuf,
    /// LSP positions: 0-based lines, UTF-16 code unit columns.
    pub range: lsp_types::Range,
    pub message: String,
}

/// What compile reads.
pub(crate) struct CompileInput<'a, TProfile: CompilationProfile> {
    pub db: &'a IsographDb<TProfile>,
    pub globs: &'a ProjectGlobs,
    pub client: ClientId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ProjectErrorsError {
    #[error("{0}")]
    MissingWorkingDirectory(MissingWorkingDirectory),
    #[error("{} is in disk_file_map but has no text", .0.lookup())]
    ListedPathWithoutText(RelativePathToSourceFile),
}

/// Replaces the pending compile timer with one that fires after `COMPILE_DEBOUNCE`.
pub(crate) fn arm_compile_debounce<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let (guard, effect) =
        freddie_timer::timer_effect_and_guard(COMPILE_DEBOUNCE, TimerPayload::CompileDebounce);
    state.compile_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

/// Computes every source file's errors, so later reads are memo hits, and logs the count.
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &IsographState<TProfile>,
) -> Vec<IsographEffect> {
    project_errors(state.compile_input().reference()).map_or_else(
        |error| tracing::error!(%error, "compile"),
        |reports| tracing::info!(errors = reports.len(), "compiled"),
    );
    no_effects()
}

/// Every error of every source file on disk as `input.client` reads it, sorted by path,
/// then by start position.
pub(crate) fn project_errors<TProfile: CompilationProfile>(
    input: &CompileInput<'_, TProfile>,
) -> Result<Vec<CompilerErrorReport>, ProjectErrorsError> {
    let per_file = input
        .db
        .disk_paths()
        .into_iter()
        .filter(|path| {
            matches!(
                input
                    .globs
                    .should_include_source_file::<TProfile>(Path::new(path.lookup())),
                IncludeFile::Include
            )
        })
        .map(|path| file_reports(input.db, input.client, path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut reports: Vec<CompilerErrorReport> = per_file.into_iter().flatten().collect();
    reports.sort_by(|a, b| {
        a.path
            .cmp(b.path.reference())
            .then(a.range.start.cmp(b.range.start.reference()))
    });
    reports.wrap_ok()
}

fn file_reports<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Result<Vec<CompilerErrorReport>, ProjectErrorsError> {
    let errors = compiler_errors_in_file(db, client, path)
        .as_ref()
        .ok_or(ProjectErrorsError::ListedPathWithoutText(path))?;
    if errors.is_empty() {
        return Vec::new().wrap_ok();
    }
    let line_index = isograph_lsp::line_index(db, client, path)
        .as_ref()
        .ok_or(ProjectErrorsError::ListedPathWithoutText(path))?;
    let absolute = crate::adapter::absolute_path(db, path)
        .map_err(ProjectErrorsError::MissingWorkingDirectory)?;
    errors
        .iter()
        .map(|error| CompilerErrorReport {
            path: absolute.clone(),
            range: line_index.range(error.location),
            message: error.item.to_string(),
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_compiler::ClientId;
    use prelude::Postfix;

    use super::{CompilerErrorReport, ProjectErrorsError, project_errors};
    use crate::effect::IsographEffect;
    use crate::event::{
        ClientConnectionEvent, ClientStartedEvent, DiskChanged, DiskFileChanged, InternalEvent,
        LiveSession, Presence,
    };
    use crate::state::{IsographState, handle, intern_config_directory};
    use crate::test_profile::TestProfile;
    use crate::watch::ProjectGlobs;

    const BROKEN: &str = "iso(`entrypoint`)";
    const CLEAN: &str = "iso(`entrypoint Query.HomeRoute`)";

    const SOURCE_FILE: isograph_compiler::FileMembership = isograph_compiler::FileMembership {
        source_file: isograph_compiler::IncludeFile::Include,
        schema_file: isograph_compiler::IncludeFile::Skip,
    };

    fn state_with(source_files: &[&str]) -> IsographState<TestProfile> {
        let owned = |globs: &[&str]| globs.iter().map(|g| (*g).to_owned()).collect::<Vec<_>>();
        let mut state = IsographState::new(
            ProjectGlobs::parse(&owned(source_files), &owned(&["schema/**"]), None)
                .expect("the test globs are valid"),
        );
        intern_config_directory(&mut state.db, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    fn state() -> IsographState<TestProfile> {
        state_with(&["src/**"])
    }

    fn present(state: &mut IsographState<TestProfile>, path: &str, contents: &str) {
        handle(
            state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from(path),
                presence: Presence::Present(contents.to_owned()),
            }))
            .to(),
        );
    }

    fn started(state: &mut IsographState<TestProfile>) -> ClientId {
        let (sender, _rx) = crossbeam::channel::unbounded();
        let (assign, _assigned) = crossbeam::channel::bounded(1);
        let effects = handle(
            state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                session: LiveSession {
                    sender,
                    capabilities: lsp_types::ClientCapabilities::default(),
                    published_diagnostics: Default::default(),
                    outstanding: Default::default(),
                },
                assign,
            })
            .to(),
        );
        let [IsographEffect::AssignClientId(assign)] =
            <[IsographEffect; 1]>::try_from(effects).expect("Started returns exactly one effect")
        else {
            panic!("Started returns AssignClientId");
        };
        assign.client
    }

    fn errors(state: &IsographState<TestProfile>) -> Vec<CompilerErrorReport> {
        project_errors(state.compile_input().reference()).expect("the config directory is set")
    }

    fn path(relative: &str) -> RelativePathToSourceFile {
        relative.intern().to()
    }

    #[test]
    fn project_errors_of_a_broken_disk_file_has_its_absolute_path_range_and_message() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", BROKEN);
        let reports = errors(state.reference());
        let input = state.compile_input();
        let diagnostics =
            isograph_lsp::lsp_diagnostics_for_file(input.db, input.client, path("src/a.ts"))
                .clone()
                .expect("the file has text");
        assert!(!reports.is_empty());
        assert!(
            reports
                .iter()
                .all(|report| report.path == Path::new("/tmp/proj/src/a.ts"))
        );
        assert_eq!(
            reports
                .iter()
                .map(|report| (report.range, report.message.clone()))
                .collect::<Vec<_>>(),
            diagnostics
                .into_iter()
                .map(|diagnostic| (diagnostic.range, diagnostic.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn project_errors_ignore_an_open_buffer() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", CLEAN);
        let client = started(&mut state);
        state
            .db
            .set_open_file(client, path("src/a.ts"), SOURCE_FILE, BROKEN.to_owned());
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_read_disk_when_a_client_has_a_clean_buffer() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", BROKEN);
        let client = started(&mut state);
        state
            .db
            .set_open_file(client, path("src/a.ts"), SOURCE_FILE, CLEAN.to_owned());
        assert!(!errors(state.reference()).is_empty());
    }

    #[test]
    fn project_errors_skip_files_outside_the_source_globs() {
        let mut state = state();
        present(&mut state, "/tmp/proj/lib/a.ts", BROKEN);
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_skip_schema_files() {
        let mut state = state();
        present(&mut state, "/tmp/proj/schema/a.graphql", "iso(` `)");
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_are_sorted_by_path_then_start() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/b.ts", BROKEN);
        present(
            &mut state,
            "/tmp/proj/src/a.ts",
            &format!("{BROKEN}\n\n{BROKEN}"),
        );
        let mut positions: Vec<(PathBuf, u32)> = errors(state.reference())
            .into_iter()
            .map(|report| (report.path, report.range.start.line))
            .collect();
        positions.dedup();
        assert_eq!(
            positions,
            [
                (PathBuf::from("/tmp/proj/src/a.ts"), 0),
                (PathBuf::from("/tmp/proj/src/a.ts"), 2),
                (PathBuf::from("/tmp/proj/src/b.ts"), 0),
            ]
        );
    }

    #[test]
    fn project_errors_of_no_disk_files_is_empty() {
        let state = state();
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_after_absent_is_empty() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", BROKEN);
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            }))
            .to(),
        );
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_of_a_parent_relative_path_is_normalized() {
        let mut state = state_with(&["../other/**"]);
        present(&mut state, "/tmp/other/a.ts", BROKEN);
        let reports = errors(state.reference());
        assert!(!reports.is_empty());
        assert!(
            reports
                .iter()
                .all(|report| report.path == Path::new("/tmp/other/a.ts"))
        );
    }

    #[test]
    fn project_errors_without_a_working_directory_is_missing_working_directory() {
        let mut state = IsographState::<TestProfile>::new(
            ProjectGlobs::parse(&["src/**".to_owned()], &[], None).expect("the test glob is valid"),
        );
        state
            .db
            .insert_disk_file(path("src/a.ts"), SOURCE_FILE, BROKEN.to_owned());
        assert_eq!(
            project_errors(state.compile_input().reference()),
            ProjectErrorsError::MissingWorkingDirectory(crate::adapter::MissingWorkingDirectory)
                .wrap_err()
        );
    }

    #[test]
    fn the_compiler_client_is_not_a_started_clients_id() {
        let mut state = state();
        let one = started(&mut state);
        let two = started(&mut state);
        let compiler = state.compile_input().client;
        assert_ne!(one, compiler);
        assert_ne!(two, compiler);
    }
}
