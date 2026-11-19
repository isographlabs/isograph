# Schema hover and go-to-definition

Hover and `textDocument/definition` look up the schema item a position names, for three positions:

- an entity name (`EntityNameWrapper`): `Pet` in `field Pet.x`, `Query` in `entrypoint Query.X`, `ID` in `$id: ID!`, `User` in `to User`. The lookup is `entity_named`.
- a declared selectable name (`SelectableNameWrapper`): `x` in `field Pet.x`, `X` in `entrypoint Query.X`. The lookup is `selectable_named` on the declaration's parent type.
- a selection name (`SelectionNameWrapper`): `id` in `field Pet.x { id }`, including a reader alias. The lookup is the selection walk (`refactors/past/hover-selection-type.md`).

Hover shows the entity or selectable. Go-to-definition jumps to its definition. Hover and go-to-definition call one function in `isograph_lsp` that maps a resolved node to the looked-up item. A failed lookup is an expected error: hover shows nothing and go-to-definition returns `null`. Every other resolved node keeps the variant-name hover and has no definition.

`entity_named` and `selectable_named` are still stubs that return an error diagnostic. Until `NetworkProtocol::parse_schema` exists, all three positions hover nothing and go-to-definition is always `null`.

Origin: isograph `crates/isograph_lsp/src/hover.rs` (`on_hover_impl`, `hover_text_for_selectable`, `format_hover_for_entity`), `crates/isograph_lsp/src/goto_definition.rs` (`on_goto_definition_impl`, `goto_entity_definition`, `lsp_location_to_scalar_response`).

## Schema types

Before:

```rust
// from crates/isograph_compiler/src/schema.rs
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
```

After:

```rust
// from crates/isograph_compiler/src/schema.rs
/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub name: EntityName,
    pub definition: DefinitionLocation,
}

/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub target_entity: EntityName,
    pub definition: DefinitionLocation,
}

/// Where an entity or selectable is defined: a span of a file, in that file's byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub path: RelativePathToSourceFile,
    pub span: Span,
}
```

`DefinitionLocation` is one struct for an item from the schema file and an item from an iso literal. Both are a path and a file-absolute span, so the reader needs no `LiteralId`. `DescriptionValue` and `RelativePathToSourceFile` are `common_lang_types`; `Span` is `span::Span`.

Readers: entity hover reads `Entity.name`. Selectable hover reads `name`, `parent_entity`, `target_entity`, `description`. Go-to-definition reads `definition` of both. `Entity` has no description: nothing supplies one yet.

`schema.rs` imports become:

```rust
// from crates/isograph_compiler/src/schema.rs
use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EntityName, RelativePathToSourceFile,
    SelectableName,
};
use pico_macros::memo;
use prelude::Postfix;
use span::Span;
```

The stubs `entity_named` and `selectable_named` are unchanged.

## Walk returns the selectable

The walk returns `DiagnosticResult<Selectable>` in place of `DiagnosticResult<(Entity, Selectable)>`; no caller reads the parent `Entity`. `entity_named` stays and returns `Entity`. The walk calls it with `?`, so a missing parent entity is the error before the selectable lookup, as upstream. The file `parent_and_selectable_for_path.rs` is renamed `selectable_for_path.rs`.

Renames (upstream name in parentheses):

- `get_parent_and_selectable_for_selection_path` -> `selectable_for_selection_path` (`get_parent_and_selectable_for_scalar_path`, `get_parent_and_selectable_for_object_path`)
- `get_parent_and_selectable_for_selection_parent` -> `selectable_for_selection_parent` (`get_parent_and_selectable_for_selection_parent`)
- `parent_object_entity_and_selectable` -> `selectable_for_entity` (`parent_object_entity_and_selectable`)

New: `selectable_for_selectable_name_path`, for the name of an entrypoint or selectable declaration. Upstream has no equivalent; its hover returns `None` on `ClientScalarSelectableNameWrapper`.

Before:

```rust
// from crates/isograph_compiler/src/parent_and_selectable_for_path.rs
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

After:

```rust
// from crates/isograph_compiler/src/selectable_for_path.rs
use common_lang_types::{DiagnosticResult, EntityName, SelectableName};
use isograph_parser::{
    SelectableNameWrapperParent, SelectableNameWrapperPath, SelectionPath, SelectionSetParent,
    SelectionSetPath,
};
use prelude::Postfix;

use crate::{ClientId, CompilationProfile, IsographDb, Selectable, entity_named, selectable_named};

