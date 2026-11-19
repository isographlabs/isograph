use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use common_lang_types::RelativePathToSourceFile;
use intern::Lookup;
use isograph_compiler::{
    ArtifactPath, ArtifactSource, ClientId, CompilationProfile, HostLanguage, IsographDb,
    LiteralParseFailed, artifact_sources_in_file, compiler_errors_in_file, source_file_paths,
};
use prelude::Postfix;

use crate::adapter::MissingWorkingDirectory;
use crate::effect::{ArtifactChanges, IsographEffect, WriteArtifactsEffect, no_effects};
use crate::state::{ArtifactFiles, IsographState};
use crate::timer::TimerPayload;

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
    pub client: ClientId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ProjectErrorsError {
    #[error("{0}")]
    MissingWorkingDirectory(MissingWorkingDirectory),
    #[error("{} is a listed source file but has no text", .0.lookup())]
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

/// Computes every source file's errors and logs the count, then generates the artifacts.
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    project_errors(state.compile_input().reference()).map_or_else(
        |error| tracing::error!(%error, "compile"),
        |reports| tracing::info!(errors = reports.len(), "compiled"),
    );
    generate_artifacts(state)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ArtifactSourcesError {
    #[error("{0}")]
    LiteralParseFailed(LiteralParseFailed),
    #[error("{} is a listed source file but has no text", .0.lookup())]
    ListedPathWithoutText(RelativePathToSourceFile),
}

/// The write of every artifact whose text differs from the artifact folder's. No effects when
/// the config has no `output_directory`, when any literal failed, or when nothing changed.
fn generate_artifacts<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let Some(folder) = state.artifact_folder.clone() else {
        return no_effects();
    };
    let Some(sources) = project_artifact_sources(state.compile_input().reference())
        .map_err(|error| tracing::warn!(%error, "artifacts not written"))
        .ok()
    else {
        return no_effects();
    };
    let next = ArtifactFiles(
        TProfile::HostLanguage::generate_artifacts(sources.reference())
            .into_iter()
            .collect(),
    );
    state
        .artifact_files
        .replace(next)
        .map(|changes| {
            IsographEffect::WriteArtifacts(WriteArtifactsEffect { folder, changes }.boxed())
        })
        .into_iter()
        .collect()
}

