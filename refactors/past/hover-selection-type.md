# Hover a selection with its type

Hover on a selection name shows the selectable it selects and that selectable's target entity, `name: TargetEntity`, in Markdown, with the existing range (the char under the cursor). The selection's parent entity comes from walking up the selection path: a selection set under a selectable declaration selects on that declaration's parent type; a selection set under a selection selects on that selection's selectable's target entity.

The schema lookups are memos that take the client, because a schema file can be open and edited in the editor. Until `NetworkProtocol::parse_schema` exists they are stubs that return an error diagnostic, so every selection-name hover is an expected error and returns no hover. Every other resolved node keeps the variant-name hover.

Origin: isograph `crates/isograph_schema/src/parent_and_selectable_for_path.rs` (`get_parent_and_selectable_for_scalar_path`, `get_parent_and_selectable_for_object_path`, `get_parent_and_selectable_for_selection_parent`, `parent_object_entity_and_selectable`), `crates/isograph_schema/src/new_data_model/data_model_access.rs` (`flattened_entity_named`), `crates/isograph_schema/src/memoized/selectable_access.rs` (`selectable_named`), `crates/isograph_lsp/src/hover.rs` (on `Err`, no hover).

## Walk

```rust
// from crates/isograph_compiler/src/parent_and_selectable_for_path.rs
use common_lang_types::{DiagnosticResult, EntityName, SelectableName};
use isograph_parser::{SelectionPath, SelectionSetParent, SelectionSetPath};
use prelude::Postfix;

use crate::{
    ClientId, CompilationProfile, Entity, IsographDb, Selectable, entity_named, selectable_named,
};

/// The entity `selection_path` selects on and the selectable it selects, as `client` reads
/// the schema.
pub fn get_parent_and_selectable_for_selection_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_path: &SelectionPath<'_>,
) -> DiagnosticResult<(Entity, Selectable)> {
    get_parent_and_selectable_for_selection_parent(
        db,
        client,
        selection_path.parent.parent.reference(),
        selection_path.inner.name.item.0.unchecked_conversion(),
    )
}

/// The entity the selection set at `selection_set_path` selects on and its selectable named
/// `selectable_name`.
pub fn get_parent_and_selectable_for_selection_parent<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_set_path: &SelectionSetPath<'_>,
    selectable_name: SelectableName,
) -> DiagnosticResult<(Entity, Selectable)> {
    match selection_set_path.parent.reference() {
        SelectionSetParent::SelectableDeclaration(selectable_declaration_path) => {
            parent_object_entity_and_selectable(
                db,
                client,
                selectable_declaration_path.inner.parent_type.item.0,
                selectable_name,
            )
        }
        SelectionSetParent::Selection(selection_path) => {
            let (_, selectable) =
                get_parent_and_selectable_for_selection_path(db, client, selection_path)?;
            parent_object_entity_and_selectable(
                db,
                client,
                selectable.target_entity,
                selectable_name,
            )
        }
    }
}

/// The entity named `parent_entity_name` and its selectable named `selectable_name`.
pub fn parent_object_entity_and_selectable<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    parent_entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<(Entity, Selectable)> {
    let entity = entity_named(db, client, parent_entity_name).clone()?;
    let selectable = selectable_named(db, client, parent_entity_name, selectable_name).clone()?;
    (entity, selectable).wrap_ok()
}
```

Delta from upstream:

- One selection type in i2, so `get_parent_and_selectable_for_scalar_path` and `get_parent_and_selectable_for_object_path` are one function, `get_parent_and_selectable_for_selection_path`. It does no scalar/object check: i2 has one kind of entity and one kind of selectable (mental-model.md).
- Every function takes `client: ClientId` and passes it down.
- `SelectionSetParent` has two variants (`SelectableDeclaration`, `Selection`) in place of upstream's three (`ObjectSelection`, `ClientFieldDeclaration`, `ClientPointerDeclaration`).
- The selection's set is `selection_path.parent.parent`: `SelectionPath`'s parent is `SelectionSlotPath`, whose parent is `SelectionSetPath`. Upstream matches `SelectionParentType::SelectionSet`.
- The lookup key is `selection_path.inner.name.item.0` (`SelectionName`) converted with `unchecked_conversion` to `SelectableName`. A reader alias does not change the key: the walk reads `inner.name`, so hover on the alias `b` of `b: bar` looks up `bar`.
- `parent_object_entity_and_selectable` takes `EntityName`, not `EntityNameWrapper`. The `Selection` arm has the target entity name from the schema, not a parsed name; wrapping it in a parser wrapper would build a parse node from schema data.
- No `Location` threading and no `selectable_is_not_defined_diagnostic`. The lookup memos return the diagnostic.
- `get_parent_for_selection_set_path` is not translated. Hover on a selection set keeps the variant-name hover, and the walk does not call it.
- `parent_object_entity_and_selectable` calls `entity_named` before `selectable_named`, so a missing entity is the error, as upstream.

