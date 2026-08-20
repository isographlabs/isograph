# `VariableDeclarationOrUsage` is the shared `$name` node

The `$name` node is the same whether it is a declaration or a use. Distinction is the resolve parent, not the payload.

Does not depend on type-annotation-null.md or parse-iso-literal-entry.md. If parse-iso-literal-entry.md Change 3 has landed, its `pub use variables` list follows the renames here.

One change: types, parse, resolve node, `Slot` pin, tests.

## Types

Most important first.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsageParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsage(
    #[resolve_field]
    pub WithSpan<VariableNameWrapper>,
);

#[derive(Debug)]
pub enum VariableDeclarationOrUsageParent<'a> {
    Declaration(VariableDeclarationPath<'a>),
    Usage(VariableUsePath<'a>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsagePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(pub common_lang_types::VariableName);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse(
    #[resolve_field]
    #[parent_variant(Usage)]
    pub WithSpan<VariableDeclarationOrUsage>,
);

pub type VariableDeclarationOrUsagePath<'a> = PositionResolutionPath<
    &'a VariableDeclarationOrUsage,
    VariableDeclarationOrUsageParent<'a>,
>;

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableDeclarationOrUsagePath<'a>>;
```

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclaration {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableDeclarationOrUsage>,
    #[resolve_field]
    #[parent_variant(Variable)]
    pub type_: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(VariableDefault)]
    pub default_value: Option<WithSpan<NonConstantValue>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclaration, UnparsedChunkItems>>>,
);

pub type VariableDeclarationListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationList, SelectableDeclarationPath<'a>>;

pub type VariableDeclarationSlotPath<'a> = PositionResolutionPath<
    &'a Slot<VariableDeclaration, UnparsedChunkItems>,
    VariableDeclarationListPath<'a>,
>;

pub type VariableDeclarationPath<'a> =
    PositionResolutionPath<&'a VariableDeclaration, VariableDeclarationSlotPath<'a>>;
```

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub struct VariableDeclarationOrUsageList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclarationOrUsage, UnparsedChunkItems>>>,
);

pub struct VariableDeclarationOrUsage {
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableNameWrapper>,
    #[parent_variant(Variable)]
    pub type_: WithSpan<TypeAnnotation>,
    #[parent_variant(VariableDefault)]
    pub default_value: Option<WithSpan<NonConstantValue>>,
}

pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct VariableUse(
    #[parent_variant(Use)]
    pub WithSpan<VariableNameWrapper>,
);

pub struct VariableNameWrapper(pub common_lang_types::VariableName);

