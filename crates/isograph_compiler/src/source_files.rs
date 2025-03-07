use std::{
    collections::HashMap,
    error::Error,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use common_lang_types::{
    relative_path_from_absolute_and_working_directory, CurrentWorkingDirectory,
    RelativePathToSourceFile, TextSource,
};
use intern::Lookup;
use isograph_config::{absolute_and_relative_paths, CompilerConfig};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_schema::{OutputFormat, UnvalidatedSchema};

use crate::{
    batch_compile::BatchCompileError,
    isograph_literals::{
        parse_iso_literals_in_file_content, process_iso_literals, read_file, read_files_in_folder,
    },
    refetch_fields::add_refetch_fields_to_objects,
    watch::{ChangedFileKind, SourceEventKind, SourceFileEvent},
};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct SourceFiles<TOutputFormat: OutputFormat> {
    pub schema: TOutputFormat::TypeSystemDocument,
    pub schema_extensions:
        HashMap<RelativePathToSourceFile, TOutputFormat::TypeSystemExtensionDocument>,
    pub contains_iso: ContainsIso,
    pub output_format: PhantomData<TOutputFormat>,
}

impl<TOutputFormat: OutputFormat> SourceFiles<TOutputFormat> {
    pub fn read_and_parse_all_files(config: &CompilerConfig) -> Result<Self, Box<dyn Error>> {
        let schema = TOutputFormat::read_and_parse_type_system_document(config)?;

        let mut schema_extensions = HashMap::new();
        for schema_extension_path in config.schema_extensions.iter() {
            let (file_path, extensions_document) =
                TOutputFormat::read_and_parse_type_system_extension_document(
                    schema_extension_path,
                    config,
                )?;
            schema_extensions.insert(file_path, extensions_document);
        }

        let mut contains_iso = ContainsIso::default();
        read_and_parse_iso_literals_from_folder(
            &mut contains_iso,
            &config.project_root,
            &config.project_root,
            config,
        )?;

        Ok(Self {
            schema,
            schema_extensions,
            contains_iso,
            output_format: PhantomData,
        })
    }

    pub fn create_unvalidated_schema(
        self,
        unvalidated_isograph_schema: &mut UnvalidatedSchema<TOutputFormat>,
        config: &CompilerConfig,
    ) -> Result<(), Box<dyn Error>> {
        let outcome = TOutputFormat::process_type_system_document(
            unvalidated_isograph_schema,
            self.schema,
            &config.options,
        )?;
        for extension_document in self.schema_extensions.into_values() {
            TOutputFormat::process_type_system_extension_document(
                unvalidated_isograph_schema,
                extension_document,
                &config.options,
            )?;
        }
        process_iso_literals(unvalidated_isograph_schema, self.contains_iso)?;
        process_exposed_fields(unvalidated_isograph_schema)?;
        unvalidated_isograph_schema
            .add_fields_to_subtypes(&outcome.type_refinement_maps.supertype_to_subtype_map)?;
        unvalidated_isograph_schema.add_link_fields()?;
        unvalidated_isograph_schema
            .add_pointers_to_supertypes(&outcome.type_refinement_maps.subtype_to_supertype_map)?;
        add_refetch_fields_to_objects(unvalidated_isograph_schema)?;
        Ok(())
    }

    pub fn update(
        &mut self,
        config: &CompilerConfig,
        changes: &[SourceFileEvent],
    ) -> Result<(), Box<dyn Error>> {
        let errors = changes
            .iter()
            .filter_map(|(event, change_kind)| match change_kind {
                ChangedFileKind::Config => {
                    panic!(
                        "Unexpected config file change. This is indicative of a bug in Isograph."
                    );
                }
                ChangedFileKind::Schema => self.handle_update_schema(config, event).err(),
                ChangedFileKind::SchemaExtension => {
                    self.handle_update_schema_extensions(config, event).err()
                }
                ChangedFileKind::JavaScriptSourceFile => {
                    self.handle_update_source_file(config, event).err()
                }
                ChangedFileKind::JavaScriptSourceFolder => {
                    self.handle_update_source_folder(config, event).err()
                }
            })
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            Err(BatchCompileError::MultipleErrors { messages: errors }.into())
        } else {
            Ok(())
        }
    }

    fn handle_update_schema(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(_) => {
                self.schema = TOutputFormat::read_and_parse_type_system_document(config)?;
            }
            SourceEventKind::Rename((_, target_path)) => {
                if config.schema.absolute_path != *target_path {
                    return Err(Box::new(BatchCompileError::SchemaNotFound));
                }
            }
            SourceEventKind::Remove(_) => return Err(Box::new(BatchCompileError::SchemaNotFound)),
        }
        Ok(())
    }

    fn handle_update_schema_extensions(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_schema_extension(path, config)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                if config
                    .schema_extensions
                    .iter()
                    .any(|x| x.absolute_path == *target_path)
                {
                    self.create_or_update_schema_extension(target_path, config)?;
                } else {
                    let interned_file_path = relative_path_from_absolute_and_working_directory(
                        config.current_working_directory,
                        source_path,
                    );
                    self.schema_extensions.remove(&interned_file_path);
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = relative_path_from_absolute_and_working_directory(
                    config.current_working_directory,
                    path,
                );
                self.schema_extensions.remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn handle_update_source_file(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_iso_literals(&config.project_root, path, config)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                let source_file_path = relative_path_from_absolute_and_working_directory(
                    config.current_working_directory,
                    source_path,
                );
                if self.contains_iso.remove(&source_file_path).is_some() {
                    self.create_or_update_iso_literals(&config.project_root, target_path, config)?
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = relative_path_from_absolute_and_working_directory(
                    config.current_working_directory,
                    path,
                );
                self.contains_iso.remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn handle_update_source_folder(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                read_and_parse_iso_literals_from_folder(
                    &mut self.contains_iso,
                    path,
                    &config.project_root,
                    config,
                )?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                self.remove_iso_literals_from_folder(source_path, config.current_working_directory);
                read_and_parse_iso_literals_from_folder(
                    &mut self.contains_iso,
                    target_path,
                    &config.project_root,
                    config,
                )?;
            }
            SourceEventKind::Remove(path) => {
                self.remove_iso_literals_from_folder(path, config.current_working_directory);
            }
        }
        Ok(())
    }

    fn remove_iso_literals_from_folder(
        &mut self,
        folder: &PathBuf,
        current_working_directory: CurrentWorkingDirectory,
    ) {
        let relative_path =
            pathdiff::diff_paths(folder, PathBuf::from(current_working_directory.lookup()))
                .expect("Expected path to be diffable")
                .to_string_lossy()
                .to_string();
        self.contains_iso
            .retain(|file_path, _| !file_path.to_string().starts_with(&relative_path));
    }

    fn create_or_update_schema_extension(
        &mut self,
        path: &Path,
        config: &CompilerConfig,
    ) -> Result<(), Box<dyn Error>> {
        let absolute_and_relative =
            absolute_and_relative_paths(config.current_working_directory, path.to_path_buf());
        let (file_path, document) = TOutputFormat::read_and_parse_type_system_extension_document(
            &absolute_and_relative,
            config,
        )?;
        self.schema_extensions.insert(file_path, document);
        Ok(())
    }

    fn create_or_update_iso_literals(
        &mut self,
        project_root: &PathBuf,
        path: &Path,
        config: &CompilerConfig,
    ) -> Result<(), BatchCompileError> {
        let canonicalized_root_path = get_canonicalized_root_path(project_root)?;
        let (path_buf, file_content) = read_file(path.to_path_buf(), &canonicalized_root_path)?;
        let (file_path, iso_literals) = parse_iso_literals_in_file_content(
            path_buf,
            file_content,
            &canonicalized_root_path,
            config,
        )?;
        if iso_literals.is_empty() {
            self.contains_iso.remove(&file_path);
        } else {
            self.contains_iso.insert(file_path, iso_literals);
        }
        Ok(())
    }
}

fn read_and_parse_iso_literals_from_folder(
    contains_iso: &mut ContainsIso,
    folder: &Path,
    project_root: &PathBuf,
    config: &CompilerConfig,
) -> Result<(), BatchCompileError> {
    let mut iso_literal_parse_errors = vec![];
    let canonicalized_root_path = get_canonicalized_root_path(project_root)?;
    for (path, file_content) in read_files_in_folder(folder, &canonicalized_root_path)? {
        match parse_iso_literals_in_file_content(
            path,
            file_content,
            &canonicalized_root_path,
            config,
        ) {
            Ok((file_path, iso_literals)) => {
                if !iso_literals.is_empty() {
                    contains_iso.insert(file_path, iso_literals);
                }
            }
            Err(e) => {
                iso_literal_parse_errors.extend(e);
            }
        };
    }
    if iso_literal_parse_errors.is_empty() {
        Ok(())
    } else {
        Err(iso_literal_parse_errors.into())
    }
}

fn get_canonicalized_root_path(project_root: &PathBuf) -> Result<PathBuf, BatchCompileError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(project_root);
    joined
        .canonicalize()
        .map_err(|e| BatchCompileError::UnableToLoadSchema {
            path: joined.clone(),
            message: e.to_string(),
        })
}