/// The selectable `selection_path` selects, as `client` reads the schema.
pub fn selectable_for_selection_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_path: &SelectionPath<'_>,
) -> DiagnosticResult<Selectable> {
    selectable_for_selection_parent(
        db,
        client,
        selection_path.parent.parent.reference(),
        selection_path.inner.name.item.0.unchecked_conversion(),
    )
}

/// The selectable named `selectable_name` on the entity the selection set at
/// `selection_set_path` selects on.
pub fn selectable_for_selection_parent<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_set_path: &SelectionSetPath<'_>,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    match selection_set_path.parent.reference() {
        SelectionSetParent::SelectableDeclaration(selectable_declaration_path) => {
            selectable_for_entity(
                db,
                client,
                selectable_declaration_path.inner.parent_type.item.0,
                selectable_name,
            )
        }
        SelectionSetParent::Selection(selection_path) => {
            let parent_selectable = selectable_for_selection_path(db, client, selection_path)?;
            selectable_for_entity(
                db,
                client,
                parent_selectable.target_entity,
                selectable_name,
            )
        }
    }
}

/// The selectable named `selectable_name` on the entity named `entity_name`.
pub fn selectable_for_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    entity_named(db, client, entity_name).clone()?;
    selectable_named(db, client, entity_name, selectable_name).clone()
}

/// The selectable that the entrypoint or selectable declaration at the parent of
/// `selectable_name_path` names.
pub fn selectable_for_selectable_name_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selectable_name_path: &SelectableNameWrapperPath<'_>,
) -> DiagnosticResult<Selectable> {
    let parent_type = match selectable_name_path.parent.reference() {
        SelectableNameWrapperParent::EntrypointDeclaration(entrypoint_declaration_path) => {
            entrypoint_declaration_path.inner.parent_type.item
        }
        SelectableNameWrapperParent::SelectableDeclaration(selectable_declaration_path) => {
            selectable_declaration_path.inner.parent_type.item
        }
    };
    selectable_named(db, client, parent_type.0, selectable_name_path.inner.0).clone()
}
```

`selectable_for_selectable_name_path` calls `selectable_named` directly: the declaration's parent type is the entity, and `selectable_named` fails when that entity does not exist.

```rust
// from crates/isograph_compiler/src/lib.rs
mod database;
mod host_language;
mod iso_literals;
mod schema;
mod selectable_for_path;
#[cfg(test)]
mod test_profile;

// ...
pub use schema::{DefinitionLocation, Entity, Selectable, entity_named, selectable_named};
pub use selectable_for_path::{
    selectable_for_entity, selectable_for_selectable_name_path, selectable_for_selection_parent,
    selectable_for_selection_path,
};
```

The unit tests move with the file and call `selectable_for_selection_path`; their assertions (`err()` equals `entity_named(Pet)`'s error) are unchanged. A new unit test resolves `x` in `field Pet.x { id }` and `X` in `entrypoint Query.X` and asserts `selectable_for_selectable_name_path(..).err()` equals `selectable_named(Pet, x)`'s and `selectable_named(Query, X)`'s error.

## The looked-up item

```rust
// from crates/isograph_lsp/src/entity_or_selectable.rs
use common_lang_types::DiagnosticResult;
use isograph_compiler::{
    ClientId, CompilationProfile, DefinitionLocation, Entity, IsographDb, Selectable,
    entity_named, selectable_for_selectable_name_path, selectable_for_selection_path,
};
use isograph_parser::IsographResolutionNode;
use prelude::Postfix;

/// The schema item a resolved position names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityOrSelectable {
    Entity(Entity),
    Selectable(Selectable),
}

impl EntityOrSelectable {
    pub fn definition(&self) -> DefinitionLocation {
        match self {
            EntityOrSelectable::Entity(entity) => entity.definition,
            EntityOrSelectable::Selectable(selectable) => selectable.definition,
        }
    }
}

