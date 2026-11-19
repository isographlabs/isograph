use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use common_lang_types::RelativePathToSourceFile;
use intern::Lookup;
use isograph_compiler::{
    ArtifactPath, ArtifactSource, ClientId, CompilationProfile, HostLanguage, IsographDb,
    LiteralParseFailed, artifact_sources_in_file, compiler_errors_in_file, project_artifacts,
    schema_errors_in_file, schema_file_paths, source_file_paths,
};
use prelude::Postfix;
use span::WithSpan;

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

/// Computes every file's errors and logs the count, then generates the artifacts when there
/// are none.
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    match project_errors(state.compile_input().reference()) {
        Err(error) => {
            tracing::error!(%error, "compile");
            no_effects()
        }
        Ok(reports) => {
            tracing::info!(errors = reports.len(), "compiled");
            if reports.is_empty() {
                generate_artifacts(state)
            } else {
                no_effects()
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ArtifactSourcesError {
    #[error("{0}")]
    LiteralParseFailed(LiteralParseFailed),
    #[error("{} is a listed source file but has no text", .0.lookup())]
    ListedPathWithoutText(RelativePathToSourceFile),
}

/// The write of every artifact whose text differs from the artifact folder's. No effects when
/// the config has no `output_directory`, when any literal failed, when the project cannot be
/// typed or printed, or when nothing changed.
fn generate_artifacts<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let Some(output) = state.artifact_output.clone() else {
        return no_effects();
    };
    let input = state.compile_input();
    let Some(files) = project_artifact_sources(input.reference())
        .map_err(|error| tracing::error!(%error, "artifacts not written"))
        .ok()
        .and_then(|sources| {
            project_artifacts(input.db, input.client, sources.reference())
                .map_err(|error| tracing::error!(%error, "artifacts not written"))
                .ok()
        })
        .and_then(|project| {
            TProfile::HostLanguage::generate_artifacts(
                project.reference(),
                output.generation.reference(),
            )
            .map_err(|error| tracing::error!(%error, "artifacts not written"))
            .ok()
        })
    else {
        return no_effects();
    };
    state
        .artifact_files
        .replace(ArtifactFiles(files.into_iter().collect()))
        .map(|changes| {
            IsographEffect::WriteArtifacts(
                WriteArtifactsEffect {
                    folder: output.folder,
                    changes,
                }
                .boxed(),
            )
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
    let source_reports = source_file_paths(input.db, input.client)
        .iter()
        .copied()
        .map(|path| {
            file_reports(
                input.db,
                input.client,
                path,
                compiler_errors_in_file(input.db, input.client, path).as_ref(),
            )
        });
    let schema_reports = schema_file_paths(input.db, input.client)
        .iter()
        .copied()
        .map(|path| {
            file_reports(
                input.db,
                input.client,
                path,
                schema_errors_in_file(input.db, input.client, path).as_ref(),
            )
        });
    let per_file = source_reports
        .chain(schema_reports)
        .collect::<Result<Vec<_>, _>>()?;
    let mut reports: Vec<CompilerErrorReport> = per_file.into_iter().flatten().collect();
    reports.sort_by(|a, b| {
        a.path
            .cmp(b.path.reference())
            .then(a.range.start.cmp(b.range.start.reference()))
    });
    reports.wrap_ok()
}

/// The reports of `errors`, the errors of `path`; `None` when the path has no text.
fn file_reports<TProfile: CompilationProfile, TError: std::fmt::Display>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    errors: Option<&Vec<WithSpan<TError>>>,
) -> Result<Vec<CompilerErrorReport>, ProjectErrorsError> {
    let errors = errors.ok_or(ProjectErrorsError::ListedPathWithoutText(path))?;
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
    const CLEAN: &str =
        "iso(`entity Query`) export const HomeRoute = iso(`field Query.HomeRoute`)(() => null)";
    const UNDEFINED_FIELD: &str =
        "iso(`entity Query`) export const x = iso(`field Query.x { nope }`)(() => null)";

    const PET_SCHEMA: &str = "scalar ID\nscalar String\ntype Query {\n  pets: [Pet!]!\n  pet(id: ID!): Pet\n}\n\ntype Pet {\n  id: ID!\n  firstName: String!\n  lastName: String!\n  best_friend: Pet\n}\n";
    const PET: &str = "export const fullName = iso(`\n  field Pet.fullName {\n    firstName\n    lastName\n  }\n`)(() => null);\n\nexport const bestFriend = iso(`\n  field Pet.bestFriend to Pet {\n    best_friend {\n      id\n    }\n  }\n`)(() => null);\n";
    const HOME_ROUTE: &str = "export const HomeRoute = iso(`\n  field Query.HomeRoute {\n    pets {\n      id\n      fullName\n      bestFriend {\n        firstName\n      }\n    }\n  }\n`)(() => null);\n\nconsole.log(iso(`entrypoint Query.HomeRoute`));\n";

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
    fn project_errors_ignore_an_editor_buffer() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", CLEAN);
        let client = started(&mut state);
        state
            .db
            .set_editor_buffer(client, path("src/a.ts"), SOURCE_FILE, BROKEN.to_owned());
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_read_disk_when_a_client_has_a_clean_buffer() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/a.ts", BROKEN);
        let client = started(&mut state);
        state
            .db
            .set_editor_buffer(client, path("src/a.ts"), SOURCE_FILE, CLEAN.to_owned());
        assert!(!errors(state.reference()).is_empty());
    }

    #[test]
    fn project_errors_skip_files_outside_the_source_globs() {
        let mut state = state();
        present(&mut state, "/tmp/proj/lib/a.ts", BROKEN);
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_of_a_schema_file_are_its_schema_errors() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/schema/a.graphql",
            "type Query {\n  a: String\n}\ntype {",
        );
        assert_eq!(
            errors(state.reference())
                .into_iter()
                .map(|report| (report.path, report.range.start, report.message))
                .collect::<Vec<_>>(),
            (
                PathBuf::from("/tmp/proj/schema/a.graphql"),
                lsp_types::Position::new(3, 5),
                "Expected non-variable identifier (e.g. 'x' or 'Foo'), found open brace ('{')."
                    .to_owned()
            )
                .wrap_vec()
        );
    }

    #[test]
    fn project_errors_of_a_clean_schema_file_are_empty() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/schema/a.graphql",
            "type Query { a: String }",
        );
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
            "scalar String\ntype Query {\n  a: String\n}\n",
        );
        present(
            &mut state,
            "/tmp/proj/src/a.ts",
            "export const x = iso(`field Query.x { nope }`)(() => null)",
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
            "scalar String\ntype Query {\n  a: String\n  nope: String\n}\n",
        );
        assert_eq!(errors(state.reference()), vec![]);
    }

    #[test]
    fn project_errors_include_a_missing_export_on_the_field_name() {
        let mut state = state();
        let text = "iso(`entity Query`) iso(`field Query.HomeRoute`)(() => null)";
        present(&mut state, "/tmp/proj/src/a.ts", text);
        let start = text.find("HomeRoute").expect("the fixture names HomeRoute") as u32;
        assert_eq!(
            errors(state.reference()),
            CompilerErrorReport {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                range: lsp_types::Range::new(
                    lsp_types::Position::new(0, start),
                    lsp_types::Position::new(0, start + 9)
                ),
                message: "This isograph field literal must be exported as a named export, for example as `export const HomeRoute`".to_owned(),
            }
            .wrap_vec()
        );
    }

    #[test]
    fn project_errors_include_a_generation_error_with_its_absolute_range() {
        let mut state = state();
        present(&mut state, "/tmp/proj/schema/schema.graphql", PET_SCHEMA);
        let text = "export const Only = iso(`field Query.Only`)(() => null);\niso(`entrypoint Query.Only`);\n";
        present(&mut state, "/tmp/proj/src/a.ts", text);
        assert_eq!(
            errors(state.reference()),
            CompilerErrorReport {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                range: lsp_types::Range::new(
                    lsp_types::Position::new(1, 22),
                    lsp_types::Position::new(1, 26)
                ),
                message: "The entrypoint selects nothing the server returns.".to_owned(),
            }
            .wrap_vec()
        );
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

    use common_lang_types::{EntityName, EntityNameAndSelectableName};
    use intern::string_key::Intern;
    use isograph_compiler::{
        ArtifactPath, CompilationProfile, EntrypointQueryError, FieldArtifactKind, Generation,
        GenerationError, GenerationOptions, LinkType, ProfileProjectArtifactsError,
        ProjectArtifacts, ProjectArtifactsError, TopLevelError, UnionWrapper, Wrapper,
        WrapperVariant, project_artifacts,
    };
    use isograph_extract_typescript::{
        TypeScriptCustomScalarType, TypeScriptHostLanguage, TypeScriptLiteralContext,
    };
    use prelude::Postfix;

    use super::project_artifact_sources;
    use crate::effect::{ArtifactChanges, IsographEffect};
    use crate::event::{DiskChanged, DiskFileChanged, InternalEvent, Presence};
    use crate::state::{
        ArtifactFiles, ArtifactFolder, ArtifactOutput, IsographState, handle,
        intern_config_directory,
    };
    use crate::test_profile::TestProfile;
    use crate::watch::ProjectGlobs;

    const QUERY_SCHEMA: &str = "scalar ID\nscalar String\ntype Query { id: ID! name: String }\n";
    const HOME_ROUTE: &str = "export const HomeRoute = iso(`field Query.HomeRoute { id }`)(() => null);\niso(`entrypoint Query.HomeRoute`)\n";
    const PET: &str = "iso(`entity Pet`)\nexport const firstName = iso(`field Pet.firstName`)(() => 'a');\nexport const lastName = iso(`field Pet.lastName`)(() => 'b');\nexport const fullName = iso(`field Pet.fullName { firstName }`)(() => 'c');\n";
    const BROKEN: &str = "iso(`entrypoint`)";

    const HOME_ROUTE_FILES: [&str; 10] = [
        "Query/HomeRoute/entrypoint.ts",
        "Query/HomeRoute/normalization_ast.ts",
        "Query/HomeRoute/output_type.ts",
        "Query/HomeRoute/param_type.ts",
        "Query/HomeRoute/query_text.graphql",
        "Query/HomeRoute/query_text.ts",
        "Query/HomeRoute/resolver_reader.ts",
        "Query/__link/output_type.ts",
        "iso.ts",
        "tsconfig.json",
    ];

    fn folder() -> ArtifactFolder {
        ArtifactFolder(PathBuf::from("/tmp/proj/src/__isograph"))
    }

    fn output<TProfile: CompilationProfile>() -> crate::state::ProfileArtifactOutput<TProfile> {
        ArtifactOutput {
            folder: folder(),
            generation: GenerationOptions {
                artifact_folder: PathBuf::from("src/__isograph"),
                custom_scalar_types: TProfile::custom_scalar_types(),
            },
        }
    }

    fn state_of<TProfile: CompilationProfile>(
        artifact_output: Option<crate::state::ProfileArtifactOutput<TProfile>>,
    ) -> IsographState<TProfile> {
        let mut state = IsographState::new(
            ProjectGlobs::parse(
                &["src/**".to_owned()],
                &["schema/**".to_owned()],
                Path::new("src").wrap_some(),
            )
            .expect("the test glob is valid"),
            artifact_output,
        );
        intern_config_directory(&mut state.db, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    /// A state with an artifact folder and `QUERY_SCHEMA` on disk.
    fn state() -> IsographState<TestProfile> {
        let mut state = state_of(output::<TestProfile>().wrap_some());
        present(&mut state, "/tmp/proj/schema/schema.graphql", QUERY_SCHEMA);
        state
    }

    fn disk_changed<TProfile: CompilationProfile>(
        state: &mut IsographState<TProfile>,
        path: &str,
        presence: Presence,
    ) {
        handle(
            state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from(path),
                presence,
            }))
            .to(),
        );
    }

    fn present<TProfile: CompilationProfile>(
        state: &mut IsographState<TProfile>,
        path: &str,
        contents: &str,
    ) {
        disk_changed(state, path, Presence::Present(contents.to_owned()));
    }

    /// The effects of the compile fire.
    fn fire<TProfile: CompilationProfile>(
        state: &mut IsographState<TProfile>,
    ) -> Vec<IsographEffect> {
        crate::compile::compile(state)
    }

    /// The changes of the one `WriteArtifacts` the compile fire returns.
    fn changes<TProfile: CompilationProfile>(
        state: &mut IsographState<TProfile>,
    ) -> ArtifactChanges {
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

    fn named(parent: &str, selectable: &str) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName {
            parent_entity_name: parent.intern().to(),
            selectable_name: selectable.intern().to(),
        }
    }

    fn project(
        state: &IsographState<TestProfile>,
    ) -> Result<ProjectArtifacts<TypeScriptLiteralContext>, ProfileProjectArtifactsError<TestProfile>>
    {
        let input = state.compile_input();
        let sources =
            project_artifact_sources(input.reference()).expect("every fixture literal parses");
        project_artifacts(input.db, input.client, sources.reference())
    }

    #[test]
    fn the_first_fire_writes_iso_ts_tsconfig_and_the_entrypoints_files() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        let changes = changes(&mut state);
        assert_eq!(written_paths(changes.reference()), HOME_ROUTE_FILES);
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
        let mut state = state_of::<TestProfile>(None);
        present(&mut state, "/tmp/proj/schema/schema.graphql", QUERY_SCHEMA);
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
    fn editing_a_selection_set_writes_that_param_type_and_the_newly_selected_output_type() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Pet.ts", PET);
        changes(&mut state);
        present(
            &mut state,
            "/tmp/proj/src/Pet.ts",
            PET.replace("{ firstName }", "{ firstName, lastName }")
                .as_str(),
        );
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            ["Pet/fullName/param_type.ts", "Pet/lastName/output_type.ts"]
        );
    }

    #[test]
    fn a_description_rewrites_the_param_types_that_select_it_and_not_its_own() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Pet.ts", PET);
        changes(&mut state);
        present(
            &mut state,
            "/tmp/proj/src/Pet.ts",
            PET.replace(
                "field Pet.firstName`",
                "field Pet.firstName \"The first name.\"`",
            )
            .as_str(),
        );
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            ["Pet/fullName/param_type.ts"]
        );
    }

    #[test]
    fn removing_an_entrypoint_deletes_its_artifacts() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(&mut state, "/tmp/proj/src/Pet.ts", PET);
        changes(&mut state);
        disk_changed(&mut state, "/tmp/proj/src/Home.ts", Presence::Absent);
        let changes = changes(&mut state);
        assert_eq!(written_paths(changes.reference()), ["iso.ts"]);
        assert_eq!(
            deleted_paths(changes.reference()),
            HOME_ROUTE_FILES[..8].to_vec()
        );
    }

    #[test]
    fn a_parse_error_writes_nothing_until_it_is_fixed() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(&mut state, "/tmp/proj/src/Broken.ts", BROKEN);
        assert!(fire(&mut state).is_empty());
        present(&mut state, "/tmp/proj/src/Broken.ts", "");
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            HOME_ROUTE_FILES
        );
    }

    #[test]
    fn a_validation_error_writes_nothing_until_it_is_fixed() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(
            &mut state,
            "/tmp/proj/src/Other.ts",
            "export const Other = iso(`field Query.Other { nope }`)(() => null);\n",
        );
        assert!(fire(&mut state).is_empty());
        present(&mut state, "/tmp/proj/src/Other.ts", "");
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            HOME_ROUTE_FILES
        );
    }

    #[test]
    fn a_broken_literal_outside_the_source_globs_does_not_block_the_write() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        present(&mut state, "/tmp/proj/lib/Broken.ts", BROKEN);
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            HOME_ROUTE_FILES
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
        assert!(!written_paths(changes.reference()).contains(&"iso.ts"));
        assert_eq!(deleted_paths(changes.reference()), ["Old/x.ts"]);
    }

    fn written_text<'a>(changes: &'a ArtifactChanges, path: &str) -> &'a str {
        changes
            .writes
            .iter()
            .find(|(written, _)| written.0 == path)
            .map(|(_, text)| text.as_str())
            .expect("the fire writes this path")
    }

    #[test]
    fn an_entrypoint_writes_its_query_text_and_the_raw_query_file() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        let changes = changes(&mut state);
        assert_eq!(
            written_text(changes.reference(), "Query/HomeRoute/query_text.graphql"),
            "query HomeRoute {\n  id\n}\n"
        );
        let module = written_text(changes.reference(), "Query/HomeRoute/query_text.ts");
        assert!(module.contains("query HomeRoute {"), "{module}");
        let entrypoint = written_text(changes.reference(), "Query/HomeRoute/entrypoint.ts");
        assert!(entrypoint.contains("queryText: "), "{entrypoint}");
        assert!(entrypoint.contains("from './query_text'"), "{entrypoint}");
    }

    const INLINED: &str = "export const Inner = iso(`field Query.Inner { id }`)(() => null);\nexport const Outer = iso(`field Query.Outer { Inner }`)(() => null);\niso(`entrypoint Query.Outer`)\n";

    #[test]
    fn editing_an_inlined_field_rewrites_its_param_type_reader_the_query_files_and_the_normalization_ast()
     {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Query.ts", INLINED);
        changes(&mut state);
        present(
            &mut state,
            "/tmp/proj/src/Query.ts",
            INLINED
                .replace("Query.Inner { id }", "Query.Inner { id, name }")
                .as_str(),
        );
        let changes = changes(&mut state);
        assert_eq!(
            written_paths(changes.reference()),
            [
                "Query/Inner/param_type.ts",
                "Query/Inner/resolver_reader.ts",
                "Query/Outer/normalization_ast.ts",
                "Query/Outer/query_text.graphql",
                "Query/Outer/query_text.ts",
            ]
        );
        assert_eq!(
            written_text(changes.reference(), "Query/Outer/query_text.graphql"),
            "query Outer {\n  id\n  name\n}\n"
        );
    }

    #[test]
    fn editing_only_a_resolver_body_rewrites_no_artifact() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Query.ts", INLINED);
        changes(&mut state);
        present(
            &mut state,
            "/tmp/proj/src/Query.ts",
            INLINED
                .replace(
                    "Query.Inner { id }`)(() => null)",
                    "Query.Inner { id }`)(() => 1)",
                )
                .as_str(),
        );
        assert!(fire(&mut state).is_empty());
    }

    #[test]
    fn a_failing_query_writes_nothing_until_it_is_fixed() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/src/Home.ts",
            HOME_ROUTE.replace("{ id }", "{ nope }").as_str(),
        );
        assert!(fire(&mut state).is_empty());
        present(&mut state, "/tmp/proj/src/Home.ts", HOME_ROUTE);
        assert_eq!(
            written_paths(changes(&mut state).reference()),
            HOME_ROUTE_FILES
        );
    }

    #[test]
    fn an_entrypoint_query_error_is_an_error_of_the_project() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/src/Home.ts",
            "export const HomeRoute = iso(`field Query.HomeRoute`)(() => null);\niso(`entrypoint Query.HomeRoute`)\n",
        );
        assert!(matches!(
            project(state.reference()),
            Err(ProjectArtifactsError::EntrypointQuery(failed))
                if failed.entrypoint == named("Query", "HomeRoute")
                    && failed.error == EntrypointQueryError::TopLevel(TopLevelError::Empty)
        ));
    }

    const REACHABILITY: &str = "export const a = iso(`field Query.a { id, b }`)(() => 1);\nexport const b = iso(`field Query.b`)(() => 1);\nexport const c = iso(`field Query.c`)(() => 1);\niso(`entrypoint Query.a`)\n";

    fn generation_of(
        project: &ProjectArtifacts<TypeScriptLiteralContext>,
        field: &str,
    ) -> (Generation, Option<usize>) {
        let field = project
            .fields
            .iter()
            .find(|artifacts| artifacts.field == named("Query", field))
            .expect("the fixture declares the field");
        (
            field.output_type,
            field.resolver_reader.as_ref().map(|reader| reader.0.len()),
        )
    }

    #[test]
    fn readers_are_generated_for_reachable_fields_and_output_types_for_selected_ones() {
        let mut state = state();
        present(&mut state, "/tmp/proj/src/Query.ts", REACHABILITY);
        let project = project(state.reference()).expect("the fixture is valid");
        assert_eq!(
            generation_of(project.reference(), "a"),
            (Generation::Generated, 2.wrap_some())
        );
        assert_eq!(
            generation_of(project.reference(), "b"),
            (Generation::Generated, 0.wrap_some())
        );
        assert_eq!(
            generation_of(project.reference(), "c"),
            (Generation::NotGenerated, None)
        );
    }

    #[test]
    fn a_pointer_has_a_link_type_for_its_target() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/src/Pet.ts",
            "iso(`entity Pet`)\nexport const b = iso(`field Pet.b`)(() => 1);\nexport const p = iso(`field Pet.p to Pet { b }`)(() => null);\n",
        );
        let project = project(state.reference()).expect("the fixture is valid");
        let pet: EntityName = "Pet".intern().to();
        assert_eq!(
            project.link_types,
            LinkType {
                entity: pet,
                concrete_types: pet.wrap_vec(),
            }
            .wrap_vec()
        );
        let p = project
            .fields
            .iter()
            .find(|artifacts| artifacts.field == named("Pet", "p"))
            .expect("the fixture declares p");
        assert_eq!(
            p.kind,
            FieldArtifactKind::Pointer(Wrapper::Union(UnionWrapper(vec![
                WrapperVariant::Inner(pet),
                WrapperVariant::Null,
            ])))
        );
    }

    #[test]
    fn an_entrypoint_with_no_field_is_a_lookup_error() {
        let mut state = state();
        present(
            &mut state,
            "/tmp/proj/src/Pet.ts",
            "iso(`entrypoint Pet.missing`)\n",
        );
        assert!(matches!(
            project(state.reference()),
            Err(ProjectArtifactsError::Generation(GenerationError::Lookup(
                _
            )))
        ));
    }

    struct UrlProfile;

    impl CompilationProfile for UrlProfile {
        type HostLanguage = TypeScriptHostLanguage;
        type NetworkProtocol = graphql_network_protocol::GraphQLNetworkProtocol;

        fn custom_scalar_types() -> BTreeMap<EntityName, TypeScriptCustomScalarType> {
            BTreeMap::from([(
                "Url".intern().to(),
                TypeScriptCustomScalarType {
                    name: "string",
                    import: None,
                },
            )])
        }
    }

    /// Every file the first fire writes for a schema with `Url` and two fields, one selecting it.
    fn url_project_files<TProfile: CompilationProfile>() -> BTreeMap<ArtifactPath, String> {
        let mut state = state_of::<TProfile>(output::<TProfile>().wrap_some());
        present(
            &mut state,
            "/tmp/proj/schema/schema.graphql",
            "scalar Url scalar ID type Query { id: ID! picture: Url! }",
        );
        present(
            &mut state,
            "/tmp/proj/src/Query.ts",
            "export const Pic = iso(`field Query.Pic { picture }`)(() => 1);\nexport const Id = iso(`field Query.Id { id }`)(() => 1);\n",
        );
        changes(&mut state).writes.into_iter().collect()
    }

    #[test]
    fn a_custom_scalar_mapping_changes_only_the_param_types_that_select_it() {
        let mapped = url_project_files::<UrlProfile>();
        let unmapped = url_project_files::<TestProfile>();
        assert_eq!(
            mapped.keys().collect::<Vec<_>>(),
            unmapped.keys().collect::<Vec<_>>()
        );
        let differing: Vec<&str> = mapped
            .iter()
            .filter(|(path, text)| {
                unmapped.get(path.dereference()) != text.dereference().wrap_some()
            })
            .map(|(path, _)| path.0.as_str())
            .collect();
        assert_eq!(differing, ["Query/Pic/param_type.ts"]);
        assert!(
            mapped[&ArtifactPath("Query/Pic/param_type.ts".to_owned())]
                .contains("readonly picture: string;")
        );
        assert!(
            unmapped[&ArtifactPath("Query/Pic/param_type.ts".to_owned())]
                .contains("readonly picture: unknown;")
        );
    }
}
