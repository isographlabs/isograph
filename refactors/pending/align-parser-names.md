# align-parser-names: match isograph names on landed parser types

Lands before parse-selection-sets.md. Renames the grammar types that parse-entrypoint.md and parse-arguments.md introduced so they use the isograph names. Behavior does not change.

Slots, `IsographResolutionNode`, `consume_*` / `require_*`, and the resolve-position wrappers that isograph does not have are out of scope; those differences stay and are listed in parsing-plan.md.

## Change 1: declaration name wrappers

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct EntityName(common_lang_types::EntityName);

impl From<intern::string_key::StringKey> for EntityName {
    fn from(key: intern::string_key::StringKey) -> Self {
        EntityName(key.to())
    }
}

pub struct ClientFieldName(common_lang_types::SelectableName);

impl From<intern::string_key::StringKey> for ClientFieldName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ClientFieldName(key.to())
    }
}

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> =
    PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    EntrypointDeclaration {
        parent_type: parent_type.interned(),
        client_field_name: client_field_name.interned(),
    }
```

After. Origin: `crates/isograph_lang_types/src/string_key_wrappers.rs` (`EntityNameWrapper`, `ClientScalarSelectableNameWrapper`). Delta: i2 wrappers stay `Copy` + `ResolvePosition` newtypes; they do not impl `From<StringKey>` or `Deref`. `interned()` infers the inner lang type; `.map` wraps.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct EntityNameWrapper(common_lang_types::EntityName);

pub struct ClientScalarSelectableNameWrapper(common_lang_types::SelectableName);

pub type EntityNameWrapperPath<'a> =
    PositionResolutionPath<&'a EntityNameWrapper, EntrypointDeclarationPath<'a>>;

pub type ClientScalarSelectableNameWrapperPath<'a> =
    PositionResolutionPath<&'a ClientScalarSelectableNameWrapper, EntrypointDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    EntrypointDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name
            .interned()
            .map(ClientScalarSelectableNameWrapper),
    }
```

`EntrypointDeclaration.parent_type` stays `WithSpan<EntityNameWrapper>`. `client_field_name` stays `WithSpan<ClientScalarSelectableNameWrapper>`. The field names do not change.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    EntityNameWrapper(EntityNameWrapperPath<'a>),
    ClientScalarSelectableNameWrapper(ClientScalarSelectableNameWrapperPath<'a>),
```

Tests that construct or match `EntityName(...)` / `ClientFieldName(...)` / `IsographResolutionNode::EntityName` / `IsographResolutionNode::ClientFieldName` respell. `as_entrypoint` still reads `declaration.parent_type.item` and compares to `"Query".intern().to()` via the inner key; wrap the expected value in `EntityNameWrapper(...)` where the test compares the wrapper.

## Change 2: `SelectionFieldArgument`

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct NamedArgument {
    #[resolve_field]
    #[parent_variant(NamedArgument)]
    pub name: WithSpan<FieldArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(NamedArgument)]
    pub value: WithSpan<NonConstantValue>,
}

pub type NamedArgumentSlotPath<'a> =
    PositionResolutionPath<&'a Slot<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>>;

pub type NamedArgumentPath<'a> =
    PositionResolutionPath<&'a NamedArgument, NamedArgumentSlotPath<'a>>;
```

After. Origin: `crates/isograph_lang_types/src/declarations/selection_argument.rs` (`SelectionFieldArgument`). Delta: the pair sits in a `Slot`; `name` is `FieldArgumentNameWrapper` so it can carry `ResolvePosition`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct SelectionFieldArgument {
    #[resolve_field]
    #[parent_variant(SelectionFieldArgument)]
    pub name: WithSpan<FieldArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(SelectionFieldArgument)]
    pub value: WithSpan<NonConstantValue>,
}

pub type SelectionFieldArgumentSlotPath<'a> = PositionResolutionPath<
    &'a Slot<SelectionFieldArgument, UnparsedChunkItems>,
    ArgumentListPath<'a>,
>;

pub type SelectionFieldArgumentPath<'a> =
    PositionResolutionPath<&'a SelectionFieldArgument, SelectionFieldArgumentSlotPath<'a>>;
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
);
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum FieldArgumentNameWrapperParent<'a> {
    SelectionFieldArgument(SelectionFieldArgumentPath<'a>),
}

