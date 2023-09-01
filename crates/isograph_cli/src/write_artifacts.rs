use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use common_lang_types::SelectableFieldName;
use intern::{string_key::Intern, Lookup};

use crate::generate_artifacts::{
    Artifact, FetchableResolver, GenerateArtifactsError, NonFetchableResolver, RefetchQueryResolver,
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
                let intermediate_folder = generated_intermediate_folder(
                    &generated_folder_root,
                    &[parent_type.name.lookup()],
                );
                let generated_file_path = intermediate_folder.join(generated_file_name);

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
                let intermediate_folder = generated_intermediate_folder(
                    &generated_folder_root,
                    &[parent_type.name.lookup()],
                );
                let generated_file_path = intermediate_folder.join(generated_file_name);

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
            Artifact::RefetchQuery(refetch_query_resolver) => {
                let RefetchQueryResolver {
                    root_fetchable_field,
                    root_fetchable_field_parent_object,
                    refetch_query_index,
                    ..
                } = &refetch_query_resolver;

                // TODO we will generate many different queries; they need unique names. For now,
                // they have a single name each artifact clobbers the previous.
                let generated_file_name = generated_file_name(
                    format!("__refetch__{}", refetch_query_index)
                        .intern()
                        .into(),
                );
                let intermediate_folder = generated_intermediate_folder(
                    &generated_folder_root,
                    &[
                        root_fetchable_field_parent_object.lookup(),
                        root_fetchable_field.lookup(),
                    ],
                );
                let generated_file_path = intermediate_folder.join(generated_file_name);

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

                let file_contents = refetch_query_resolver.file_contents();

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

fn generated_intermediate_folder(project_root: &PathBuf, items: &[&'static str]) -> PathBuf {
    let mut project_root = project_root.clone();
    for item in items.iter() {
        project_root = project_root.join(item);
    }
    project_root
}
