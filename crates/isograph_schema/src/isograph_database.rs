use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

use common_lang_types::{
    CurrentWorkingDirectory, Location, PrintLocationFn, RelativePathToSourceFile, Span, TextSource,
    text_with_carats,
};
use isograph_config::CompilerConfig;
use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};
use prelude::Postfix;

use crate::NetworkProtocol;

#[derive(Default, Debug, Db)]
pub struct IsographDatabase<TNetworkProtocol: NetworkProtocol> {
    storage: Storage<Self>,
    #[tracked]
    iso_literal_map: IsoLiteralMap,
    #[tracked]
    standard_sources: StandardSources,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<TNetworkProtocol>,
}

#[derive(Debug, Default)]
pub struct IsoLiteralMap(pub HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct SchemaSource {
    #[key]
    pub relative_path: RelativePathToSourceFile,
    pub content: String,
    // Do we need this?
    pub text_source: TextSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct IsoLiteralsSource {
    #[key]
    pub relative_path: RelativePathToSourceFile,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct OpenFileMap(pub HashMap<RelativePathToSourceFile, SourceId<OpenFileSource>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct OpenFileSource {
    #[key]
    pub relative_path: RelativePathToSourceFile,
    pub content: String,
}

// We're using this type to constrain the types of sources that we accept. i.e.
// in theory, you can have a TNetworkProtocol with a different Source associated
// type, but for now, we get a source + set of extensions, and have to restrict
// TNetworkProtocol accordingly. Perhaps the config can have a generic, and
// thus we can thread this further back, but that is not yet implemented.
#[derive(Debug, Clone, Default)]
pub struct StandardSources {
    // TODO we should just store this as a singleton, and not have it in standard sources.
    // Or perhaps, we store the schema source directly here.
    pub schema_source_id: SourceId<SchemaSource>,
    pub schema_extension_sources: BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
}

impl<TNetworkProtocol: NetworkProtocol> IsographDatabase<TNetworkProtocol> {
    pub fn get_current_working_directory(&self) -> CurrentWorkingDirectory {
        *self
            .get_singleton::<CurrentWorkingDirectory>()
            .expect("Expected CurrentWorkingDirectory to have been set")
    }

    pub fn get_isograph_config(&self) -> &CompilerConfig {
        self.get_singleton::<CompilerConfig>()
            .expect("Expected CompilerConfig to have been set")
    }

    pub fn get_schema_source(&self) -> &SchemaSource {
        self.get(self.standard_sources.schema_source_id)
    }

    pub fn get_open_file(
        &self,
        file: RelativePathToSourceFile,
    ) -> Option<SourceId<OpenFileSource>> {
        // TODO this really should be untracked, or at minimum,
        // untracked (if the value is Some), and tracked (otherwise).
        // See [this issue](https://github.com/isographlabs/isograph/issues/787)
        self.get_open_file_map().tracked().0.get(&file).cloned()
    }

    pub fn remove_schema_extension(
        &mut self,
        relative_path: RelativePathToSourceFile,
    ) -> Option<SourceId<SchemaSource>> {
        self.get_standard_sources_mut()
            .tracked()
            .schema_extension_sources
            .remove(&relative_path)
            .inspect(|&source_id| self.remove(source_id))
    }

    pub fn get_iso_literal(
        &self,
        relative_path: RelativePathToSourceFile,
    ) -> Option<SourceId<IsoLiteralsSource>> {
        self.get_iso_literal_map()
            .tracked()
            .0
            .get(&relative_path)
            .cloned()
    }

    pub fn insert_iso_literal(&mut self, relative_path: RelativePathToSourceFile, content: String) {
        let source_id = self.set(IsoLiteralsSource {
            relative_path,
            content,
        });
        self.get_iso_literal_map_mut()
            .tracked()
            .0
            .insert(relative_path, source_id);
    }

    pub fn remove_iso_literal(
        &mut self,
        relative_path: RelativePathToSourceFile,
    ) -> Option<SourceId<IsoLiteralsSource>> {
        self.get_iso_literal_map_mut()
            .tracked()
            .0
            .remove(&relative_path)
            .inspect(|&source_id| self.remove(source_id))
    }

    pub fn remove_iso_literals_from_path(&mut self, relative_path: &str) {
        let removed_source_ids = self
            .get_iso_literal_map_mut()
            .tracked()
            .0
            .extract_if(|k, _| k.to_string().starts_with(relative_path))
            .map(|(_, v)| v)
            .collect::<Vec<_>>();

        for source_id in removed_source_ids {
            self.remove(source_id);
        }
    }

    pub fn insert_open_file(&mut self, relative_path: RelativePathToSourceFile, content: String) {
        let source_id = self.set(OpenFileSource {
            relative_path,
            content,
        });
        self.get_open_file_map_mut()
            .tracked()
            .0
            .insert(relative_path, source_id);
    }

    pub fn remove_open_file(&mut self, relative_path: RelativePathToSourceFile) -> bool {
        self.get_open_file_map_mut()
            .tracked()
            .0
            .remove(&relative_path)
            .map(|source_id| self.remove(source_id))
            .is_some()
    }

    pub fn print_location_fn<'a>(&'a self) -> PrintLocationFn<'a> {
        (move |location: Location, f: &mut std::fmt::Formatter<'_>| match location {
            Location::Embedded(embedded_location) => {
                let read_out_text = match file_text_at_span_at_location(
                    self,
                    embedded_location.text_source.relative_path_to_source_file,
                    // The TextSource span
                    embedded_location.text_source.span,
                ) {
                    Some(text) => text,
                    None => {
                        return write!(
                            f,
                            "\nERROR: File not found. This is indicative of a bug in Isograph."
                        );
                    }
                };

                // The inner span
                let text_with_carats = text_with_carats(&read_out_text, embedded_location.span);

                let file_path = embedded_location.text_source.relative_path_to_source_file;
                write!(f, "{file_path}\n{text_with_carats}")
            }
            Location::Generated => write!(f, "\n<generated>"),
        })
        .boxed()
    }
}

// TODO can we return &'db str?
#[memo]
pub fn file_text_at_span_at_location<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    relative_path: RelativePathToSourceFile,
    // Note: we pass the span as well, so that if unrelated text changes (e.g. surrounding JavaScript,
    // or the iso literal is moved around), then we can recalculate this fn but short circuit the surrounding
    // fn.
    span: Option<Span>,
) -> Option<String> {
    let file_content = match file_text_at_location(db, relative_path) {
        Some(s) => s,
        None => return None,
    };

    match span {
        Some(span) => file_content[span.as_usize_range()].to_string().wrap_some(),
        None => file_content.clone().wrap_some(),
    }
}

#[memo]
fn file_text_at_location<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    relative_path: RelativePathToSourceFile,
) -> Option<String> {
    // TODO do something smarter

    // First, check if its open
    if let Some(text) = db.get_open_file(relative_path) {
        return db.get(text).content.clone().wrap_some();
    }

    // Check if it's an iso literal
    if let Some(iso_literal) = db.get_iso_literal_map().tracked().0.get(&relative_path) {
        return db
            .get(iso_literal.dereference())
            .content
            .clone()
            .wrap_some();
    }

    let standard_sources = db.get_standard_sources();
    let standard_sources = standard_sources.tracked();
    let config = db.get_isograph_config();

    // Then schema source
    if relative_path == config.schema.relative_path {
        return db
            .get(standard_sources.schema_source_id)
            .content
            .clone()
            .wrap_some();
    }

    // Is it a schema extension? Maybe! Try that.
    return db
        .standard_sources
        .schema_extension_sources
        .get(&relative_path)
        .map(|extension_source| db.get(extension_source.dereference()).content.clone());
}