## Schema types and lookups

```rust
// from crates/isograph_compiler/src/schema.rs
use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, SelectableName};
use pico_macros::memo;
use prelude::Postfix;

use crate::{ClientId, CompilationProfile, IsographDb};

/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub name: EntityName,
}

/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub target_entity: EntityName,
}

/// The entity named `entity_name` in the schema as `client` reads it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    _client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Entity> {
    Diagnostic::new(
        format!("The schema is not available yet, so entity `{entity_name}` cannot be looked up."),
        None,
    )
    .wrap_err()
}

/// The selectable named `selectable_name` on the entity named `entity_name` in the schema as
/// `client` reads it.
#[memo]
pub fn selectable_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    _client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    Diagnostic::new(
        format!(
            "The schema is not available yet, so selectable \
            `{entity_name}.{selectable_name}` cannot be looked up."
        ),
        None,
    )
    .wrap_err()
}
```

`Entity` and `Selectable` are the minimal shapes of mental-model.md's `Entity` and `Selectable`: one kind of each. `Entity` keeps a named field because `selectables` will join it. `Selectable.target_entity` is a name, resolved by `entity_named` on demand; it becomes a `Wrapper` when wrappers exist. Hover reads `Selectable.name` and `Selectable.target_entity`. `Entity.name` has no reader in this change; the walk returns the parent `Entity` as upstream does. The stubs construct neither type.

The memos return `DiagnosticResult` (`common_lang_types::diagnostic`), as upstream `selectable_named` does. Upstream `flattened_entity_named` returns `Option`; i2 returns the diagnostic so the stub can say why. `Location` is `None`: the lookup has no source position. The client parameter is `_client` while the stub bodies do not read it; it becomes `client` when the bodies read the schema source. Both get real bodies with `NetworkProtocol::parse_schema`.

```rust
// from crates/isograph_compiler/src/lib.rs
mod database;
mod host_language;
mod iso_literals;
mod parent_and_selectable_for_path;
mod schema;
#[cfg(test)]
mod test_profile;

pub use database::{ClientId, DiskFile, DiskFileMap, IsographDb, file_contents};
pub use host_language::*;
pub use iso_literals::{
    LiteralId, extract_iso_literals, iso_literal_extraction, iso_literal_parse_errors_in_file,
    parsed_iso_literal, parsed_iso_literal_by_id, parsed_iso_literals_in_file,
    text_through_last_iso_literal,
};
pub use parent_and_selectable_for_path::{
    get_parent_and_selectable_for_selection_parent, get_parent_and_selectable_for_selection_path,
    parent_object_entity_and_selectable,
};
pub use schema::{Entity, Selectable, entity_named, selectable_named};
```

## Hover

Before:

```rust
// from crates/isograph_lsp/src/hover.rs
pub fn hover_for_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Hover> {
    let (literal_id, node, cursor) = resolved_position_at_lsp_position(db, client, path, position)?;
    let span = file_span_of_literal_span(db, literal_id, cursor)?;
    lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: node.to_string(),
        }),
        range: line_index(db, client, path)
            .as_ref()?
            .range(span)
            .wrap_some(),
    }
    .wrap_some()
}
```

After:

```rust
// from crates/isograph_lsp/src/hover.rs
pub fn hover_for_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Hover> {
    let (literal_id, node, cursor) = resolved_position_at_lsp_position(db, client, path, position)?;
    let value = hover_markdown(db, client, node)?;
    let span = file_span_of_literal_span(db, literal_id, cursor)?;
    lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value,
        }),
        range: line_index(db, client, path)
            .as_ref()?
            .range(span)
            .wrap_some(),
    }
    .wrap_some()
}

/// The Markdown hover text of `node` as `client` reads the schema. A selection name whose
/// selectable is not found has none.
fn hover_markdown<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: IsographResolutionNode<'_>,
) -> Option<String> {
    match node {
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => {
            selection_hover_markdown(db, client, selection_name_path.parent.reference())
        }
        other => other.to_string().wrap_some(),
    }
}

/// `name: TargetEntity` for the selectable `selection_path` selects.
fn selection_hover_markdown<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_path: &SelectionPath<'_>,
) -> Option<String> {
    let (_, selectable) =
        get_parent_and_selectable_for_selection_path(db, client, selection_path).ok()?;
    format!("{}: {}", selectable.name, selectable.target_entity).wrap_some()
}
```

