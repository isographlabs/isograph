use std::collections::BTreeSet;

use common_lang_types::RelativePathToSourceFile;
use graphql_schema_parser::{
    DefinitionError, ParsedSchema, TypeSystemDefinitionOrExtension, parse_schema,
};
use isograph_compiler::{ClientId, CompilationProfile, IsographDb, file_contents};
use pico_macros::memo;
use prelude::Postfix;
use span::WithSpan;

/// Every schema file `client` reads: disk paths and this client's editor buffer paths, sorted.
#[memo]
pub fn schema_file_paths<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let view = db.get_schema_path_map();
    let map = view.tracked();
    map.disk
        .iter()
        .copied()
        .chain(
            map.editor_buffers
                .iter()
                .filter(|key| key.client == client)
                .map(|key| key.path),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The parse of one schema file as `client` reads it; `None` when the file has no text.
#[memo]
pub fn parsed_schema_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<ParsedSchema> {
    file_contents(db, client, path).as_deref().map(parse_schema)
}

/// One top-level entry of a schema file.
pub(crate) struct SchemaEntry<'db> {
    pub path: RelativePathToSourceFile,
    pub entry: &'db Result<WithSpan<TypeSystemDefinitionOrExtension>, WithSpan<DefinitionError>>,
}

/// Every parsed entry of every schema file `client` reads, with its file.
pub(crate) fn schema_entries<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> impl Iterator<Item = SchemaEntry<'_>> {
    schema_file_paths(db, client).iter().flat_map(move |path| {
        parsed_schema_file(db, client, path.dereference())
            .iter()
            .flat_map(|parsed| parsed.definitions.iter())
            .map(move |entry| SchemaEntry {
                path: path.dereference(),
                entry,
            })
    })
}
