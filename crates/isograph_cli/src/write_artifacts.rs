use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use common_lang_types::{IsographObjectTypeName, SelectableFieldName};
use intern::Lookup;

use crate::generate_artifacts::{
    Artifact, FetchableResolver, GenerateArtifactsError, NonFetchableResolver,
};

pub(crate) fn write_artifacts<'schema>(
    artifacts: Vec<Artifact<'schema>>,
    project_root: &PathBuf,
) -> Result<(), GenerateArtifactsError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let project_root = current_dir.join(project_root).canonicalize().map_err(|e| {
        GenerateArtifactsError::UnableToCanonicalizePath {
            path: project_root.clone(),
            message: e,
        }
    })?;

    let generated_folder_root = project_root.join("__isograph");

    if generated_folder_root.exists() {
        fs::remove_dir_all(&generated_folder_root).map_err(|e| {
            GenerateArtifactsError::UnableToDeleteDirectory {
                path: project_root.clone(),
                message: e,
            }
        })?;
    }
    fs::create_dir_all(&generated_folder_root).map_err(|e| {
        GenerateArtifactsError::UnableToCreateDirectory {
            path: project_root.clone(),
            message: e,
        }
    })?;
    for artifact in artifacts {
        match artifact {
            Artifact::FetchableResolver(fetchable_resolver) => {
                let FetchableResolver {
                    query_name,
                    parent_type,
                    ..
                } = &fetchable_resolver;

                let generated_file_name = generated_file_name((*query_name).into());
                let generated_file_path = generated_file_path(
                    &generated_folder_root,
                    parent_type.name,
                    &generated_file_name,
                );
                let intermediate_folder =
                    generated_intermediate_folder(&generated_folder_root, parent_type.name);

                fs::create_dir_all(&intermediate_folder).map_err(|e| {
                    GenerateArtifactsError::UnableToCreateDirectory {
                        path: intermediate_folder.clone(),
                        message: e,
                    }
                })?;

                let mut file = File::create(&generated_file_path).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;

                let file_contents = fetchable_resolver.file_contents();

                file.write(file_contents.as_bytes()).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;
            }
            Artifact::NonFetchableResolver(non_fetchable_resolver) => {
                let NonFetchableResolver {
                    parent_type,
                    resolver_field_name,
                    ..
                } = &non_fetchable_resolver;

                let generated_file_name = generated_file_name(*resolver_field_name);
                let generated_file_path = generated_file_path(
                    &generated_folder_root,
                    parent_type.name,
                    &generated_file_name,
                );
                let intermediate_folder =
                    generated_intermediate_folder(&generated_folder_root, parent_type.name);

                fs::create_dir_all(&intermediate_folder).map_err(|e| {
                    GenerateArtifactsError::UnableToCreateDirectory {
                        path: intermediate_folder.clone(),
                        message: e,
                    }
                })?;

                let mut file = File::create(&generated_file_path).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;

                let file_contents = non_fetchable_resolver.file_contents();

                file.write(file_contents.as_bytes()).map_err(|e| {
                    GenerateArtifactsError::UnableToWriteToArtifactFile {
                        path: generated_file_path.clone(),
                        message: e,
                    }
                })?;
            }
        }
    }
    Ok(())
}

fn generated_file_name(field_name: SelectableFieldName) -> PathBuf {
    PathBuf::from(format!("{}.isograph.tsx", field_name))
}

fn generated_file_path(
    project_root: &PathBuf,
    parent_type_name: IsographObjectTypeName,
    file_name: &PathBuf,
) -> PathBuf {
    project_root.join(parent_type_name.lookup()).join(file_name)
}

fn generated_intermediate_folder(
    project_root: &PathBuf,
    parent_type_name: IsographObjectTypeName,
) -> PathBuf {
    project_root.join(parent_type_name.lookup())
}
