use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use common_lang_types::{SourceFileName, TextSource};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{parse_schema, parse_schema_extensions};
use intern::string_key::Intern;
use isograph_config::CompilerConfig;
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_schema::UnvalidatedSchema;

use crate::{
    batch_compile::BatchCompileError,
    isograph_literals::{
        process_iso_literals, read_and_parse_iso_literals, read_file, read_files_in_folder,
    },
    refetch_fields::add_refetch_fields_to_objects,
    schema::read_schema_file,
    watch::{ChangedFileKind, SourceEventKind, SourceFileEvent},
};

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct SourceFiles {
    pub schema: GraphQLTypeSystemDocument,
    pub schema_extensions: HashMap<SourceFileName, GraphQLTypeSystemExtensionDocument>,
    pub contains_iso: ContainsIso,
}

impl SourceFiles {
    pub fn read_and_parse_all_files(config: &CompilerConfig) -> Result<Self, BatchCompileError> {
        let schema = read_and_parse_graphql_schema(&config.schema)?;

        let mut schema_extensions = HashMap::new();
        for schema_extension_path in config.schema_extensions.iter() {
            let (file_path, extensions_document) =
                read_and_parse_schema_extensions(schema_extension_path)?;
            schema_extensions.insert(file_path, extensions_document);
        }

        let mut contains_iso = ContainsIso::default();
        read_and_parse_iso_literals_from_folder(
            &mut contains_iso,
            &config.project_root,
            &config.project_root,
        )?;

        Ok(Self {
            schema,
            schema_extensions,
            contains_iso,
        })
    }

    pub fn create_unvalidated_schema(
        self,
        schema: &mut UnvalidatedSchema,
        config: &CompilerConfig,
    ) -> Result<(), BatchCompileError> {
        let outcome = schema.process_graphql_type_system_document(self.schema, config.options)?;
        for extension_document in self.schema_extensions.into_values() {
            let _extension_outcome = schema
                .process_graphql_type_extension_document(extension_document, config.options)?;
        }
        process_iso_literals(schema, self.contains_iso)?;
        process_exposed_fields(schema)?;
        schema.add_fields_to_subtypes(&outcome.type_refinement_maps.supertype_to_subtype_map)?;
        schema
            .add_pointers_to_supertypes(&outcome.type_refinement_maps.subtype_to_supertype_map)?;
        add_refetch_fields_to_objects(schema)?;
        Ok(())
    }