/// The lookup of the schema item `node` names, as `client` reads the schema. `None` when
/// `node` names no schema item.
pub(crate) fn entity_or_selectable_of_node<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: &IsographResolutionNode<'_>,
) -> Option<DiagnosticResult<EntityOrSelectable>> {
    match node {
        IsographResolutionNode::EntityNameWrapper(entity_name_path) => {
            entity_named(db, client, entity_name_path.inner.0)
                .clone()
                .map(EntityOrSelectable::Entity)
                .wrap_some()
        }
        IsographResolutionNode::SelectableNameWrapper(selectable_name_path) => {
            selectable_for_selectable_name_path(db, client, selectable_name_path)
                .map(EntityOrSelectable::Selectable)
                .wrap_some()
        }
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => {
            selectable_for_selection_path(db, client, selection_name_path.parent.reference())
                .map(EntityOrSelectable::Selectable)
                .wrap_some()
        }
        _ => None,
    }
}
```

`Option<DiagnosticResult<_>>` keeps three outcomes apart: `None` (the node names no schema item; hover shows the variant name), `Some(Err)` (the lookup failed; no hover, no definition), `Some(Ok)` (the item). The `_` arm is required while `IsographResolutionNode` is `#[non_exhaustive]` and this match is outside `isograph_parser`. A separate parser change removes `#[non_exhaustive]`; this match becomes exhaustive after it lands. `EntityOrSelectable` lives in `isograph_lsp` because only hover and go-to-definition read it.

```rust
// from crates/isograph_lsp/src/lib.rs
mod definition;
mod document_highlight;
mod entity_or_selectable;
// ...
pub use definition::definition_at_lsp_position;
pub use entity_or_selectable::EntityOrSelectable;
```

`resolved_position_at_lsp_position` in `hover.rs` becomes `pub(crate)`; its body is unchanged.

## Hover

Before (from `refactors/past/hover-selection-type.md`):

```rust
// from crates/isograph_lsp/src/hover.rs
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

After:

```rust
// from crates/isograph_lsp/src/hover.rs
/// The Markdown hover text of `node` as `client` reads the schema. A schema name whose
/// lookup fails has none.
fn hover_markdown<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: IsographResolutionNode<'_>,
) -> Option<String> {
    match entity_or_selectable_of_node(db, client, node.reference()) {
        Some(lookup) => lookup
            .ok()
            .map(|item| entity_or_selectable_hover_markdown(item.reference())),
        None => node.to_string().wrap_some(),
    }
}

fn entity_or_selectable_hover_markdown(item: &EntityOrSelectable) -> String {
    match item {
        EntityOrSelectable::Entity(entity) => entity_hover_markdown(entity),
        EntityOrSelectable::Selectable(selectable) => selectable_hover_markdown(selectable),
    }
}

fn entity_hover_markdown(entity: &Entity) -> String {
    format!("entity **{}**", entity.name)
}

fn selectable_hover_markdown(selectable: &Selectable) -> String {
    let description = selectable
        .description
        .map(|description| format!("\n\n{description}"))
        .unwrap_or_default();
    format!(
        "field **{}.{}**: **{}**{description}",
        selectable.parent_entity, selectable.name, selectable.target_entity
    )
}
```

`.ok()` drops the diagnostic: a failed lookup is an expected error, as upstream does with `if let Ok(..) = get_parent_and_selectable_for_scalar_path(..)` and `.ok_or(LSPRuntimeError::ExpectedError)?` on `flattened_entity_named`. `selection_hover_markdown` and the `SelectionPath` and `get_parent_and_selectable_for_selection_path` imports are removed.

Delta from upstream `format_hover_for_entity`: `entity` in place of `Object` (one kind of entity), no description. Delta from upstream `hover_text_for_selectable`: no `server_or_client` prefix (one kind of selectable), no arguments, no parent description. The target entity follows the name; the description follows a blank line when present.

## Go-to-definition

```rust
// from crates/isograph_lsp/src/definition.rs
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
```

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn definition_response<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    params: lsp_types::GotoDefinitionParams,
) -> LSPRuntimeResult<<lsp_types::request::GotoDefinition as lsp_types::request::Request>::Result>
{
    let position = params.text_document_position_params;
    let path = relative_from_uri(db, position.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::definition_at_lsp_position(db, client, path, position.position)
        .map(|definition| {
            definition_lsp_location(db, client, definition)
                .map(lsp_types::GotoDefinitionResponse::Scalar)
        })
        .transpose()
}

/// The LSP location of `definition`, in the line index of the defining file as `client`
/// reads it.
fn definition_lsp_location<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    definition: DefinitionLocation,
) -> LSPRuntimeResult<lsp_types::Location> {
    let uri = uri_for_path(db, definition.path)
        .map_err(|error| LSPRuntimeError::UnexpectedError(error.to_string()))?;
    let line_index = isograph_lsp::line_index(db, client, definition.path)
        .as_ref()
        .ok_or_else(|| {
            LSPRuntimeError::UnexpectedError(format!(
                "the defining file `{}` has no text",
                definition.path
            ))
        })?;
    lsp_types::Location {
        uri,
        range: line_index.range(definition.span),
    }
    .wrap_ok()
}
```

