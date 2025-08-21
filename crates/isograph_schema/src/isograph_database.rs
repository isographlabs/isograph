use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

use common_lang_types::{CurrentWorkingDirectory, RelativePathToSourceFile, TextSource};
use isograph_config::CompilerConfig;
use pico::{Database, SourceId, Storage};
use pico_macros::{memo, Db, Singleton, Source};

use crate::NetworkProtocol;

#[derive(Default, Debug, Db)]
pub struct IsographDatabase<TNetworkProtocol: NetworkProtocol> {
    pub storage: Storage<Self>,
    phantom_data: PhantomData<TNetworkProtocol>,
}

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

#[derive(Singleton, Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Singleton, PartialEq, Eq)]
pub struct StandardSources {
    // TODO we should just store this as a singleton, and not have it in standard sources.
    // Or perhaps, we store the schema source directly here.
    pub schema_source_id: SourceId<SchemaSource>,
    pub schema_extension_sources: BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
}

impl<TNetworkProtocol: NetworkProtocol + 'static> IsographDatabase<TNetworkProtocol> {
    pub fn get_current_working_directory(&self) -> CurrentWorkingDirectory {
        *self
            .get_singleton::<CurrentWorkingDirectory>()
            .expect("Expected CurrentWorkingDirectory to have been set")
    }

    pub fn get_isograph_config(&self) -> &CompilerConfig {
        self.get_singleton::<CompilerConfig>()
            .expect("Expected CompilerConfig to have been set")
    }

    pub fn get_standard_sources(&self) -> &StandardSources {
        self.get_singleton::<StandardSources>()
            .expect("Expected StandardSources to have been set")
    }

    pub fn get_open_file_map(&self) -> &OpenFileMap {
        self.get_singleton::<OpenFileMap>()
            .expect("Expected OpenFileMap to have been set")
    }
}

#[memo]
pub fn get_open_file<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    file: RelativePathToSourceFile,
) -> Option<SourceId<OpenFileSource>> {
    let file_map = db.get_open_file_map();
    file_map.0.get(&file).cloned()
}