/// The artifact sources of every source file on disk as `input.client` reads them. `Err` names
/// the first literal, in path order, that failed.
pub(crate) fn project_artifact_sources<TProfile: CompilationProfile>(
    input: &CompileInput<'_, TProfile>,
) -> Result<BTreeSet<ArtifactSource>, ArtifactSourcesError> {
    let mut paths: Vec<RelativePathToSourceFile> =
        source_file_paths(input.db, input.client).clone();
    paths.sort_by(|a, b| a.lookup().cmp(b.lookup()));
    paths
        .into_iter()
        .map(|path| {
            artifact_sources_in_file(input.db, input.client, path)
                .as_ref()
                .ok_or(ArtifactSourcesError::ListedPathWithoutText(path))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|source| {
            source
                .dereference()
                .map_err(ArtifactSourcesError::LiteralParseFailed)
        })
        .collect()
}

impl ArtifactFiles {
    /// Records `next` as the artifact folder's files and returns how the folder must change.
    /// `None` when nothing changes.
    pub(crate) fn replace(&mut self, next: ArtifactFiles) -> Option<ArtifactChanges> {
        let previous = std::mem::replace(self, next);
        let writes: Vec<(ArtifactPath, String)> = self
            .0
            .iter()
            .filter(|(path, text)| {
                previous.0.get(path.dereference()) != text.dereference().wrap_some()
            })
            .map(|(path, text)| (path.clone(), text.clone()))
            .collect();
        let deletes: Vec<ArtifactPath> = previous
            .0
            .into_keys()
            .filter(|path| !self.0.contains_key(path))
            .collect();
        if writes.is_empty() && deletes.is_empty() {
            return None;
        }
        ArtifactChanges { writes, deletes }.wrap_some()
    }
}

/// Every error of every source file on disk as `input.client` reads it, sorted by path,
/// then by start position.
pub(crate) fn project_errors<TProfile: CompilationProfile>(
    input: &CompileInput<'_, TProfile>,
) -> Result<Vec<CompilerErrorReport>, ProjectErrorsError> {
    let per_file = source_file_paths(input.db, input.client)
        .iter()
        .copied()
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
    const CLEAN: &str = "iso(`entity Query`) iso(`field Query.HomeRoute`)";
    const UNDEFINED_FIELD: &str = "iso(`entity Query`) iso(`field Query.x { nope }`)";

    const PET_SCHEMA: &str = "type Query {\n  pets: [Pet!]!\n  pet(id: ID!): Pet\n}\n\ntype Pet {\n  id: ID!\n  firstName: String!\n  lastName: String!\n  best_friend: Pet\n}\n";
    const PET: &str = "export const fullName = iso(`\n  field Pet.fullName {\n    firstName\n    lastName\n  }\n`);\n\nexport const bestFriend = iso(`\n  field Pet.bestFriend to Pet {\n    best_friend {\n      id\n    }\n  }\n`);\n";
    const HOME_ROUTE: &str = "export const HomeRoute = iso(`\n  field Query.HomeRoute @component {\n    pets {\n      id\n      fullName\n      bestFriend {\n        firstName\n      }\n    }\n  }\n`);\n\nconsole.log(iso(`entrypoint Query.HomeRoute`));\n";

    const SOURCE_FILE: isograph_compiler::FileMembership = isograph_compiler::FileMembership {
        source_file: isograph_compiler::IncludeFile::Include,
        schema_file: isograph_compiler::IncludeFile::Skip,
    };

    fn state_with(source_files: &[&str]) -> IsographState<TestProfile> {
        let owned = |globs: &[&str]| globs.iter().map(|g| (*g).to_owned()).collect::<Vec<_>>();
        let mut state = IsographState::new(
            ProjectGlobs::parse(&owned(source_files), &owned(&["schema/**"]), None)
                .expect("the test globs are valid"),
            None,
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
    fn project_errors_include_a_validation_error_with_its_absolute_path_range_and_message() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", UNDEFINED_FIELD);
        let start = UNDEFINED_FIELD
            .find("nope")
            .expect("the fixture selects nope") as u32;
        assert_eq!(
            errors(state.reference()),
            CompilerErrorReport {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                range: lsp_types::Range::new(
                    lsp_types::Position::new(0, start),
                    lsp_types::Position::new(0, start + 4)
                ),
                message: "Field `Query.nope` is not defined.".to_owned(),
            }
            .wrap_vec()
        );
    }

    #[test]
    fn project_errors_follow_a_schema_change() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/schema/schema.graphql",
            "type Query {\n  a: String\n}\n",
        );
        present(
            &mut state,
            "/tmp/proj/src/a.ts",
            "iso(`field Query.x { nope }`)",
        );
        assert_eq!(
            errors(state.reference())
                .into_iter()
                .map(|report| report.message)
                .collect::<Vec<_>>(),
            "Field `Query.nope` is not defined.".to_owned().wrap_vec()
        );
        present(
            &mut state,
            "/tmp/proj/schema/schema.graphql",
            "type Query {\n  a: String\n  nope: String\n}\n",
        );
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_of_a_clean_pet_demo_like_project_is_empty() {
        let mut state = state();
        present(&mut state, "/tmp/proj/schema/schema.graphql", PET_SCHEMA);
        present(&mut state, "/tmp/proj/src/Pet.ts", PET);
        present(&mut state, "/tmp/proj/src/HomeRoute.ts", HOME_ROUTE);
        assert_eq!(errors(state.reference()), vec![]);
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
            None,
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

#[cfg(test)]
mod generation_tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use isograph_compiler::ArtifactPath;
    use prelude::Postfix;

    use crate::effect::{ArtifactChanges, IsographEffect};
    use crate::event::{DiskChanged, DiskFileChanged, InternalEvent, Presence};
    use crate::state::{
        ArtifactFiles, ArtifactFolder, IsographState, handle, intern_config_directory,
    };
    use crate::test_profile::TestProfile;
    use crate::watch::ProjectGlobs;

    const HOME_ROUTE: &str = "export const A = iso(`entrypoint Query.HomeRoute`);\n";
    const FULL_NAME: &str = "export const fullName = iso(`field Pet.fullName { firstName }`);\n";
    const BROKEN: &str = "iso(`entrypoint`)";

    fn folder() -> ArtifactFolder {
        ArtifactFolder(PathBuf::from("/tmp/proj/src/__isograph"))
    }

    fn state_with_folder(artifact_folder: Option<ArtifactFolder>) -> IsographState<TestProfile> {
        let mut state = IsographState::new(
            ProjectGlobs::parse(&["src/**".to_owned()], &[], Path::new("src").wrap_some())
                .expect("the test glob is valid"),
            artifact_folder,
        );
        intern_config_directory(&mut state.db, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    fn state() -> IsographState<TestProfile> {
        state_with_folder(folder().wrap_some())
    }

    fn disk_changed(state: &mut IsographState<TestProfile>, path: &str, presence: Presence) {
        handle(
            state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from(path),
                presence,
            }))
            .to(),
        );
    }

    fn present(state: &mut IsographState<TestProfile>, path: &str, contents: &str) {
        disk_changed(state, path, Presence::Present(contents.to_owned()));
    }

    /// The effects of the compile fire.
    fn fire(state: &mut IsographState<TestProfile>) -> Vec<IsographEffect> {
        crate::compile::compile(state)
    }

    /// The changes of the one `WriteArtifacts` the compile fire returns.
    fn changes(state: &mut IsographState<TestProfile>) -> ArtifactChanges {
        let [IsographEffect::WriteArtifacts(write)] =
            <[IsographEffect; 1]>::try_from(fire(state)).expect("the fire returns one effect")
        else {
            panic!("the fire returns WriteArtifacts");
        };
        assert_eq!(write.folder, folder());
        write.changes
    }

    fn written_paths(changes: &ArtifactChanges) -> Vec<&str> {
        changes
            .writes
            .iter()
            .map(|(path, _)| path.0.as_str())
            .collect()
    }

    fn deleted_paths(changes: &ArtifactChanges) -> Vec<&str> {
        changes.deletes.iter().map(|path| path.0.as_str()).collect()
    }

    #[test]
    fn the_first_fire_writes_iso_ts_and_the_entrypoint_artifact() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        let changes = changes(&mut state);
        assert_eq!(
            written_paths(changes.reference()),
            ["Query/HomeRoute/entrypoint.ts", "iso.ts"]
        );
        assert!(changes.deletes.is_empty());
    }

    #[test]
    fn handle_of_the_compile_timer_returns_the_write() {
        let mut state = state();
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/Home.ts"),
                presence: Presence::Present(HOME_ROUTE.to_owned()),
            }))
            .to(),
        );
        let [
            IsographEffect::Timer(_diagnostics),
            IsographEffect::Timer(compile),
        ] = <[IsographEffect; 2]>::try_from(effects).expect("DiskChanged returns two effects")
        else {
            panic!("DiskChanged returns the diagnostics timer and the compile timer");
        };
        assert!(matches!(
            handle(&mut state, compile.event.to()).as_slice(),
            [IsographEffect::WriteArtifacts(_)]
        ));
    }

    #[test]
    fn without_an_artifact_folder_the_fire_writes_nothing() {
        let mut state = state_with_folder(None);
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        assert!(fire(&mut state).is_empty());
    }

    #[test]
    fn a_second_fire_with_no_change_writes_nothing() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        changes(&mut state);
        assert!(fire(&mut state).is_empty());
    }

    #[test]
    fn editing_a_selection_set_does_not_change_iso_ts() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Pet.ts", FULL_NAME);
        changes(&mut state);
        present(
            &mut state,
            "/tmp/proj/src/Pet.ts",
            "export const fullName = iso(`field Pet.fullName { firstName lastName }`);\n",
        );
        assert!(fire(&mut state).is_empty());
    }

    #[test]
    fn adding_a_field_rewrites_only_iso_ts() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        changes(&mut state);
        present(&mut state, "/tmp/proj/src/Pet.ts", FULL_NAME);
        let changes = changes(&mut state);
        assert_eq!(written_paths(changes.reference()), ["iso.ts"]);
        assert!(changes.deletes.is_empty());
    }

    #[test]
    fn removing_an_entrypoint_deletes_its_artifact() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(&mut state, "/tmp/proj/src/Pet.ts", FULL_NAME);
        changes(&mut state);
        disk_changed(&mut state, "/tmp/proj/src/Home.ts", Presence::Absent);
        let changes = changes(&mut state);
        assert_eq!(written_paths(changes.reference()), ["iso.ts"]);
        assert_eq!(
            deleted_paths(changes.reference()),
            ["Query/HomeRoute/entrypoint.ts"]
        );
    }

    #[test]
    fn a_parse_error_writes_nothing_until_it_is_fixed() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(&mut state, "/tmp/proj/src/Broken.ts", BROKEN);
        assert!(fire(&mut state).is_empty());
        present(&mut state, "/tmp/proj/src/Broken.ts", FULL_NAME);
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            ["Query/HomeRoute/entrypoint.ts", "iso.ts"]
        );
    }

    #[test]
    fn a_broken_literal_outside_the_source_globs_does_not_block_the_write() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(&mut state, "/tmp/proj/lib/Broken.ts", BROKEN);
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            ["Query/HomeRoute/entrypoint.ts", "iso.ts"]
        );
    }

    #[test]
    fn the_first_fire_diffs_against_the_files_read_from_the_folder() {
        let mut generated = state();
        present(&mut generated, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        let iso = changes(&mut generated)
            .writes
            .into_iter()
            .find(|(path, _)| path.0 == "iso.ts")
            .expect("the fire writes iso.ts");

        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        let effects = handle(
            &mut state,
            InternalEvent::ArtifactFilesRead(ArtifactFiles(BTreeMap::from([
                iso,
                (ArtifactPath("Old/x.ts".to_owned()), "stale".to_owned()),
            ])))
            .to(),
        );
        assert!(effects.is_empty());
        let changes = changes(&mut state);
        assert_eq!(
            written_paths(changes.reference()),
            ["Query/HomeRoute/entrypoint.ts"]
        );
        assert_eq!(deleted_paths(changes.reference()), ["Old/x.ts"]);
    }
}