pub enum VariableNameWrapperParent<'a> {
    Use(VariableUsePath<'a>),
    Declaration(VariableDeclarationOrUsagePath<'a>),
}

pub enum NonConstantValueParent<'a> {
    Argument(Box<ArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
    VariableDefault(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListLiteralValuePath<'a>>),
}
```

`VariableUse` stays a newtype so `#[parent_variant(Usage)]` has a pin. It does not add fields. `VariableNameWrapperParent` is deleted. `NonConstantValue::Variable(VariableUse)` is unchanged at the enum. `NonConstantValueParent::VariableDefault` and `TypeAnnotationParent::Variable` take `VariableDeclarationPath`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
```

Before: `Option<WithSpan<VariableDeclarationOrUsageList>>`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableUse(VariableUsePath<'a>),
    VariableNameWrapper(VariableNameWrapperPath<'a>),
    VariableDeclarationOrUsage(VariableDeclarationOrUsagePath<'a>),
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    VariableDeclarationList(VariableDeclarationListPath<'a>),
    VariableDeclaration(VariableDeclarationPath<'a>),
```

Before: `VariableDeclarationOrUsageSlot`, `VariableDeclarationOrUsageList`, `VariableDeclarationOrUsage` named the declaration list, slot, and declaration struct. After: those three names are the `$name` node and the new `VariableDeclaration*` variants. `VariableNameWrapper` stays a leaf.

```rust
// from crates/isograph_parser/src/chunk.rs
        (<VariableDeclaration, UnparsedChunkItems>, VariableDeclarationListPath<'a>),
```

Before: `(<VariableDeclarationOrUsage, UnparsedChunkItems>, VariableDeclarationOrUsageListPath<'a>)`.

`UnparsedChunkItemsParent::VariableDeclarationOrUsageSlot` becomes `VariableDeclarationSlot`. The `From` impl follows.

```rust
// from crates/isograph_parser/src/parse_error.rs
    VariableDeclaration,
```

Before: `Expectation::VariableDeclarationOrUsage`. Display stays `a variable declaration, like '$id: ID!'`. The Display test uses the new variant.

```rust
// from crates/isograph_parser/src/variables.rs
impl<'a> From<VariableDeclarationSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationSlotPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationSlot(path)
    }
}
```

Before: `From<VariableDeclarationOrUsageSlotPath<'a>>` produced `VariableDeclarationOrUsageSlot`.

## Parse

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    missing_dollar: Expectation,
) -> Result<WithSpan<VariableDeclarationOrUsage>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        cursor
            .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
            .map_err(|()| cursor.expected(missing_dollar))?;
        let name = cursor
            .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
            .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
        VariableDeclarationOrUsage(name.interned().map(VariableNameWrapper)).wrap_ok()
    })
}
```

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    missing_dollar: Expectation,
) -> Result<WithSpan<VariableNameWrapper>, WithSpan<ParseError>> {
    cursor
        .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
        .map_err(|()| cursor.expected(missing_dollar))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    name.interned().map(VariableNameWrapper).wrap_ok()
}
```

Span is `$` plus the identifier. Call sites still wrap a use in `VariableUse(...)` and still pass the result to `parse_name_colon` as the declaration lhs.

```rust
// from crates/isograph_parser/src/variables.rs
fn parse_variable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<VariableDeclaration, WithSpan<ParseError>> {
    let (name, type_) = parse_name_colon(
        cursor,
        |cursor| parse_variable_name(cursor, Expectation::VariableDeclaration),
        parse_type_annotation,
    )?;
    let default_value =
        match cursor.consume_token_if(NonBracketTokenKind::Equals, SemanticToken::Equals) {
            Some(_) => parse_non_constant_value(cursor)?.wrap_some(),
            None => None,
        };
    VariableDeclaration {
        name,
        type_,
        default_value,
    }
    .wrap_ok()
}
```

Before: returned `VariableDeclarationOrUsage` and passed `Expectation::VariableDeclarationOrUsage`. `consume_variable_declaration_list` builds `VariableDeclarationList`.

Who calls: `parse_variable_declaration`, `parse_non_constant_value`'s `$` arm, `parse_variable_name` callers. `chunk.rs` pin list.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_dollar_in_a_use_resolves_to_declaration_or_usage_with_usage_parent() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Usage(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "x")) {
            IsographResolutionNode::VariableNameWrapper(name) => {
                assert_eq!(name.inner.0, "x".intern().to());
            }
            node => panic!("expected the name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_dollar_in_a_declaration_resolves_to_declaration_or_usage_with_declaration_parent() {
        let text = "field Query.Foo($id: ID) { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Declaration(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::VariableNameWrapper(_) => {}
            node => panic!("expected the name leaf, got {node:?}"),
        }
    }
```

`argument_names_resolve_through_the_selection` and `a_default_variable_resolves_through_variable_default` currently expect `VariableUse` on `$`. They expect `VariableDeclarationOrUsage` with `Usage` parent. `type_names_resolve_through_their_annotation_ancestry` currently matches `VariableNameWrapperParent::Declaration`; the name's parent is `VariableDeclarationOrUsagePath`, whose parent is `Declaration`.

Helpers in that test module that name `VariableDeclarationOrUsage` as the declaration struct (`as_declared`, `variables_of`) take `VariableDeclaration` / `VariableDeclarationList`.