pub enum NonConstantValueParent<'a> {
    SelectionFieldArgument(Box<SelectionFieldArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
}
```

Change 2 and Change 3 land together: `ObjectEntry.name` cannot stay on `FieldArgumentNameWrapper` once that wrapper has only the argument as a parent.

`parse_argument` keeps that name. Origin: `parse_argument` in `crates/isograph_lang_parser/src/parse_iso_literal.rs`.

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectionFieldArgument, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::Argument, Expectation::Argument)?;
    SelectionFieldArgument { name, value }.wrap_ok()
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<SelectionFieldArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    SelectionFieldArgumentSlot(SelectionFieldArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    SelectionFieldArgumentSlot(SelectionFieldArgumentSlotPath<'a>),
    SelectionFieldArgument(SelectionFieldArgumentPath<'a>),
```

`From<SelectionFieldArgumentSlotPath>` for `IsographResolutionNode` and for `UnparsedChunkItemsParent`. Every `NamedArgument` identifier in arguments.rs tests, chunk.rs pins, and parsing-standards.md listings respells.

## Change 3: object-entry keys are `ValueKeyName`

Origin: `parse_object_entry` in `crates/isograph_lang_parser/src/parse_iso_literal.rs` returns `NameValuePair<ValueKeyName, NonConstantValue>`. Argument names are `FieldArgumentName`. Those are different lang types.

i2 cannot use `NameValuePair`: a `Slot<T, E>` pin's `T` has one list parent, and argument lists and object literals are two list parents. `ObjectEntry` stays. Its `name` field uses `ValueKeyNameWrapper`.

Before (after Change 2, `ObjectEntry` still on `FieldArgumentNameWrapper`):

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct ObjectEntry {
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub name: WithSpan<FieldArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

pub enum FieldArgumentNameWrapperParent<'a> {
    SelectionFieldArgument(SelectionFieldArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
}
```

After:

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct ObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ValueKeyNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectEntryPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ValueKeyNameWrapper(common_lang_types::ValueKeyName);

pub type ValueKeyNameWrapperPath<'a> =
    PositionResolutionPath<&'a ValueKeyNameWrapper, ObjectEntryPath<'a>>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SelectionFieldArgumentPath<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct FieldArgumentNameWrapper(common_lang_types::FieldArgumentName);

pub type FieldArgumentNameWrapperPath<'a> =
    PositionResolutionPath<&'a FieldArgumentNameWrapper, SelectionFieldArgumentPath<'a>>;
```

`FieldArgumentNameWrapper` is vanilla. Directive arguments are `SelectionFieldArgument` (parse-directives.md), so this parent does not grow. `FieldArgumentNameWrapperParent` is deleted.

`SelectionFieldArgument.name` drops `#[parent_variant(SelectionFieldArgument)]`; the child's parent is the pair path.

`ObjectEntry.name` is vanilla: `ValueKeyNameWrapper::Parent` is `ObjectEntryPath`. No `parent_variant` on that field.

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_name_colon_value<N: From<intern::string_key::StringKey>>(
    cursor: &mut ItemCursor<'_>,
    name_token: SemanticToken,
    missing_name: Expectation,
) -> Result<(WithSpan<N>, WithSpan<NonConstantValue>), WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_non_constant_value(cursor)?;
    (name.interned(), value).wrap_ok()
}

fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectionFieldArgument, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::Argument, Expectation::Argument)?;
    SelectionFieldArgument {
        name: name.map(FieldArgumentNameWrapper),
        value,
    }
    .wrap_ok()
}

fn parse_object_entry(cursor: &mut ItemCursor<'_>) -> Result<ObjectEntry, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::ObjectKey, Expectation::ObjectEntry)?;
    ObjectEntry {
        name: name.map(ValueKeyNameWrapper),
        value,
    }
    .wrap_ok()
}
```

`interned()` infers `FieldArgumentName` or `ValueKeyName` from `.map`. `parse_name_colon_value`'s `N` is that inner lang type.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ValueKeyNameWrapper(ValueKeyNameWrapperPath<'a>),
```

Tests that read `as_entry(...).name.item` compare `ValueKeyNameWrapper("id".intern().to())`.

## Change 4: `parse_non_constant_value`

Before: `parse_value`. After: `parse_non_constant_value`. Origin: `parse_non_constant_value` in `crates/isograph_lang_parser/src/parse_iso_literal.rs`. The body is unchanged. Call sites in `parse_name_colon_value` and tests respell.

## Tests

Existing arguments.rs and parse_iso_literal.rs tests. Identifier respell plus the `ValueKeyNameWrapper` comparisons. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.

## Landing checklist

1. The four renames, the `ValueKeyNameWrapper` split, the `From<StringKey>` deletions, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
