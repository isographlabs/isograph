# Selection completion

`textDocument/completion` inside a selection set offers the fields of the entity that selection set selects on, from the source-backed lookups (`refactors/past/source-backed-lookups.md`).

Origin: isograph `crates/isograph_lsp/src/completion.rs` (`on_completion`) and `get_parent_for_selection_set_path` / `selectables_for_entity` (`crates/isograph_schema`). Delta: i2 resolves the char before the caret, not a zero-width span at the caret; it also accepts a caret on a selection's name; it excludes fields already selected in the set; items carry the target entity as `detail`; a caret outside every literal returns `null`, and a caret inside a literal but not in a selection set returns an empty list.

## What the user sees

```ts
iso(`entity Pet`)
iso(`field Pet.owner to Pet "Who owns it."`)
iso(`field Pet.name`)
iso(`field Pet.card { name, | }`)        // offers `owner` (detail `Pet`, documentation `Who owns it.`), `card`; not `name`
iso(`field Pet.card { owner { na| } }`)  // offers `card`, `name`, `owner`: the set under `owner` selects on `Pet`; `na` is under the caret, so `name` stays
```

The editor asks with no trigger character (ctrl-space, or as the user types an identifier). Accepting an item inserts the field name only, never braces. A caret on `field`, on an argument, or between literals' items returns an empty list; a caret in the TypeScript around the literals returns `null`, so the editor's TypeScript completions apply. A selection set whose parent entity is unknown returns an empty list.

## Position

```rust
// from crates/isograph_lsp/src/completion.rs
/// The completion items at `position` in `path` as `client` reads it. `None` outside every
/// iso literal.
pub fn completion_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<Vec<lsp_types::CompletionItem>> {
    let (literal_id, before_caret) = iso_literal_before_lsp_position(db, client, path, position)?;
    before_caret
        .and_then(|span| {
            let parsed = parsed_iso_literal_by_id(db, literal_id)?;
            completion_items(db, client, resolve_offset(parsed, span)?)
        })
        .unwrap_or_default()
        .wrap_some()
}

/// The literal whose interior contains the caret at `position` (its end included), and the
/// span of the char before the caret relative to that interior. `None` for the span when
/// the caret is at the start of the interior.
fn iso_literal_before_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Option<Span>)> {
    let offset = byte_offset_at_lsp_position(db, client, path, position)?;
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions.iter().enumerate().find_map(|(index, extraction)| {
        let within = offset.checked_sub(extraction.iso_literal_start_index.0 as u32)?;
        let before = extraction.iso_literal_text.get(..within as usize)?;
        (
            LiteralId { client, path, index },
            before
                .chars()
                .next_back()
                .map(|character| Span::new(within - character.len_utf8() as u32, within)),
        )
            .wrap_some()
    })
}
```

`iso_literal_at_lsp_position` (`hover.rs`) finds the char under the caret; this finds the char before it, so the caret at the end of a name or just inside `}` still names the set. `get(..within)` is `None` past the interior's end and inside a multi-byte char, so the caret has to be in the interior or at its end.

```rust
// from crates/isograph_lsp/src/completion.rs
/// The items for a completion request whose char before the caret resolves to `node`. `None`
/// when `node` is not in a selection set, or the set's entity is unknown.
fn completion_items<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    node: IsographResolutionNode<'_>,
) -> Option<Vec<lsp_types::CompletionItem>> {
    let (selection_set_path, under_caret) = match node.reference() {
        IsographResolutionNode::SelectionSet(selection_set_path) => (selection_set_path, None),
        IsographResolutionNode::SelectionNameWrapper(selection_name_path) => (
            selection_name_path.parent.parent.parent.reference(),
            selection_name_path.parent.inner.wrap_some(),
        ),
        IsographResolutionNode::IsoLiteralSlot(_)
        | IsographResolutionNode::ChunkedRoot(_)
        // ... every other variant of `IsographResolutionNode`, listed as in
        // `entity_or_selectable_of_node`
        | IsographResolutionNode::ListLiteralValueSlot(_) => return None,
    };
    let entity = entity_for_selection_set_path(db, client, selection_set_path).ok()?;
    let selected: BTreeSet<SelectableName> = selection_set_path
        .inner
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .filter(|selection| {
            under_caret.is_none_or(|under_caret| selection.item.name.location != under_caret.name.location)
        })
        .map(|selection| selection.item.name.item.0.unchecked_conversion())
        .collect();
    selectables_for_entity(db, client, entity)
        .as_ref()
        .ok()?
        .iter()
        .filter(|selectable| !selected.contains(&selectable.name))
        .map(completion_item)
        .collect::<Vec<_>>()
        .wrap_some()
}

fn completion_item(selectable: &Selectable) -> lsp_types::CompletionItem {
    lsp_types::CompletionItem {
        label: selectable.name.to_string(),
        kind: lsp_types::CompletionItemKind::FIELD.wrap_some(),
        detail: selectable.target_entity.map(|target| target.to_string()),
        documentation: selectable
            .description
            .map(|description| lsp_types::Documentation::String(description.to_string())),
        ..Default::default()
    }
}
```