The range comes from the defining file's `line_index`, not the requesting file's. Every `PathUriError` (`MissingWorkingDirectory`, `NotAbsolute`, `InvalidUri`) is a broken invariant, so it is `UnexpectedError`. A `definition.path` with no text is also `UnexpectedError`: the schema that produced the definition read that file through `file_contents`, so the same client reads text there.

Registration, after `HoverRequest` in `dispatch_lsp_request`:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::GotoDefinition>(
                crate::adapter::definition_response::<TProfile>,
            )?
```

Capability, after `hover_provider`:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        definition_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

`ServerCapabilities::definition_provider` is `Option<OneOf<bool, DefinitionOptions>>` in lsp-types 0.97.0.

## End-user experience

Until the schema exists: hover on `Pet`, `x`, `id`, `Query`, `X`, or `ID` in `field Pet.x($id: ID!) { id }` and `entrypoint Query.X` shows nothing, and go-to-definition on any of them does nothing. Hover on `field`, `entrypoint`, `.`, `{`, or a comma shows the variant name as today; go-to-definition there does nothing.

When the schema has `Pet` defined on line 1 of `schema.graphql` and `Pet.owner -> User` described as `"The owner"` on line 2: hover on `Pet` shows `entity **Pet**`. Hover on `owner` in `field Pet.x { owner }` shows

```text
field **Pet.owner**: **User**

The owner
```

Go-to-definition on `Pet` or `owner` opens `schema.graphql` at that name, using the client's open buffer of `schema.graphql` when it has one.

## Tests

- `crates/isograph_lsp/src/hover.rs`: `entity_hover_markdown` of an `Entity` built in the test is `"entity **Pet**"`. `selectable_hover_markdown` of a `Selectable` built in the test is `"field **Pet.owner**: **User**\n\nThe owner"` with a description and `"field **Pet.owner**: **User**"` without.
- `crates/isograph_lsp/src/entity_or_selectable.rs`: `EntityOrSelectable::definition` of an `Entity` and of a `Selectable` built in the test with different `DefinitionLocation`s is each one's own.
- `crates/isograph_cli/src/adapter.rs`, `definition_lsp_location` of a `DefinitionLocation` built in the test:
  - Disk file `src/schema.graphql` is `"type Pet {\n  owner: User\n}\n"`; the span of `owner` maps to URI `file:///tmp/proj/src/schema.graphql` and range line 1, characters 2 to 7.
  - With client 1's open buffer `"\ntype Pet {\n  owner: User\n}\n"` over that disk file, client 1 gets line 2 and client 2 gets line 1.
  - A path with no file is `UnexpectedError`.
- `crates/isograph_cli/src/state.rs`, through `handle`:
  - `hover_on_a_schema_name_is_null_until_the_schema_exists`: hover `None` at `Pet`, `x`, `ID`, `id` of `iso(\`field Pet.x($id: ID!) { id }\`)`, at `Query` and `X` of `iso(\`entrypoint Query.X\`)`, at `owner` and `name` of `iso(\`field Pet.fullName { owner { name } }\`)`, and at `b` and `bar` of `iso(\`field Pet.fullName { b: bar }\`)`. In those literals `field` hovers `SelectableDeclaration` and `entrypoint` hovers `EntrypointDeclaration`.
  - `definition_is_null_until_the_schema_exists`: `textDocument/definition` is `null` at each of those positions and at `field`, `entrypoint`, and `{`.
  - Existing hover tests that hovered `Query`, `HomeRoute`, `x`, or `id` to assert a variant name or a range move to `entrypoint`, `field`, `.`, or `{`, keeping what each test distinguishes (buffer versus disk, before versus after didChange, one literal versus the other, the UTF-16 column).
- `crates/isograph_cli/src/lsp_socket.rs`: `initialize_advertises_formatting_will_save_wait_until_hover_and_highlight` becomes `initialize_advertises_formatting_will_save_wait_until_hover_highlight_and_definition` and asserts `definitionProvider` is `true`.
- `crates/isograph_lsp/src/hover.rs` existing range tests that hover `Query` or the last char of `HomeRoute` move to positions that still hover (the `entrypoint` keyword, `.`, `}`).
