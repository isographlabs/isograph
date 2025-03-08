use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
};

use common_lang_types::{
    relative_path_from_absolute_and_working_directory, AbsolutePathAndRelativePath,
    CurrentWorkingDirectory, RelativePathToSourceFile, TextSource,
};
use intern::Lookup;
use isograph_config::{absolute_and_relative_paths, CompilerConfig};
use isograph_lang_types::{IsoLiteralsSource, SchemaSource};
use pico::{Database, SourceId};

use crate::{
    batch_compile::BatchCompileError,
    isograph_literals::{read_file, read_files_in_folder},
    watch::{ChangedFileKind, SourceEventKind, SourceFileEvent},
};

#[derive(Debug, Clone)]
pub struct SourceFiles {
    pub schema: SourceId<SchemaSource>,
    pub schema_extensions: HashMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
    pub iso_literals: HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>,
}

impl SourceFiles {
    pub fn read_all(db: &mut Database, config: &CompilerConfig) -> Result<Self, Box<dyn Error>> {
        let schema = read_schema(db, &config.schema, config.current_working_directory)?;
        let schema_extensions = read_schema_extensions(db, config)?;
        let iso_literals = read_iso_literals_from_project_root(db, config)?;
        Ok(Self {
            schema,
            schema_extensions,
            iso_literals,
        })
    }

    pub fn read_updates(
        &mut self,
        db: &mut Database,
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
                ChangedFileKind::Schema => self.handle_update_schema(db, config, event).err(),
                ChangedFileKind::SchemaExtension => self
                    .handle_update_schema_extensions(db, config, event)
                    .err(),
                ChangedFileKind::JavaScriptSourceFile => {
                    self.handle_update_source_file(db, config, event).err()
                }
                ChangedFileKind::JavaScriptSourceFolder => {
                    self.handle_update_source_folder(db, config, event).err()
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
        db: &mut Database,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(_) => {
                self.schema = read_schema(db, &config.schema, config.current_working_directory)?;
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
        db: &mut Database,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_schema_extension(db, path, config)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                if config
                    .schema_extensions
                    .iter()
                    .any(|x| x.absolute_path == *target_path)
                {
                    self.create_or_update_schema_extension(db, target_path, config)?;
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

    fn create_or_update_schema_extension(
        &mut self,
        db: &mut Database,
        path: &Path,
        config: &CompilerConfig,
    ) -> Result<(), Box<dyn Error>> {
        let absolute_and_relative =
            absolute_and_relative_paths(config.current_working_directory, path.to_path_buf());
        let schema_id = read_schema(db, &absolute_and_relative, config.current_working_directory)?;
        self.schema_extensions
            .insert(absolute_and_relative.relative_path, schema_id);
        Ok(())
    }

    fn handle_update_source_file(
        &mut self,
        db: &mut Database,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_iso_literals(db, path, config)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                let source_file_path = relative_path_from_absolute_and_working_directory(
                    config.current_working_directory,
                    source_path,
                );
                if self.iso_literals.remove(&source_file_path).is_some() {
                    self.create_or_update_iso_literals(db, target_path, config)?
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = relative_path_from_absolute_and_working_directory(
                    config.current_working_directory,
                    path,
                );
                self.iso_literals.remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn create_or_update_iso_literals(
        &mut self,
        db: &mut Database,
        path: &Path,
        config: &CompilerConfig,
    ) -> Result<(), Box<dyn Error>> {
        let (relative_path, content) =
            read_file(path.to_path_buf(), config.current_working_directory)?;
        let source_id = db.set(IsoLiteralsSource {
            relative_path,
            content,
        });
        self.iso_literals.insert(relative_path, source_id);
        Ok(())
    }

    fn handle_update_source_folder(
        &mut self,
        db: &mut Database,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(folder) => {
                read_iso_literals_from_folder(db, &mut self.iso_literals, folder, config)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                self.remove_iso_literals_from_folder(source_path, config.current_working_directory);
                read_iso_literals_from_folder(db, &mut self.iso_literals, target_path, config)?;
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
        self.iso_literals
            .retain(|file_path, _| !file_path.to_string().starts_with(&relative_path));
    }
}

pub fn read_schema(
    db: &mut Database,
    schema_path: &AbsolutePathAndRelativePath,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<SourceId<SchemaSource>, Box<dyn Error>> {
    let content = read_schema_file(&schema_path.absolute_path)?;
    let text_source = TextSource {
        relative_path_to_source_file: schema_path.relative_path,
        span: None,
        current_working_directory,
    };
    let schema_id = db.set(SchemaSource {
        relative_path: schema_path.relative_path,
        content,
        text_source,
    });
    Ok(schema_id)
}

pub fn read_schema_file(path: &PathBuf) -> Result<String, BatchCompileError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(path);
    let canonicalized_existing_path =
        joined
            .canonicalize()
            .map_err(|e| BatchCompileError::UnableToLoadSchema {
                path: joined,
                message: e.to_string(),
            })?;

    if !canonicalized_existing_path.is_file() {
        return Err(BatchCompileError::SchemaNotAFile {
            path: canonicalized_existing_path,
        });
    }

    let contents = std::fs::read(canonicalized_existing_path.clone()).map_err(|e| {
        BatchCompileError::UnableToReadFile {
            path: canonicalized_existing_path.clone(),
            message: e.to_string(),
        }
    })?;

    let contents = std::str::from_utf8(&contents)
        .map_err(|e| BatchCompileError::UnableToConvertToString {
            path: canonicalized_existing_path.clone(),
            reason: e,
        })?
        .to_owned();

    Ok(contents)
}

pub fn read_schema_extensions(
    db: &mut Database,
    config: &CompilerConfig,
) -> Result<HashMap<RelativePathToSourceFile, SourceId<SchemaSource>>, Box<dyn Error>> {
    let mut schema_extensions = HashMap::new();
    for schema_extension_path in config.schema_extensions.iter() {
        let schema_extension =
            read_schema(db, schema_extension_path, config.current_working_directory)?;
        schema_extensions.insert(schema_extension_path.relative_path, schema_extension);
    }
    Ok(schema_extensions)
}

pub fn read_iso_literals_from_project_root(
    db: &mut Database,
    config: &CompilerConfig,
) -> Result<HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>, Box<dyn Error>> {
    let mut iso_literals = HashMap::new();
    read_iso_literals_from_folder(db, &mut iso_literals, &config.project_root, config)?;
    Ok(iso_literals)
}

pub fn read_iso_literals_from_folder(
    db: &mut Database,
    iso_literals: &mut HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>,
    folder: &Path,
    config: &CompilerConfig,
) -> Result<(), Box<dyn Error>> {
    for (relative_path, content) in read_files_in_folder(folder, config.current_working_directory)?
    {
        let source_id = db.set(IsoLiteralsSource {
            relative_path,
            content,
        });
        iso_literals.insert(relative_path, source_id);
    }
    Ok(())
}