The last arm names every other variant of `IsographResolutionNode`, as `entity_or_selectable_of_node` does, so a new variant is a compile error here. A selection is excluded by its field name, so `alias: name` excludes `name`. The selection under the caret is compared by the span of its field name, which is unique within one parse. `insert_text` is unset, so the editor inserts the label.

## The entity of a selection set

```rust
// from crates/isograph_compiler/src/selectable_for_path.rs
/// The entity the selection set at `selection_set_path` selects on.
pub fn entity_for_selection_set_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_set_path: &SelectionSetPath<'_>,
) -> DiagnosticResult<EntityName> {
    let entity_name = match selection_set_path.parent.reference() {
        SelectionSetParent::SelectableDeclaration(selectable_declaration_path) => {
            selectable_declaration_path.inner.parent_type.item.0
        }
        SelectionSetParent::Selection(selection_path) => {
            selectable_target(selectable_for_selection_path(db, client, selection_path)?.reference())?
        }
    };
    entity_named(db, client, entity_name).clone().map(|entity| entity.name)
}
```

Before, `selectable_for_selection_parent` held that match and called `selectable_for_entity`. After:

```rust
// from crates/isograph_compiler/src/selectable_for_path.rs
pub fn selectable_for_selection_parent<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    selection_set_path: &SelectionSetPath<'_>,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    let entity_name = entity_for_selection_set_path(db, client, selection_set_path)?;
    selectable_named(db, client, entity_name, selectable_name).clone()
}
```

`selectable_for_entity` has no caller left and is deleted. The walk's results are unchanged: it still fails at an unknown entity before looking up the selectable.

## Selectables of an entity

```rust
// from crates/isograph_compiler/src/schema.rs
/// Every selectable on the entity named `entity_name` that exactly one source file
/// declaration defines, by name.
#[memo]
pub fn selectables_for_entity<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Vec<Selectable>> {
    entity_named(db, client, entity_name).clone()?;
    source_iso_literal_items(db, client)
        .filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Selectable(declaration) => (declaration.parent_type.item.0
                == entity_name)
                .then(|| selectable_of_declaration(source_item.path, source_item.literal_start, declaration)),
            IsoLiteralItem::Entity(_) | IsoLiteralItem::Entrypoint(_) => None,
        })
        .fold(
            BTreeMap::<SelectableName, Vec<Selectable>>::new(),
            |mut by_name, selectable| {
                by_name.entry(selectable.name).or_default().push(selectable);
                by_name
            },
        )
        .into_values()
        .filter_map(|definitions| {
            <[Selectable; 1]>::try_from(definitions)
                .ok()
                .map(|[selectable]| selectable)
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}
```

A duplicated field is left out, matching `selectable_named`, which is `Err` for it. The list is sorted by name.

## Adapter

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn completion_response<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    params: lsp_types::CompletionParams,
) -> LSPRuntimeResult<Option<lsp_types::CompletionResponse>> {
    let position = params.text_document_position;
    let path = relative_from_uri(db, position.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::completion_at_lsp_position(db, client, path, position.position)
        .map(lsp_types::CompletionResponse::Array)
        .wrap_ok()
}
```

`dispatch_lsp_request` (`state.rs`) adds `.on_request_sync::<lsp_types::request::Completion>(crate::adapter::completion_response::<TProfile>)?` after `DocumentHighlightRequest`. `run_session` (`lsp_socket.rs`) adds `completion_provider: lsp_types::CompletionOptions::default().wrap_some()` (no trigger characters, no resolve).

## Tests

`crates/isograph_lsp/src/completion.rs` (Pet fixture above in `src/Pet.ts`, the literal under test in `src/x.ts`), asserting labels, and kind, detail, and documentation of `owner`:

- `field Pet.card { | }`, `{|}`: every field of `Pet`.
- `field Pet.card { na| }`: every field, `name` included.
- `field Pet.card { owner { | } }`: every field of `Pet` (through `to Pet`).
- `field Pet.card { name, | }` and `{ n: name, | }`: `name` excluded.
- caret in the TypeScript between literals: `None`.
- caret on `field`, on an argument value, at the start of the interior: `Some([])`.
- `field Nope.card { | }` and `field Pet.card { plain { | } }` under a field without target: `Some([])`.
- `selectables_for_entity`: a duplicated field is left out; an unknown entity is `Err`.

`crates/isograph_cli`: the `initialize` result has `completionProvider`; a `textDocument/completion` request through `handle` returns the array of items, and `null` outside a literal.
