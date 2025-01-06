use std::path::PathBuf;

use graphql_artifact_generation::get_artifact_path_and_content;
use isograph_config::{create_config, CompilerConfig, GenerateFileExtensionsOption};
use isograph_schema::{Schema, UnvalidatedSchema};

use crate::{
    batch_compile::{BatchCompileError, CompilationStats},
    source_files::SourceFiles,
    watch::SourceFileEvent,
    write_artifacts::write_artifacts_to_disk,
};

pub struct CompilerState {
    pub config: CompilerConfig,
    pub source_files: Option<SourceFiles>,
}

impl CompilerState {
    pub fn new(config_location: PathBuf) -> Self {
        Self {
            config: create_config(config_location),
            source_files: None,
        }
    }

    /// This the "workhorse" command of batch compilation.
    ///
    /// ## Overall plan
    ///
    /// When the compiler runs in batch mode, we must do the following things. This
    /// description is a bit simplified.
    ///
    /// - Read and parse things:
    ///   - Read and parse the GraphQL schema
    ///   - Read and parse the Isograph literals
    /// - Combine everything into an UnvalidatedSchema.
    /// - Turn the UnvalidatedSchema into a ValidatedSchema
    ///   - Note: at this point, we do most of the validations, like ensuring that
    ///     all selected fields exist and are of the correct types, parameters are
    ///     passed when needed, etc.
    /// - Generate an in-memory representation of all of the generated files
    ///   (called artifacts). This step should not fail. It should panic if any
    ///   invariant is violated, or represent that invariant in the type system.
    /// - Delete and recreate the artifacts on disk.
    ///
    /// ## Additional things we do
    ///
    /// In addition to the things we do above, we also do some specific things like:
    ///
    /// - if a client field is defined on an interface, add it to each concrete
    ///   type. So, if User implements Actor, you can define Actor.NameDisplay, and
    ///   select User.NameDisplay
    /// - create fields from exposeAs directives
    ///
    /// These are less "core" to the overall mission, and thus invite the question
    /// of whether they belong in this function, or at all.
    ///
    /// ## Sequentially written vs Salsa architecture
    ///
    /// Isograph is currently written in a fairly sequential fashion, e.g.:
    ///
    /// let result_1 = step_1()?;
    /// let result_2 = step_2()?;
    /// step_3(result_1, result_2)?;
    ///
    /// Where each step is completed before the next one starts. This has advantages:
    /// namely, it is easy to read. But, we most likely want to report all the errors
    /// we can (i.e. from both step_1 and step_2), rather than just the first error
    /// encountered (i.e. just step_1).
    ///
    /// In the long term, we want to describe everything as a tree, e.g.
    /// `step_3 -> [step_1, step_2]`, and this will "naturally" parallelize everything.
    /// This is also necessary to adopt a Rust Analyzer-like (Salsa) architecture, which is
    /// important for language server performance. In a Salsa architecture, we invalidate
    /// leaves (e.g. a given file changed), and invalidate everything that depends on that
    /// leaf. Then, when we need a result (e.g. the errors to show on a given file), we
    /// re-evaluate (or re-use the cached value) of everything from that result on down.
    pub fn batch_compile(self) -> Result<CompilationStats, BatchCompileError> {
        let source_files = SourceFiles::read_and_parse_all_files(&self.config)?;
        let stats = source_files.contains_iso.stats();
        let total_artifacts_written = validate_and_create_artifacts_from_source_files(
            source_files,
            &self.config,
            self.config.options.generate_file_extensions,
            self.config.options.no_babel_transform,
        )?;
        Ok(CompilationStats {
            client_field_count: stats.client_field_count,
            entrypoint_count: stats.entrypoint_count,
            total_artifacts_written,
        })
    }

    pub fn compile(&mut self) -> Result<CompilationStats, BatchCompileError> {
        let source_files = SourceFiles::read_and_parse_all_files(&self.config)?;
        let stats = source_files.contains_iso.stats();
        self.source_files = Some(source_files.clone());
        let total_artifacts_written = validate_and_create_artifacts_from_source_files(
            source_files,
            &self.config,
            self.config.options.generate_file_extensions,
            self.config.options.no_babel_transform,
        )?;
        Ok(CompilationStats {
            client_field_count: stats.client_field_count,
            entrypoint_count: stats.entrypoint_count,
            total_artifacts_written,
        })
    }

    pub fn update(
        &mut self,
        changes: &[SourceFileEvent],
    ) -> Result<CompilationStats, BatchCompileError> {
        let source_files = self.update_and_clone_source_files(changes)?;
        let stats = source_files.contains_iso.stats();
        let total_artifacts_written = validate_and_create_artifacts_from_source_files(
            source_files,
            &self.config,
            self.config.options.generate_file_extensions,
            self.config.options.no_babel_transform,
        )?;
        Ok(CompilationStats {
            client_field_count: stats.client_field_count,
            entrypoint_count: stats.entrypoint_count,
            total_artifacts_written,
        })
    }

    fn update_and_clone_source_files(
        &mut self,
        changes: &[SourceFileEvent],
    ) -> Result<SourceFiles, BatchCompileError> {
        match &mut self.source_files {
            Some(source_files) => {
                source_files.update(&self.config, changes)?;
                Ok(source_files.clone())
            }
            None => {
                let source_files = SourceFiles::read_and_parse_all_files(&self.config)?;
                self.source_files = Some(source_files.clone());
                Ok(source_files)
            }
        }
    }
}

pub fn validate_and_create_artifacts_from_source_files(
    source_files: SourceFiles,
    config: &CompilerConfig,
    file_extensions: GenerateFileExtensionsOption,
    no_babel_transform: bool,
) -> Result<usize, BatchCompileError> {
    // Create schema
    let mut unvalidated_schema = UnvalidatedSchema::new();
    source_files.create_unvalidated_schema(&mut unvalidated_schema, config)?;

    // Validate
    let validated_schema = Schema::validate_and_construct(unvalidated_schema)?;

    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.
    let artifacts = get_artifact_path_and_content(
        &validated_schema,
        &config.project_root,
        &config.artifact_directory,
        file_extensions,
        no_babel_transform,
    );

    let total_artifacts_written = write_artifacts_to_disk(artifacts, &config.artifact_directory)?;
    Ok(total_artifacts_written)
}