/// Here, we are processing exposeAs fields. Note that we only process these
/// directives on root objects (Query, Mutation, Subscription) and we should
/// validate that no other types have exposeAs directives.
fn process_exposed_fields<TOutputFormat: OutputFormat>(
    schema: &mut UnvalidatedSchema<TOutputFormat>,
) -> Result<(), BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    for fetchable_object_id in fetchable_types.into_iter() {
        schema.add_exposed_fields_to_parent_object_types(fetchable_object_id)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ContainsIso {
    pub files: HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>,
}

impl ContainsIso {
    pub fn stats(&self) -> ContainsIsoStats {
        let mut client_field_count: usize = 0;
        let mut client_pointer_count: usize = 0;
        let mut entrypoint_count: usize = 0;
        for iso_literals in self.values() {
            for (iso_literal, ..) in iso_literals {
                match iso_literal {
                    IsoLiteralExtractionResult::ClientFieldDeclaration(_) => {
                        client_field_count += 1
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => entrypoint_count += 1,
                    IsoLiteralExtractionResult::ClientPointerDeclaration(_) => {
                        client_pointer_count += 1
                    }
                }
            }
        }
        ContainsIsoStats {
            client_field_count,
            entrypoint_count,
            client_pointer_count,
        }
    }
}

impl Deref for ContainsIso {
    type Target = HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl DerefMut for ContainsIso {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.files
    }
}

pub struct ContainsIsoStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    #[allow(unused)]
    pub client_pointer_count: usize,
}
