# Selection hover and go-to-definition

Hover on a selection name shows the selectable it selects: its parent entity, its name, its target entity, and its description when it has one. `textDocument/definition` on a selection name jumps to where that selectable is defined. Both call one function in `isograph_lsp` that resolves the position and runs `get_parent_and_selectable_for_selection_path` (landed in `refactors/past/hover-selection-type.md`).

The schema lookups `entity_named` and `selectable_named` are still stubs that return an error diagnostic. Until `NetworkProtocol::parse_schema` exists, every selection-name hover returns no hover and every go-to-definition returns `null`.

Origin: isograph `crates/isograph_lsp/src/hover.rs` (`hover_text_for_selectable`), `crates/isograph_lsp/src/goto_definition.rs` (`on_goto_definition_impl`, `lsp_location_to_scalar_response`).

## Selectable

Before:

```rust
// from crates/isograph_compiler/src/schema.rs
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
/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub target_entity: EntityName,
    pub definition: DefinitionLocation,
}

/// Where a selectable is defined: a span of a file, in that file's byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub path: RelativePathToSourceFile,
    pub span: Span,
}
```

`DefinitionLocation` is one struct for a selectable from the schema file and a selectable from an iso literal. Both are a path and a file-absolute span. `span` is file-absolute, not literal-relative, so the reader needs no `LiteralId`. `DescriptionValue` and `RelativePathToSourceFile` are `common_lang_types`; `Span` is `span::Span`.

Readers: hover reads `parent_entity`, `name`, `target_entity`, `description`. Go-to-definition reads `definition`. `Entity` stays `{ name }`; hover and go-to-definition on an `EntityNameWrapper` are not part of this change.

`schema.rs` imports become `use common_lang_types::{Diagnostic, DiagnosticResult, DescriptionValue, EntityName, RelativePathToSourceFile, SelectableName};` and `use span::Span;`. `lib.rs` exports `DefinitionLocation` with `Entity` and `Selectable`:

```rust
// from crates/isograph_compiler/src/lib.rs
pub use schema::{DefinitionLocation, Entity, Selectable, entity_named, selectable_named};
```

## Shared step in isograph_lsp

```rust
// from crates/isograph_lsp/src/hover.rs
/// The selectable that the selection at `selection_name_path` selects, as `client` reads the
/// schema. `None` when the lookup fails.
pub(crate) fn selectable_of_selection_name<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_name_path: &SelectionNameWrapperPath<'_>,
) -> Option<Selectable> {
    let (_, selectable) = get_parent_and_selectable_for_selection_path(
        db,
        client,
        selection_name_path.parent.reference(),
    )
    .ok()?;
    selectable.wrap_some()
}
```

`.ok()?` drops the diagnostic: a failed lookup is an expected error, so hover returns no hover and go-to-definition returns `null`, as upstream does with `if let Ok(..) = get_parent_and_selectable_for_scalar_path(..)`.

`resolved_position_at_lsp_position` in `hover.rs` becomes `pub(crate)`; its body is unchanged.

```rust
// from crates/isograph_lsp/src/definition.rs
use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{ClientId, CompilationProfile, IsographDb, Selectable};
use isograph_parser::IsographResolutionNode;

use crate::hover::{resolved_position_at_lsp_position, selectable_of_selection_name};

/// The selectable that the selection name at `position` in `path` selects, as `client` reads
/// it.
pub fn selectable_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<Selectable> {
    let (_, node, _) = resolved_position_at_lsp_position(db, client, path, position)?;
    match node {
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => {
            selectable_of_selection_name(db, client, selection_name_path.reference())
        }
        _ => None,
    }
}
```

The `_` arm is required: `IsographResolutionNode` is `#[non_exhaustive]`, and this match is outside `isograph_parser`.

```rust
// from crates/isograph_lsp/src/lib.rs
mod definition;
// ...
pub use definition::selectable_at_lsp_position;
```

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
fn hover_markdown<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: IsographResolutionNode<'_>,
) -> Option<String> {
    match node {
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => {
            selectable_of_selection_name(db, client, selection_name_path.reference())
                .map(|selectable| selectable_hover_markdown(selectable.reference()))
        }
        other => other.to_string().wrap_some(),
    }
}

