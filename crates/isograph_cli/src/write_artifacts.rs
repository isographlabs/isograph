use std::io;
use std::path::{Path, PathBuf};

use isograph_compiler::ArtifactPath;
use prelude::Postfix;

use crate::effect::WriteArtifactsEffect;
use crate::state::{ArtifactFiles, ArtifactFolder};

/// Every file under `folder` and its text. A missing folder has no files. A file that is not
/// UTF-8 is listed with its lossy text, which matches no artifact, so the next write replaces
/// it.
pub(crate) fn read_artifact_files(folder: &ArtifactFolder) -> ArtifactFiles {
    ArtifactFiles(
        files_under(folder.0.reference())
            .into_iter()
            .filter_map(|file| {
                let relative = file.strip_prefix(folder.0.reference()).ok()?;
                let segments: Vec<&str> = relative
                    .components()
                    .map(|component| component.as_os_str().to_str())
                    .collect::<Option<_>>()?;
                let bytes = std::fs::read(file.reference())
                    .map_err(|error| {
                        tracing::warn!(%error, path = %file.display(), "could not read an artifact")
                    })
                    .ok()?;
                (
                    ArtifactPath(segments.join("/")),
                    String::from_utf8_lossy(bytes.as_slice()).into_owned(),
                )
                    .wrap_some()
            })
            .collect(),
    )
}

/// Every file at any depth under `directory`. A missing directory has none.
fn files_under(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                files_under(path.reference())
            } else {
                path.wrap_vec()
            }
        })
        .collect()
}

/// Applies `effect.changes` under `effect.folder`. Every path it touches is the folder joined
/// with an `ArtifactPath`'s segments.
pub(crate) fn write_artifacts(effect: WriteArtifactsEffect) -> io::Result<()> {
    let WriteArtifactsEffect { folder, changes } = effect;
    changes.writes.iter().try_for_each(|(path, text)| {
        let file = file_path(folder.reference(), path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, text)
    })?;
    changes.deletes.iter().try_for_each(|path| {
        let file = file_path(folder.reference(), path);
        std::fs::remove_file(file.reference())?;
        remove_empty_ancestors(folder.0.reference(), file.reference());
        ().wrap_ok()
    })
}

fn file_path(folder: &ArtifactFolder, path: &ArtifactPath) -> PathBuf {
    path.0
        .split('/')
        .fold(folder.0.clone(), |joined, segment| joined.join(segment))
}

/// Removes each folder between `file` and `folder`, innermost first, until one is not empty.
fn remove_empty_ancestors(folder: &Path, file: &Path) {
    let _ = file
        .ancestors()
        .skip(1)
        .take_while(|ancestor| *ancestor != folder && ancestor.starts_with(folder))
        .try_for_each(std::fs::remove_dir);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use isograph_compiler::ArtifactPath;
    use prelude::Postfix;

    use super::{read_artifact_files, write_artifacts};
    use crate::effect::{ArtifactChanges, WriteArtifactsEffect};
    use crate::state::{ArtifactFiles, ArtifactFolder};

    fn path(text: &str) -> ArtifactPath {
        ArtifactPath(text.to_owned())
    }

    fn write(root: &Path, relative: &str, text: &str) {
        let file = root.join(relative);
        std::fs::create_dir_all(file.parent().expect("a test file has a parent"))
            .expect("a test can create folders");
        std::fs::write(file, text).expect("a test can write a file");
    }

    fn folder_in(root: &Path) -> ArtifactFolder {
        ArtifactFolder(root.join("src/__isograph"))
    }

    #[test]
    fn reading_a_missing_folder_has_no_files() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        assert_eq!(
            read_artifact_files(&folder_in(dir.path())),
            ArtifactFiles::default()
        );
    }

    #[test]
    fn reading_a_nested_folder_lists_every_file_with_its_text() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        write(dir.path(), "src/__isograph/iso.ts", "iso");
        write(dir.path(), "src/__isograph/Query/A/entrypoint.ts", "a");
        write(dir.path(), "src/beside.ts", "beside");
        assert_eq!(
            read_artifact_files(&folder_in(dir.path())),
            ArtifactFiles(BTreeMap::from([
                (path("Query/A/entrypoint.ts"), "a".to_owned()),
                (path("iso.ts"), "iso".to_owned()),
            ]))
        );
    }

    #[test]
    fn writing_changes_touches_only_the_listed_files_inside_the_folder() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let folder = folder_in(dir.path());
        write(dir.path(), "src/__isograph/iso.ts", "old iso");
        write(dir.path(), "src/__isograph/Query/A/entrypoint.ts", "a");
        write(dir.path(), "src/__isograph/Old/B/entrypoint.ts", "stale");
        write(dir.path(), "src/beside.ts", "beside");
        let unchanged = dir.path().join("src/__isograph/Query/A/entrypoint.ts");
        let modified_before = std::fs::metadata(unchanged.as_path())
            .and_then(|metadata| metadata.modified())
            .expect("the test wrote this file");

        write_artifacts(WriteArtifactsEffect {
            folder: folder.clone(),
            changes: ArtifactChanges {
                writes: vec![
                    (path("iso.ts"), "new iso".to_owned()),
                    (path("Pet/C/entrypoint.ts"), "c".to_owned()),
                ],
                deletes: path("Old/B/entrypoint.ts").wrap_vec(),
            },
        })
        .expect("the test folder is writable");

        assert_eq!(
            read_artifact_files(&folder),
            ArtifactFiles(BTreeMap::from([
                (path("Pet/C/entrypoint.ts"), "c".to_owned()),
                (path("Query/A/entrypoint.ts"), "a".to_owned()),
                (path("iso.ts"), "new iso".to_owned()),
            ]))
        );
        assert_eq!(
            std::fs::metadata(unchanged.as_path())
                .and_then(|metadata| metadata.modified())
                .expect("the file is still there"),
            modified_before
        );
        assert!(!dir.path().join("src/__isograph/Old").exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("src/beside.ts"))
                .expect("the file beside the folder is still there"),
            "beside"
        );
    }
}
