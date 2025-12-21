use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use common_lang_types::{
    AbsolutePathAndRelativePath, LocationFreeDiagnostic, LocationFreeDiagnosticResult,
    LocationFreeDiagnosticVecResult, RelativePathToSourceFile, TextSource,
    relative_path_from_absolute_and_working_directory,
};
use intern::Lookup;
use isograph_config::absolute_and_relative_paths;
use isograph_schema::{IsographDatabase, NetworkProtocol, SchemaSource, StandardSources};
use pico::{Database, SourceId};
use prelude::Postfix;

use crate::{
    read_files::{read_file, read_files_in_folder},
    watch::{ChangedFileKind, SourceEventKind, SourceFileEvent},
    write_artifacts::unable_to_do_something_at_path_diagnostic,
};

pub fn initialize_sources<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
) -> LocationFreeDiagnosticResult<()> {
    let schema = db.get_isograph_config().schema.clone();
    let schema_source_id = read_schema(db, &schema)?;
    let schema_extension_sources = read_schema_extensions(db)?;
    *db.get_standard_sources_mut().tracked() = StandardSources {
        schema_source_id,
        schema_extension_sources,
    };
    read_iso_literals_from_project_root(db)
}

pub fn update_sources<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    changes: &[SourceFileEvent],
) -> LocationFreeDiagnosticVecResult<()> {
    let errors = changes
        .iter()
        .filter_map(|(event, change_kind)| match change_kind {
            ChangedFileKind::Config => {
                panic!("Unexpected config file change. This is indicative of a bug in Isograph.");
            }
            ChangedFileKind::Schema => handle_update_schema(db, event).err(),
            ChangedFileKind::SchemaExtension => handle_update_schema_extensions(db, event).err(),
            ChangedFileKind::JavaScriptSourceFile => handle_update_source_file(db, event).err(),
            ChangedFileKind::JavaScriptSourceFolder => handle_update_source_folder(db, event).err(),
        })
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        errors.wrap_err()
    } else {
        Ok(())
    }
}

fn handle_update_schema<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    event_kind: &SourceEventKind,
) -> LocationFreeDiagnosticResult<()> {
    let schema = db.get_isograph_config().schema.clone();
    match event_kind {
        SourceEventKind::CreateOrModify(_) => {
            db.get_standard_sources_mut().tracked().schema_source_id = read_schema(db, &schema)?;
        }
        SourceEventKind::Rename((_, target_path)) => {
            if schema.absolute_path != *target_path {
                db.remove(db.get_standard_sources().untracked().schema_source_id);
                db.get_standard_sources_mut().tracked().schema_source_id = SourceId::default();

                return schema_not_found_diagnostic().wrap_err();
            }
        }
        SourceEventKind::Remove(_) => {
            db.remove(db.get_standard_sources().untracked().schema_source_id);
            db.get_standard_sources_mut().tracked().schema_source_id = SourceId::default();

            return schema_not_found_diagnostic().wrap_err();
        }
    }
    Ok(())
}

fn handle_update_schema_extensions<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    event_kind: &SourceEventKind,
) -> LocationFreeDiagnosticResult<()> {
    match event_kind {
        SourceEventKind::CreateOrModify(path) => {
            create_or_update_schema_extension(db, path)?;
        }
        SourceEventKind::Rename((source_path, target_path)) => {
            if db
                .get_isograph_config()
                .schema_extensions
                .iter()
                .any(|x| x.absolute_path == *target_path)
            {
                create_or_update_schema_extension(db, target_path)?;
            } else {
                let interned_file_path = relative_path_from_absolute_and_working_directory(
                    db.get_current_working_directory(),
                    source_path,
                );
                db.remove_schema_extension(interned_file_path);
            }
        }
        SourceEventKind::Remove(path) => {
            let interned_file_path = relative_path_from_absolute_and_working_directory(
                db.get_current_working_directory(),
                path,
            );
            db.remove_schema_extension(interned_file_path);
        }
    }
    Ok(())
}

fn create_or_update_schema_extension<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    path: &Path,
) -> LocationFreeDiagnosticResult<()> {
    let absolute_and_relative =
        absolute_and_relative_paths(db.get_current_working_directory(), path.to_path_buf());
    let schema_id = read_schema(db, &absolute_and_relative)?;
    db.get_standard_sources_mut()
        .tracked()
        .schema_extension_sources
        .insert(absolute_and_relative.relative_path, schema_id);
    Ok(())
}

