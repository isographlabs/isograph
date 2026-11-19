use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{
    ClientId, CompilationProfile, DefinitionLocation, IsographDb, LiteralId, ReferenceTarget,
    parsed_iso_literal_by_id, references_in_file, source_file_paths,
};
use isograph_parser::resolve_offset;
use prelude::Postfix;
use span::Span;

use crate::entity_or_selectable::{ProfileEntityOrSelectable, entity_or_selectable_of_node};
use crate::hover::iso_literal_at_lsp_position;
use crate::line_index::line_index;

/// What a position inside an iso literal names, for find references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferencesUnderCursor {
    Target(References),
    /// The position names no entity or selectable, or names one whose lookup fails.
    NoTarget,
}

/// The declaration of an entity or selectable, and every reference to it in the source files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct References {
    /// `None` for a selectable the protocol defines with no text.
    pub declaration: Option<DefinitionLocation>,
    pub references: Vec<ReferencesInFile>,
}

/// The ranges of the references in one source file, in that file's line index as the client
/// reads it. `ranges` is not empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferencesInFile {
    pub path: RelativePathToSourceFile,
    pub ranges: Vec<lsp_types::Range>,
}

/// The references to the item named at `position` in `path` as `client` reads it. `None` when
/// `position` is not inside an iso literal.
pub fn references_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<ReferencesUnderCursor> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    entity_or_selectable_at(db, client, literal_id, cursor)
        .map_or(ReferencesUnderCursor::NoTarget, |item| {
            ReferencesUnderCursor::Target(references_to(db, client, item.reference()))
        })
        .wrap_some()
}

/// The schema item the char at `cursor` in `literal_id` names, when its lookup succeeds.
fn entity_or_selectable_at<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    literal_id: LiteralId,
    cursor: Span,
) -> Option<ProfileEntityOrSelectable<TProfile>> {
    let parsed = parsed_iso_literal_by_id(db, literal_id)?;
    entity_or_selectable_of_node(db, client, resolve_offset(parsed, cursor)?.reference())?.ok()
}

fn references_to<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    item: &ProfileEntityOrSelectable<TProfile>,
) -> References {
    let target = item.reference_target();
    References {
        declaration: item.definition(),
        references: source_file_paths(db, client)
            .iter()
            .filter_map(|path| references_in_file_to(db, client, path.dereference(), target))
            .collect(),
    }
}

/// The references to `target` in `path`. `None` when there are none.
fn references_in_file_to<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    target: ReferenceTarget,
) -> Option<ReferencesInFile> {
    let references = references_in_file(db, client, path).as_ref()?;
    let line_index = line_index(db, client, path).as_ref()?;
    let ranges = references
        .iter()
        .filter(|reference| reference.item == target)
        .map(|reference| line_index.range(reference.location))
        .collect::<Vec<_>>();
    (!ranges.is_empty()).then_some(ReferencesInFile { path, ranges })
}
