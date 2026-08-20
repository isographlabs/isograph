# selectable-name-wrapper: `SelectableNameWrapper` and `FieldDeclaration`

`ClientScalarSelectableNameWrapper` is `SelectableNameWrapper`. `ClientFieldDeclaration` is `FieldDeclaration`. `client_field_name` is `name`. `SelectionNameWrapper` stays: a selection's `name` and `reader_alias` are that wrapper. The left-hand side of `Type.name` on an entrypoint or field stays `EntityNameWrapper`.

Lands after parse-type-dot-name.md, before optional-to.md. No grammar change.

Origin: `ClientScalarSelectableNameWrapper` in `crates/isograph_parser/src/parse_iso_literal.rs`. Delta: rename to `SelectableNameWrapper`. Origin for the declaration: `ClientFieldDeclaration` in the same file. Delta: `FieldDeclaration`, field `name`.

## Changes to the wrapper

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(
    parent_type = ClientScalarSelectableNameWrapperParent<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct ClientScalarSelectableNameWrapper(common_lang_types::SelectableName);

pub enum ClientScalarSelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}

pub type ClientScalarSelectableNameWrapperPath<'a> = PositionResolutionPath<
    &'a ClientScalarSelectableNameWrapper,
    ClientScalarSelectableNameWrapperParent<'a>,
>;
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SelectableNameWrapperParent<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct SelectableNameWrapper(common_lang_types::SelectableName);

pub enum SelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    FieldDeclaration(FieldDeclarationPath<'a>),
}

pub type SelectableNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectableNameWrapper, SelectableNameWrapperParent<'a>>;
```

`SelectionNameWrapper` is unchanged: parent `SelectionPath`, used for `Selection.name` and `Selection.reader_alias`.

```rust
// from crates/isograph_parser/src/selections.rs
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectableName);
```

## Changes to FieldDeclaration

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct EntrypointDeclaration {
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
}

pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}

pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
}

pub type ClientFieldDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientFieldDeclaration, IsoLiteralSlotPath<'a>>;

pub type DescriptionPath<'a> =
    PositionResolutionPath<&'a Description, ClientFieldDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub name: WithSpan<SelectableNameWrapper>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct FieldDeclaration {
    #[resolve_field]
    #[parent_variant(FieldDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(FieldDeclaration)]
    pub name: WithSpan<SelectableNameWrapper>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(FieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}

pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    FieldDeclaration(FieldDeclarationPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
}

pub type FieldDeclarationPath<'a> =
    PositionResolutionPath<&'a FieldDeclaration, IsoLiteralSlotPath<'a>>;

pub type DescriptionPath<'a> =
    PositionResolutionPath<&'a Description, FieldDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(FieldDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = FieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description(common_lang_types::DescriptionValue);
```

`IsoLiteralItem::Field` stays the variant name. `parent_type` stays `EntityNameWrapper`.

## Changes to SelectionSetParent

Before:

```rust
// from crates/isograph_parser/src/selections.rs
pub enum SelectionSetParent<'a> {
    ClientFieldDeclaration(crate::ClientFieldDeclarationPath<'a>),
    Selection(Box<SelectionPath<'a>>),
}
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
pub enum SelectionSetParent<'a> {
    FieldDeclaration(crate::FieldDeclarationPath<'a>),
    Selection(Box<SelectionPath<'a>>),
}
```

`Selection.name` and `reader_alias` stay `SelectionNameWrapper`. Construction stays `first.interned().map(SelectionNameWrapper)`.

## Changes to parse_entrypoint and parse_field

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let (parent_type, client_field_name) = parse_type_dot_name(cursor)?;
    EntrypointDeclaration {
        parent_type,
        client_field_name: client_field_name.map(ClientScalarSelectableNameWrapper),
    }
```

```rust
    let (parent_type, client_field_name) = parse_type_dot_name(cursor)?;
    ClientFieldDeclaration {
        parent_type,
        client_field_name: client_field_name.map(ClientScalarSelectableNameWrapper),
        variable_definitions,
        description,
        selection_set,
    }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    EntrypointDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
    }
```

```rust
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    FieldDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        variable_definitions,
        description,
        selection_set,
    }
```

`parse_field` returns `FieldDeclaration`. `as_field` returns `&FieldDeclaration`.

## Changes to variables.rs

`VariableDeclarationOrUsageList`'s `parent_type` is `FieldDeclarationPath`. The path alias uses `FieldDeclarationPath`. `TypeAnnotationParent` is unchanged in this doc.

```rust
// from crates/isograph_parser/src/variables.rs
#[resolve_position(parent_type = FieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsageList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclarationOrUsage, UnparsedChunkItems>>>,
);

pub type VariableDeclarationOrUsageListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsageList, FieldDeclarationPath<'a>>;
```

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    FieldDeclaration(FieldDeclarationPath<'a>),
    SelectableNameWrapper(SelectableNameWrapperPath<'a>),
```

`ClientFieldDeclaration` and `ClientScalarSelectableNameWrapper` variants are deleted. `SelectionNameWrapper` stays.

A position on an entrypoint name or a field name answers `SelectableNameWrapper`. A position on a selection name or `reader_alias` answers `SelectionNameWrapper`. A position on `Query` in `entrypoint Query.foo` answers `EntityNameWrapper`.

## Tests

Every `ClientScalarSelectableNameWrapper`, `ClientFieldDeclaration`, `client_field_name`, and `ClientFieldDeclarationPath` identifier in `crates/isograph_parser` becomes the new name. `SelectionNameWrapper` identifiers stay.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        match parse.resolve((), span_of(text, "foo")) {
            IsographResolutionNode::SelectableNameWrapper(_) => {}
            node => panic!("expected the selectable name leaf, got {node:?}"),
        }
```

```rust
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::SelectionNameWrapper(_) => {}
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
```

`as_field` still exists; it returns `&FieldDeclaration`. Assertions on `declaration.client_field_name` become `declaration.name`.

Test function names that say `scalar` or `object` for nested-set presence:

```
scalar_selections_parse → selections_without_a_nested_set
object_selections_nest → selections_nest
arguments_parse_on_scalar_and_object_selections → arguments_parse_on_selections
a_field_declaration_parses_with_scalar_selections → a_field_declaration_parses_with_selections
```

## AGENTS.md on landing

```
Parser interned-key wrappers are named `$RoleWrapper` and wrap the `common_lang_types` interned type for that role (`FieldArgumentNameWrapper(FieldArgumentName)`, `SelectionNameWrapper(SelectableName)`, `SelectableNameWrapper(SelectableName)`). They do not implement `From<StringKey>`: `string_key_newtype!` already does that on the inner type. Construction is `token.interned().map(SelectionNameWrapper)`. A selection name and a `reader_alias` are `SelectionNameWrapper`. An entrypoint name and a field name are `SelectableNameWrapper`. The left-hand side of `Type.name` is `EntityNameWrapper`.
```

The Selection vs Selectable paragraph stays: a selection node is not named `Selectable*`. A selection's interned name is still `SelectableName`.

## Landing checklist

1. The parse_iso_literal.rs, selections.rs, variables.rs, and isograph_resolution_node.rs renames, the test identifier updates, the test function renames, and the AGENTS.md wrapper sentence; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
