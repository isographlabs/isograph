use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{ClientId, CompilationProfile, DefinitionLocation, IsographDb};
use prelude::Postfix;

use crate::entity_or_selectable::entity_or_selectable_of_node;
use crate::hover::resolved_position_at_lsp_position;

/// Where the schema item named at `position` in `path` is defined, as `client` reads it.
pub fn definition_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<DefinitionLocation> {
    let (_, node, _) = resolved_position_at_lsp_position(db, client, path, position)?;
    let item = entity_or_selectable_of_node(db, client, node.reference())?.ok()?;
    item.definition().wrap_some()
}
