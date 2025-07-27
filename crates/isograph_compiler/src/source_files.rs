use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    path::{Path, PathBuf},
};

use common_lang_types::{
    relative_path_from_absolute_and_working_directory, AbsolutePathAndRelativePath,
    CurrentWorkingDirectory, RelativePathToSourceFile, TextSource,
};
use intern::Lookup;
use isograph_config::absolute_and_relative_paths;
use isograph_lang_types::{IsoLiteralsSource, SchemaSource};
use isograph_schema::StandardSources;
use pico::{Database, SourceId};

use crate::{
    batch_compile::BatchCompileError,
    db_singletons::{get_current_working_directory, get_isograph_config},
    isograph_literals::{read_file, read_files_in_folder},
    watch::{ChangedFileKind, SourceEventKind, SourceFileEvent},
};

#[derive(Debug, Clone)]
pub struct SourceFiles {
    pub sources: StandardSources,
    pub iso_literals: IsoLiteralMap,
}

#[derive(Debug, Clone)]
pub struct IsoLiteralMap(pub HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>);

pub fn read_all_source_files(db: &mut Database) -> Result<SourceFiles, Box<dyn Error>> {
    let schema = get_isograph_config(db).schema.clone();
    let schema_source_id = read_schema(db, &schema)?;
    let schema_extension_sources = read_schema_extensions(db)?;
    let iso_literals = read_iso_literals_from_project_root(db)?;
    Ok(SourceFiles {
        sources: StandardSources {
            schema_source_id,
            schema_extension_sources,
        },
        iso_literals,
    })
}

impl SourceFiles {
    pub fn read_updates(
        &mut self,
        db: &mut Database,
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
                ChangedFileKind::Schema => self.handle_update_schema(db, event).err(),
                ChangedFileKind::SchemaExtension => {
                    self.handle_update_schema_extensions(db, event).err()
                }
                ChangedFileKind::JavaScriptSourceFile => {
                    self.handle_update_source_file(db, event).err()
                }
                ChangedFileKind::JavaScriptSourceFolder => {
                    self.handle_update_source_folder(db, event).err()
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
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        let schema = get_isograph_config(db).schema.clone();
        match event_kind {
            SourceEventKind::CreateOrModify(_) => {
                self.sources.schema_source_id = read_schema(db, &schema)?;
            }
            SourceEventKind::Rename((_, target_path)) => {
                if schema.absolute_path != *target_path {
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
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_schema_extension(db, path)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                if get_isograph_config(db)
                    .schema_extensions
                    .iter()
                    .any(|x| x.absolute_path == *target_path)
                {
                    self.create_or_update_schema_extension(db, target_path)?;
                } else {
                    let interned_file_path = relative_path_from_absolute_and_working_directory(
                        get_current_working_directory(db),
                        source_path,
                    );
                    self.sources
                        .schema_extension_sources
                        .remove(&interned_file_path);
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = relative_path_from_absolute_and_working_directory(
                    get_current_working_directory(db),
                    path,
                );
                self.sources
                    .schema_extension_sources
                    .remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn create_or_update_schema_extension(
        &mut self,
        db: &mut Database,
        path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let absolute_and_relative =
            absolute_and_relative_paths(get_current_working_directory(db), path.to_path_buf());
        let schema_id = read_schema(db, &absolute_and_relative)?;
        self.sources
            .schema_extension_sources
            .insert(absolute_and_relative.relative_path, schema_id);
        Ok(())
    }

    fn handle_update_source_file(
        &mut self,
        db: &mut Database,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_iso_literals(db, path)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                let source_file_path = relative_path_from_absolute_and_working_directory(
                    get_current_working_directory(db),
                    source_path,
                );
                if self.iso_literals.0.remove(&source_file_path).is_some() {
                    self.create_or_update_iso_literals(db, target_path)?
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = relative_path_from_absolute_and_working_directory(
                    get_current_working_directory(db),
                    path,
                );
                self.iso_literals.0.remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn create_or_update_iso_literals(
        &mut self,
        db: &mut Database,
        path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let (relative_path, content) =
            read_file(path.to_path_buf(), get_current_working_directory(db))?;
        let source_id = db.set(IsoLiteralsSource {
            relative_path,
            content,
        });
        self.iso_literals.0.insert(relative_path, source_id);
        Ok(())
    }

    fn handle_update_source_folder(
        &mut self,
        db: &mut Database,
        event_kind: &SourceEventKind,
    ) -> Result<(), Box<dyn Error>> {
        match event_kind {
            SourceEventKind::CreateOrModify(folder) => {
                read_iso_literals_from_folder(db, &mut self.iso_literals, folder)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                self.remove_iso_literals_from_folder(
                    source_path,
                    get_current_working_directory(db),
                );
                read_iso_literals_from_folder(db, &mut self.iso_literals, target_path)?;
            }
            SourceEventKind::Remove(path) => {
                self.remove_iso_literals_from_folder(path, get_current_working_directory(db));
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
            .0
            .retain(|file_path, _| !file_path.to_string().starts_with(&relative_path));
    }
}

pub fn read_schema(
    db: &mut Database,
    schema_path: &AbsolutePathAndRelativePath,
) -> Result<SourceId<SchemaSource>, Box<dyn Error>> {
    let content = read_schema_file(&schema_path.absolute_path)?;
    let text_source = TextSource {
        relative_path_to_source_file: schema_path.relative_path,
        span: None,
        current_working_directory: get_current_working_directory(db),
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
) -> Result<BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>, Box<dyn Error>> {
    let config_schema_extensions = get_isograph_config(db).schema_extensions.clone();
    let mut schema_extensions = BTreeMap::new();
    for schema_extension_path in config_schema_extensions.iter() {
        let schema_extension = read_schema(db, schema_extension_path)?;
        schema_extensions.insert(schema_extension_path.relative_path, schema_extension);
    }
    Ok(schema_extensions)
}

pub fn read_iso_literals_from_project_root(
    db: &mut Database,
) -> Result<IsoLiteralMap, Box<dyn Error>> {
    let project_root = get_isograph_config(db).project_root.clone();
    let mut iso_literals = IsoLiteralMap(HashMap::new());
    read_iso_literals_from_folder(db, &mut iso_literals, &project_root)?;
    Ok(iso_literals)
}

pub fn read_iso_literals_from_folder(
    db: &mut Database,
    iso_literals: &mut IsoLiteralMap,
    folder: &Path,
) -> Result<(), Box<dyn Error>> {
    for (relative_path, content) in read_files_in_folder(folder, get_current_working_directory(db))?
    {
        let source_id = db.set(IsoLiteralsSource {
            relative_path,
            content,
        });
        iso_literals.0.insert(relative_path, source_id);
    }
    Ok(())
}