fn handle_update_source_file<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    event_kind: &SourceEventKind,
) -> LocationFreeDiagnosticResult<()> {
    match event_kind {
        SourceEventKind::CreateOrModify(path) => {
            create_or_update_iso_literals(db, path)?;
        }
        SourceEventKind::Rename((source_path, target_path)) => {
            let source_file_path = relative_path_from_absolute_and_working_directory(
                db.get_current_working_directory(),
                source_path,
            );
            if db.remove_iso_literal(source_file_path).is_some() {
                create_or_update_iso_literals(db, target_path)?
            }
        }
        SourceEventKind::Remove(path) => {
            let interned_file_path = relative_path_from_absolute_and_working_directory(
                db.get_current_working_directory(),
                path,
            );
            db.remove_iso_literal(interned_file_path);
        }
    }
    Ok(())
}

fn create_or_update_iso_literals<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    path: &Path,
) -> LocationFreeDiagnosticResult<()> {
    let (relative_path, content) =
        // TODO this function should live here
        read_file(path.to_path_buf(), db.get_current_working_directory())?;
    db.insert_iso_literal(relative_path, content);
    Ok(())
}

fn handle_update_source_folder<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    event_kind: &SourceEventKind,
) -> LocationFreeDiagnosticResult<()> {
    match event_kind {
        SourceEventKind::CreateOrModify(folder) => {
            read_iso_literals_from_folder(db, folder)?;
        }
        SourceEventKind::Rename((source_path, target_path)) => {
            remove_iso_literals_from_folder(db, source_path);
            read_iso_literals_from_folder(db, target_path)?;
        }
        SourceEventKind::Remove(path) => {
            remove_iso_literals_from_folder(db, path);
        }
    }
    Ok(())
}

fn remove_iso_literals_from_folder<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    folder: &PathBuf,
) {
    let current_working_directory = db.get_current_working_directory();
    let relative_path =
        pathdiff::diff_paths(folder, PathBuf::from(current_working_directory.lookup()))
            .expect("Expected path to be diffable")
            .to_string_lossy()
            .to_string();
    db.remove_iso_literals_from_path(&relative_path);
}

fn read_schema<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    schema_path: &AbsolutePathAndRelativePath,
) -> LocationFreeDiagnosticResult<SourceId<SchemaSource>> {
    let content = read_schema_file(&schema_path.absolute_path)?;
    let text_source = TextSource {
        relative_path_to_source_file: schema_path.relative_path,
        span: None,
    };
    db.set(SchemaSource {
        relative_path: schema_path.relative_path,
        content,
        text_source,
    })
    .wrap_ok()
}

fn read_schema_file(path: &PathBuf) -> LocationFreeDiagnosticResult<String> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(path);
    let canonicalized_existing_path = joined.canonicalize().map_err(|e| {
        unable_to_do_something_at_path_diagnostic(
            &joined,
            &e.to_string(),
            "canonicalize schema path",
        )
    })?;

    if !canonicalized_existing_path.is_file() {
        return format!(
            "Attempted to load the schema at the following path: \
            {canonicalized_existing_path:?}, but that is not a file."
        )
        .to::<LocationFreeDiagnostic>()
        .wrap_err();
    }

    let contents = std::fs::read(canonicalized_existing_path.clone()).map_err(|e| {
        unable_to_do_something_at_path_diagnostic(
            &canonicalized_existing_path,
            &e.to_string(),
            "read file",
        )
    })?;

    std::str::from_utf8(&contents)
        .map_err(|e| {
            unable_to_do_something_at_path_diagnostic(path, &e.to_string(), "convert to string")
        })?
        .to_owned()
        .wrap_ok()
}

fn read_schema_extensions<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
) -> LocationFreeDiagnosticResult<BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>> {
    let config_schema_extensions = db.get_isograph_config().schema_extensions.clone();
    let mut schema_extensions = BTreeMap::new();
    for schema_extension_path in config_schema_extensions.iter() {
        let schema_extension = read_schema(db, schema_extension_path)?;
        schema_extensions.insert(schema_extension_path.relative_path, schema_extension);
    }
    schema_extensions.wrap_ok()
}

fn read_iso_literals_from_project_root<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
) -> LocationFreeDiagnosticResult<()> {
    let project_root = db.get_isograph_config().project_root.clone();
    read_iso_literals_from_folder(db, &project_root)
}

fn read_iso_literals_from_folder<TNetworkProtocol: NetworkProtocol>(
    db: &mut IsographDatabase<TNetworkProtocol>,
    folder: &Path,
) -> LocationFreeDiagnosticResult<()> {
    for (relative_path, content) in
        // TODO this function should live here
        read_files_in_folder(folder, db.get_current_working_directory())?
    {
        db.insert_iso_literal(relative_path, content);
    }
    Ok(())
}

fn schema_not_found_diagnostic() -> LocationFreeDiagnostic {
    "Schema not found. Cannot proceed without a schema"
        .to_string()
        .into()
}
