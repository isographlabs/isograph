# selectable-declaration: `FieldDeclaration` is `SelectableDeclaration`

`FieldDeclaration` is `SelectableDeclaration`. The keyword in the source is still `field`. Origin: mental model `SelectableDeclaration` in `docs-website/docs/design-docs/mental-model.md`.

Lands before optional-field-selection-set.md. No grammar change.

## What is renamed

```
FieldDeclaration                 -> SelectableDeclaration
FieldDeclarationPath             -> SelectableDeclarationPath
parse_field                      -> parse_selectable_declaration
as_field                         -> as_selectable
IsoLiteralItem::Field            -> IsoLiteralItem::Selectable
IsographResolutionNode::FieldDeclaration -> SelectableDeclaration

SelectionFieldArgument           -> SelectionArgument
SelectionFieldArgumentPath       -> SelectionArgumentPath
SelectionFieldArgumentSlotPath   -> SelectionArgumentSlotPath
FieldArgumentNameWrapper         -> ArgumentNameWrapper
FieldArgumentNameWrapperPath     -> ArgumentNameWrapperPath
```

Parent-enum variants named `FieldDeclaration` become `SelectableDeclaration`:

```
EntityNameWrapperParent
SelectableNameWrapperParent
SelectionSetParent
TypeAnnotationParent
IsographFieldDirectiveListParent   (parse-directives.md, after this doc)
```

`Description`'s `parent_type` is `SelectableDeclarationPath`. `VariableDeclarationOrUsageList`'s `parent_type` is `SelectableDeclarationPath`.

The match arm is still `"field"`:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "field" => IsoLiteralItem::Selectable(parse_selectable_declaration(cursor)?).wrap_ok(),
```

## What is not renamed

- The keyword `field`.
- `SemanticToken::FieldName`: highlighter role for an identifier in `Type.name` and in a selection, not the declaration type.
- `FieldArgumentName` in `common_lang_types`: isograph still uses it. The parser interned key is a new `ArgumentName`.
- `Expectation::Selection` (a selection in a set). Its message becomes `"a selection"` (it currently says `"a field selection"`).

## Changes to the declaration

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(FieldDeclaration),
}

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
    #[parent_variant(FieldDeclaration)]
    pub target_type: Option<WithSpan<TypeAnnotation>>,
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

pub enum SelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    FieldDeclaration(FieldDeclarationPath<'a>),
}

pub type FieldDeclarationPath<'a> =
    PositionResolutionPath<&'a FieldDeclaration, IsoLiteralSlotPath<'a>>;
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Selectable(SelectableDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectableDeclaration {
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub name: WithSpan<SelectableNameWrapper>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub target_type: Option<WithSpan<TypeAnnotation>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}

pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
}

pub enum SelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
}

pub type SelectableDeclarationPath<'a> =
    PositionResolutionPath<&'a SelectableDeclaration, IsoLiteralSlotPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description(common_lang_types::DescriptionValue);
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// The name of an entrypoint or selectable, `foo` in `entrypoint Query.foo`.
pub struct SelectableNameWrapper(common_lang_types::SelectableName);
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_selectable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectableDeclaration, WithSpan<ParseError>> {
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let target_type = consume_to_target(cursor)?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor, Expectation::ToOrDescriptionOrSelectionSet)?;
    SelectableDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        variable_definitions,
        target_type,
        description,
        selection_set,
    }
    .wrap_ok()
}
```

## Changes to argument names

Origin: `FieldArgumentName` in `crates/common_lang_types/src/string_key_types.rs` and `SelectionFieldArgument` in `crates/isograph_parser/src/arguments.rs`. Delta: parser interned key is `ArgumentName`; the pair is `SelectionArgument`. `FieldArgumentName` stays in `common_lang_types`.

```rust
// from crates/common_lang_types/src/string_key_types.rs
string_key_newtype!(ArgumentName);
string_key_equality!(ArgumentName, VariableName);
string_key_one_way_conversion!(from: InputValueName, to: ArgumentName);
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionArgument, UnparsedChunkItems>>>,
);

pub struct SelectionArgument {
    #[resolve_field]
    pub name: WithSpan<ArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(SelectionArgument)]
    pub value: WithSpan<NonConstantValue>,
}

#[resolve_position(parent_type = SelectionArgumentPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentNameWrapper(common_lang_types::ArgumentName);
```

`NonConstantValueParent::SelectionFieldArgument` becomes `SelectionArgument`. `UnparsedChunkItemsParent::SelectionFieldArgumentSlot` becomes `SelectionArgumentSlot`. `parse_argument` returns `SelectionArgument`. Construction is `name.map(ArgumentNameWrapper)`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    SelectionArgumentSlot(SelectionArgumentSlotPath<'a>),
    SelectionArgument(SelectionArgumentPath<'a>),
    ArgumentNameWrapper(ArgumentNameWrapperPath<'a>),
```

## Changes to selections.rs and variables.rs

```rust
// from crates/isograph_parser/src/selections.rs
pub enum SelectionSetParent<'a> {
    SelectableDeclaration(crate::SelectableDeclarationPath<'a>),
    Selection(Box<SelectionPath<'a>>),
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("a selection")]
    Selection,
```

```rust
// from crates/isograph_parser/src/variables.rs
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsageList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclarationOrUsage, UnparsedChunkItems>>>,
);

pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
}

pub type VariableDeclarationOrUsageListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsageList, SelectableDeclarationPath<'a>>;
```

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    SelectableDeclaration(SelectableDeclarationPath<'a>),
```

`FieldDeclaration` is deleted.

## Tests

Every `FieldDeclaration`, `FieldDeclarationPath`, `parse_field`, `as_field`, `IsoLiteralItem::Field`, `SelectionFieldArgument`, and `FieldArgumentNameWrapper` identifier in `crates/isograph_parser` becomes the new name. Test function names that say `field_declaration` become `selectable_declaration`. Fixtures still write the keyword `field`.

```rust
    fn as_selectable(parse: &WithSpan<IsoLiteralParse>) -> &SelectableDeclaration {
        match parsed_item(parse).expect("the fixture's literal parsed an item") {
            IsoLiteralItem::Selectable(declaration) => declaration,
            item => panic!("expected a selectable declaration, got {item:?}"),
        }
    }
```

`Expectation::Selection.to_string()` is `"a selection"`.

## AGENTS.md on landing

The interned-key sentence lists `ArgumentNameWrapper(ArgumentName)` in place of `FieldArgumentNameWrapper(FieldArgumentName)`. An entrypoint name and a selectable name are `SelectableNameWrapper`.

## Landing checklist

1. The renames in parse_iso_literal.rs, selections.rs, variables.rs, arguments.rs, chunk.rs, isograph_resolution_node.rs, parse_error.rs, `ArgumentName` in common_lang_types, tests, and the AGENTS.md sentence. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