/// The Markdown hover text of `selectable`.
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

Delta from upstream `hover_text_for_selectable`: no `server_or_client` prefix (one kind of selectable), no arguments, no parent description (`Entity` has none). The target entity follows the name. The description follows a blank line when present, as upstream `format_hover_for_entity` does.

## Go-to-definition

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
    isograph_lsp::selectable_at_lsp_position(db, client, path, position.position)
        .map(|selectable| {
            definition_lsp_location(db, client, selectable.definition)
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

The range comes from the defining file's `line_index`, not the hovered file's. Every `PathUriError` (`MissingWorkingDirectory`, `NotAbsolute`, `InvalidUri`) is a broken invariant, so it is `UnexpectedError`. A `definition.path` with no text is also `UnexpectedError`: the schema that produced the definition read that file through `file_contents`, so the same client reads text there. `definition_lsp_location` takes `DefinitionLocation`, not `Selectable`, because it reads only `definition`.

Registration, after `HoverRequest` in `dispatch_lsp_request`:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::GotoDefinition>(
                crate::adapter::definition_response::<TProfile>,
            )?
```

Capability:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        hover_provider: lsp_types::HoverProviderCapability::Simple(true).wrap_some(),
        definition_provider: lsp_types::OneOf::Left(true).wrap_some(),
        document_highlight_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

`ServerCapabilities::definition_provider` is `Option<OneOf<bool, DefinitionOptions>>` in lsp-types 0.97.0.

## End-user experience

Until the schema exists: hover on `id` in `field Pet.fullName { id }` shows nothing, and go-to-definition there does nothing. Go-to-definition on `Pet`, `fullName`, or a comma also does nothing. When the schema has `Pet.owner -> User` described as `"The owner"` and defined at `owner` on line 2 of `schema.graphql`, hover on `owner` shows

```text
field **Pet.owner**: **User**

The owner
```

and go-to-definition on `owner` opens `schema.graphql` with `owner` on line 2 selected, using the client's open buffer of `schema.graphql` when it has one.

## Unregistered-method tests

`textDocument/definition` is now registered. These tests send it as a method the server does not handle and expect `MethodNotFound`; they switch to `textDocument/references`, which the server does not register:

- `lsp_request_returns_method_not_found` in `crates/isograph_cli/src/state.rs` (method and the expected `"No handler registered for method 'textDocument/references'"` message).
- `unknown_request_after_initialize_is_method_not_found_then_hello_world`, `unknown_request_is_still_method_not_found`, and the request on the second connection in the test at `crates/isograph_cli/src/lsp_socket.rs` that drops the first connection.

`calls_first_matching_request_handler` in `lsp_request_dispatch.rs` registers its own `GotoDefinition` handler and is unchanged.

## Tests

- `crates/isograph_lsp/src/hover.rs`: `selectable_hover_markdown` of a `Selectable` built in the test, with a description (`"field **Pet.owner**: **User**\n\nThe owner"`) and without (`"field **Pet.owner**: **User**"`).
- `crates/isograph_cli/src/adapter.rs`: `definition_lsp_location` of a `DefinitionLocation` built in the test.
  - Disk file `src/schema.graphql` is `"type Pet {\n  owner: User\n}\n"`; the span of `owner` maps to URI `file:///tmp/proj/src/schema.graphql` and range line 1, characters 2 to 7.
  - With client 1's open buffer `"\ntype Pet {\n  owner: User\n}\n"` over that disk file, client 1 gets line 2 and client 2 gets line 1.
  - A path with no file is `UnexpectedError`.
- `crates/isograph_cli/src/state.rs`, through `handle` and `textDocument/definition`: `null` at `id` of `FIELD_X`, at `owner` and `name` of `iso(\`field Pet.fullName { owner { name } }\`)`, and at `Query`, `x`, and `{` of `FIELD_X`.
- `crates/isograph_cli/src/lsp_socket.rs`: `initialize_advertises_formatting_will_save_wait_until_hover_and_highlight` becomes `initialize_advertises_formatting_will_save_wait_until_hover_highlight_and_definition` and asserts `definitionProvider` is `true`.