    pub fn update(
        &mut self,
        config: &CompilerConfig,
        changes: &[SourceFileEvent],
    ) -> Result<(), BatchCompileError> {
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
            Err(BatchCompileError::MultipleErrors { messages: errors })
        } else {
            Ok(())
        }
    }

    fn handle_update_schema(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), BatchCompileError> {
        match event_kind {
            SourceEventKind::CreateOrModify(_) => {
                self.schema = read_and_parse_graphql_schema(&config.schema)?;
            }
            SourceEventKind::Rename((_, target_path)) => {
                if config.schema != *target_path {
                    return Err(BatchCompileError::SchemaNotFound);
                }
            }
            SourceEventKind::Remove(_) => return Err(BatchCompileError::SchemaNotFound),
        }
        Ok(())
    }

    fn handle_update_schema_extensions(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), BatchCompileError> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_schema_extension(path)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                if config.schema_extensions.contains(target_path) {
                    self.create_or_update_schema_extension(target_path)?;
                } else {
                    let interned_file_path = intern_file_path(source_path);
                    self.schema_extensions.remove(&interned_file_path);
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = intern_file_path(path);
                self.schema_extensions.remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn handle_update_source_file(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), BatchCompileError> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                self.create_or_update_iso_literals(&config.project_root, path)?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                let source_file_path = intern_file_path(source_path);
                if self.contains_iso.remove(&source_file_path).is_some() {
                    self.create_or_update_iso_literals(&config.project_root, target_path)?
                }
            }
            SourceEventKind::Remove(path) => {
                let interned_file_path = intern_file_path(path);
                self.contains_iso.remove(&interned_file_path);
            }
        }
        Ok(())
    }

    fn handle_update_source_folder(
        &mut self,
        config: &CompilerConfig,
        event_kind: &SourceEventKind,
    ) -> Result<(), BatchCompileError> {
        match event_kind {
            SourceEventKind::CreateOrModify(path) => {
                read_and_parse_iso_literals_from_folder(
                    &mut self.contains_iso,
                    path,
                    &config.project_root,
                )?;
            }
            SourceEventKind::Rename((source_path, target_path)) => {
                let path_string = source_path.to_string_lossy().to_string();
                self.contains_iso
                    .retain(|file_path, _| !file_path.to_string().starts_with(&path_string));
                read_and_parse_iso_literals_from_folder(
                    &mut self.contains_iso,
                    target_path,
                    &config.project_root,
                )?;
            }
            SourceEventKind::Remove(path) => {
                let path_string = path.to_string_lossy().to_string();
                self.contains_iso
                    .retain(|file_path, _| !file_path.to_string().starts_with(&path_string));
            }
        }
        Ok(())
    }

    fn create_or_update_schema_extension(
        &mut self,
        path: &PathBuf,
    ) -> Result<(), BatchCompileError> {
        let (file_path, document) = read_and_parse_schema_extensions(path)?;
        self.schema_extensions.insert(file_path, document);
        Ok(())
    }

    fn create_or_update_iso_literals(
        &mut self,
        project_root: &PathBuf,
        path: &Path,
    ) -> Result<(), BatchCompileError> {
        let canonicalized_root_path = get_canonicalized_root_path(project_root)?;
        let (path_buf, file_content) = read_file(path.to_path_buf(), &canonicalized_root_path)?;
        let (file_path, iso_literals) =
            read_and_parse_iso_literals(path_buf, file_content, &canonicalized_root_path)?;
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
) -> Result<(), BatchCompileError> {
    let mut iso_literal_parse_errors = vec![];
    let canonicalized_root_path = get_canonicalized_root_path(project_root)?;
    for (path, file_content) in read_files_in_folder(folder, &canonicalized_root_path)? {
        match read_and_parse_iso_literals(path, file_content, &canonicalized_root_path) {
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

fn read_and_parse_graphql_schema(
    schema_path: &PathBuf,
) -> Result<GraphQLTypeSystemDocument, BatchCompileError> {
    let content = read_schema_file(schema_path)?;
    let schema_text_source = TextSource {
        path: schema_path
            .to_str()
            .expect("Expected schema to be valid string")
            .intern()
            .into(),
        span: None,
    };
    let schema = parse_schema(&content, schema_text_source)
        .map_err(|with_span| with_span.to_with_location(schema_text_source))?;
    Ok(schema)
}

fn intern_file_path(path: &Path) -> SourceFileName {
    path.to_string_lossy().into_owned().intern().into()
}

pub fn read_and_parse_schema_extensions(
    schema_extension_path: &PathBuf,
) -> Result<(SourceFileName, GraphQLTypeSystemExtensionDocument), BatchCompileError> {
    let file_path = schema_extension_path
        .to_str()
        .expect("Expected schema extension to be valid string")
        .intern()
        .into();
    let extension_content = read_schema_file(schema_extension_path)?;
    let extension_text_source = TextSource {
        path: file_path,
        span: None,
    };

    let schema_extensions = parse_schema_extensions(&extension_content, extension_text_source)
        .map_err(|with_span| with_span.to_with_location(extension_text_source))?;

    Ok((file_path, schema_extensions))
}

fn get_canonicalized_root_path(project_root: &PathBuf) -> Result<PathBuf, BatchCompileError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(project_root);
    joined
        .canonicalize()
        .map_err(|message| BatchCompileError::UnableToLoadSchema {
            path: joined.clone(),
            message,
        })
}

/// Here, we are processing exposeAs fields. Note that we only process these
/// directives on root objects (Query, Mutation, Subscription) and we should
/// validate that no other types have exposeAs directives.
fn process_exposed_fields(schema: &mut UnvalidatedSchema) -> Result<(), BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    for fetchable_object_id in fetchable_types.into_iter() {
        schema.add_exposed_fields_to_parent_object_types(fetchable_object_id)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ContainsIso(pub HashMap<SourceFileName, Vec<(IsoLiteralExtractionResult, TextSource)>>);

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
    type Target = HashMap<SourceFileName, Vec<(IsoLiteralExtractionResult, TextSource)>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ContainsIso {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct ContainsIsoStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    #[allow(unused)]
    pub client_pointer_count: usize,
}