The `other` arm binds `IsographResolutionNode`, which still includes `SelectionNameWrapper`. `IsographResolutionNode` is `#[non_exhaustive]`, so a match outside `isograph_parser` must have a wildcard arm; listing every variant does not remove it. The arm only formats the variant name.

`.ok()?` drops the diagnostic: an error from the lookup is an expected error and returns no hover, as upstream `on_hover_impl` does with `if let Ok(..) = get_parent_and_selectable_for_scalar_path(..)`.

## End-user experience

Until the schema exists, hover on a selection name (`id` in `field Pet.fullName { id }`, `owner` and `name` in `{ owner { name } }`, `b` and `bar` in `{ b: bar }`) shows nothing. Hover on `Pet` shows `EntityNameWrapper`, on `fullName` shows `SelectableNameWrapper`, on a comma between selections shows `SelectionSet`, as today. When the schema has `Pet.owner -> User` and `User.name -> String`, hover on `name` in `field Pet.fullName { owner { name } }` shows `name: String`.

## Test profile in isograph_compiler

The `TestProfile` in `database.rs`'s test module moves verbatim (with `TestHostError`, `TestHostLanguage`, `TestNetworkProtocol`) to `crates/isograph_compiler/src/test_profile.rs` as `pub(crate)` items under `#[cfg(test)] mod test_profile;`, so `parent_and_selectable_for_path.rs` tests use it. Delta: `pub(crate)` on the four items. `database.rs` tests import `crate::test_profile::TestProfile`. This mirrors `crates/isograph_lsp/src/test_profile.rs`.

```rust
// from crates/isograph_compiler/src/test_profile.rs
use std::path::Path;

use thiserror::Error;

use crate::host_language::{
    CompilationProfile, HostLanguage, IsoLiteralExtraction, NetworkProtocol, SkipSourceFile,
};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("test host")]
pub(crate) struct TestHostError;

pub(crate) struct TestHostLanguage;

impl HostLanguage for TestHostLanguage {
    type Error = TestHostError;
    type LiteralContext = ();

    fn extract_iso_literals_from_text(_contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        Vec::new()
    }

    fn should_skip_source_file(_relative_path: &Path) -> SkipSourceFile {
        SkipSourceFile::Keep
    }
}

pub(crate) struct TestNetworkProtocol;

impl NetworkProtocol for TestNetworkProtocol {}

pub(crate) struct TestProfile;

impl CompilationProfile for TestProfile {
    type HostLanguage = TestHostLanguage;
    type NetworkProtocol = TestNetworkProtocol;
}
```

## Tests

`crates/isograph_compiler/src/parent_and_selectable_for_path.rs`: parse a literal with `parse_iso_literal`, resolve the first char of a selection name with `resolve_offset`, take the `SelectionNameWrapper` path's parent, and call `get_parent_and_selectable_for_selection_path`. Each result's `err()` equals `entity_named(&db, client, Pet).clone().err()`:

- `field Pet.fullName { id }` at `id` (top-level selection).
- `field Pet.fullName { owner { name } }` at `name` (nested: the recursion reaches `owner` first, and `entity_named(Pet)` fails there).
- `field Pet.fullName { b: bar }` at `b` and at `bar` (alias).

`crates/isograph_cli/src/state.rs`, through `handle` and a `textDocument/hover` request:

- New `hover_on_a_selection_name_is_null_until_the_schema_exists`: `id` in `FIELD_X`, `id` and `name` in `PET_ONE_LINE`, `owner` and `name` in `iso(\`field Pet.fullName { owner { name } }\`)`, and `b` and `bar` in `iso(\`field Pet.fullName { b: bar }\`)` hover `None`. In that nested literal `Pet` hovers `EntityNameWrapper` and `fullName` hovers `SelectableNameWrapper`.
- `hover_reads_the_clients_buffer_and_disk_after_did_close`, `hover_in_two_literals_on_two_lines_resolves_in_each`, `hover_in_two_literals_on_one_line_resolves_in_each` assert `some("SelectionNameWrapper")` at `id` of `FIELD_X`. They move to `x{` of `FIELD_X` and assert `some("SelectableNameWrapper")`, so each still distinguishes the two readers or the two literals.
