use std::collections::HashMap;

use common_lang_types::{RelativePathToSourceFile, TextSource};
use pico::SourceId;
use pico_macros::{Singleton, Source};

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
