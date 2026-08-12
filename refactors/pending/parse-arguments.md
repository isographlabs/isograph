# parse-arguments: argument lists and values

Fourth doc of the series parsing-plan.md orders, after parse-fields.md. Selections gain argument lists, and the value grammar arrives: variables, strings, integers, booleans, null, and object literals. parse-variables.md reuses the value grammar for defaults.

## The grammar this doc accepts

A selection may carry a paren group between its name and its optional selection set:

```
[<Identifier> :] <Identifier> [<paren group>] [<brace group>]
```

Each contentful chunk of the paren group's interior is one argument:

```
<Identifier> : <value>
```

A value is one of:

```
$ <Identifier>          a variable
"..."                   a string literal (the span includes the quotes)
42, -7                  an integer literal, converted to i64
true, false             a boolean
null                    null
{ <entries> }           an object literal, each contentful chunk one `<Identifier> : <value>` entry
```

An argument or entry chunk that fails becomes an unparsed item; its siblings parse normally.

## Changes to parse_error.rs

`ParseError` gains the integer variant, `Expectation` gains three:

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum ParseError {
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    /// Temporary: parse-pointers.md removes this variant.
    UnsupportedDeclarationType,
    /// The token is a well-formed integer literal, so the conversion fails only on
    /// overflow.
    IntegerOutOfRange,
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum Expectation {
    // ... the parse-fields.md variants ...
    /// One argument: `name: value`.
    Argument,
    /// One value: a variable, string, integer, boolean, null, or object literal.
    Value,
    /// One object-literal entry: `name: value`.
    ObjectEntry,
}
```

The new `Display` arms:

```rust
// from crates/isograph_parser/src/parse_error.rs
            ParseError::IntegerOutOfRange => {
                write!(f, "This integer does not fit in a 64-bit signed integer.")
            }
```

```rust
// from crates/isograph_parser/src/parse_error.rs
            Expectation::Argument => write!(f, "an argument, like 'id: $id'"),
            Expectation::Value => {
                write!(f, "a value, like $foo, 42, \"bar\", true, false, null, or an object literal")
            }
            Expectation::ObjectEntry => write!(f, "an object entry, like 'id: 4'"),
```

## New module: arguments.rs

```rust
// from crates/isograph_parser/src/arguments.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::IntoSafePeekable;
use span::{Span, WithSpan};

use crate::{
    consume_token_if, expect_chunk_end, expect_token, parse_level_items, token_text, BracketKind,
    Chunk, ChunkContentItem, ChunkContents, Expectation, Found, IsographResolutionNode,
    NonBracketTokenKind, ObjectSelectionPath, ParseError, ScalarSelectionPath, UnparsedItem,
};

/// The arguments a `( ... )` group holds, one per contentful chunk of its interior.
/// The wrapping `WithSpan`'s span covers the parens.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(#[resolve_field] pub Vec<WithSpan<Argument>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Argument {
    Named(NamedArgument),
    Unparsed(#[resolve_field(parent_variant = ArgumentList)] UnparsedItem),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedArgument {
    #[resolve_field]
    pub name: WithSpan<ArgumentName>,
    #[resolve_field(parent_variant = Argument)]
    pub value: WithSpan<NonConstantValue>,
}

/// A value in an argument position. The wrapping `WithSpan`'s span covers the whole
/// value: `$id`, `"..."` with its quotes, `{ ... }` with its braces.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringValue),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
}

/// `$name`. The dollar's position answers this node; the name is its own leaf.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse {
    pub dollar: WithSpan<Dollar>,
    #[resolve_field]
    pub name: WithSpan<VariableName>,
}

/// A string literal value; its text, quotes included, is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringValue;

/// An integer literal value, converted; the source text is the span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

/// `true` or `false`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BooleanValue(pub bool);

/// `null`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullValue;

/// The entries a `{ ... }` value holds, one per contentful chunk of its interior.
/// The wrapping `WithSpan`'s span covers the braces.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectLiteral(#[resolve_field] pub Vec<WithSpan<ObjectEntry>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectLiteralPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ObjectEntry {
    Named(NamedObjectEntry),
    Unparsed(#[resolve_field(parent_variant = ObjectLiteral)] UnparsedItem),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectLiteralPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ObjectEntryName>,
    #[resolve_field(parent_variant = ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

/// An argument's name. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedArgumentPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentName;

/// A used variable's name, without its dollar. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableUsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableName;

/// An object-literal entry's name. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedObjectEntryPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectEntryName;

/// The `$` of a variable use. Positions on it answer the variable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Dollar;

#[derive(Debug)]
pub enum ArgumentListParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    Argument(NamedArgumentPath<'a>),
    ObjectEntry(Box<NamedObjectEntryPath<'a>>),
    // parse-variables.md adds VariableDefault
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;

pub type NamedArgumentPath<'a> = PositionResolutionPath<&'a NamedArgument, ArgumentListPath<'a>>;

pub type VariableUsePath<'a> = PositionResolutionPath<&'a VariableUse, NonConstantValueParent<'a>>;

pub type StringValuePath<'a> = PositionResolutionPath<&'a StringValue, NonConstantValueParent<'a>>;

pub type IntegerValuePath<'a> = PositionResolutionPath<&'a IntegerValue, NonConstantValueParent<'a>>;

pub type BooleanValuePath<'a> = PositionResolutionPath<&'a BooleanValue, NonConstantValueParent<'a>>;

pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, NonConstantValueParent<'a>>;

pub type ObjectLiteralPath<'a> = PositionResolutionPath<&'a ObjectLiteral, NonConstantValueParent<'a>>;

pub type NamedObjectEntryPath<'a> = PositionResolutionPath<&'a NamedObjectEntry, ObjectLiteralPath<'a>>;

pub type ArgumentNamePath<'a> = PositionResolutionPath<&'a ArgumentName, NamedArgumentPath<'a>>;

pub type VariableNamePath<'a> = PositionResolutionPath<&'a VariableName, VariableUsePath<'a>>;

pub type ObjectEntryNamePath<'a> = PositionResolutionPath<&'a ObjectEntryName, NamedObjectEntryPath<'a>>;
```

`NonConstantValueParent::ObjectEntry` is boxed to break the cycle `NonConstantValueParent -> NamedObjectEntryPath -> ObjectLiteralPath -> NonConstantValueParent`. `UnparsedItemParent` in selections.rs gains the two new list contexts:

```rust
// from crates/isograph_parser/src/selections.rs
pub enum UnparsedItemParent<'a> {
    SelectionSet(SelectionSetPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    // parse-variables.md adds VariableDeclarationList
}
```

The parse functions:

```rust
// from crates/isograph_parser/src/arguments.rs
/// The next item, consumed, when it is a paren group; its interior parses into
/// arguments, infallibly, each failed chunk degrading to `Argument::Unparsed`.
pub(crate) fn consume_argument_list(
    text: &str,
    items: &mut ChunkContents<'_>,
) -> Option<WithSpan<ArgumentList>> {
    let peek = items.peek()?;
    let item = peek.view();
    match &item.item {
        ChunkContentItem::Group(group) if group.opening.item.0 == BracketKind::Parenthesis => {
            let argument_list = ArgumentList(parse_level_items(
                &group.children.item,
                Expectation::Argument,
                |chunk| parse_argument(text, chunk),
                Argument::Unparsed,
            ));
            let span = item.location;
            peek.commit();
            Some(WithSpan::new(argument_list, span))
        }
        _ => None,
    }
}

fn parse_argument(text: &str, chunk: &WithSpan<Chunk>) -> Result<Argument, WithSpan<ParseError>> {
    let mut items = chunk.item.contents.iter().safe_peekable();
    let name = expect_token(
        &mut items,
        NonBracketTokenKind::Identifier,
        Expectation::Argument,
        chunk.location.start,
    )?;
    let colon = expect_token(
        &mut items,
        NonBracketTokenKind::Colon,
        Expectation::Token(NonBracketTokenKind::Colon),
        name.end,
    )?;
    let value = parse_value(text, &mut items, colon.end)?;
    expect_chunk_end(&mut items, Expectation::Separator)?;
    Ok(Argument::Named(NamedArgument {
        name: WithSpan::new(ArgumentName, name),
        value,
    }))
}

fn parse_object_entry(
    text: &str,
    chunk: &WithSpan<Chunk>,
) -> Result<ObjectEntry, WithSpan<ParseError>> {
    let mut items = chunk.item.contents.iter().safe_peekable();
    let name = expect_token(
        &mut items,
        NonBracketTokenKind::Identifier,
        Expectation::ObjectEntry,
        chunk.location.start,
    )?;
    let colon = expect_token(
        &mut items,
        NonBracketTokenKind::Colon,
        Expectation::Token(NonBracketTokenKind::Colon),
        name.end,
    )?;
    let value = parse_value(text, &mut items, colon.end)?;
    expect_chunk_end(&mut items, Expectation::Separator)?;
    Ok(ObjectEntry::Named(NamedObjectEntry {
        name: WithSpan::new(ObjectEntryName, name),
        value,
    }))
}

/// One value as the next item (or two, for a variable's `$ name`); the error otherwise.
pub(crate) fn parse_value(
    text: &str,
    items: &mut ChunkContents<'_>,
    missing_at: u32,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    let Some(peek) = items.peek() else {
        return Err(WithSpan::new(
            ParseError::expected(Expectation::Value, Found::EndOfChunk),
            Span::new(missing_at, missing_at),
        ));
    };
    let item = peek.view();
    let span = item.location;
    match &item.item {
        ChunkContentItem::NonBracket(token) if token.0 == NonBracketTokenKind::Dollar => {
            peek.commit();
            let name = expect_token(
                items,
                NonBracketTokenKind::Identifier,
                Expectation::Token(NonBracketTokenKind::Identifier),
                span.end,
            )?;
            Ok(WithSpan::new(
                NonConstantValue::Variable(VariableUse {
                    dollar: WithSpan::new(Dollar, span),
                    name: WithSpan::new(VariableName, name),
                }),
                Span::join(span, name),
            ))
        }
        ChunkContentItem::NonBracket(token) if token.0 == NonBracketTokenKind::StringLiteral => {
            peek.commit();
            Ok(WithSpan::new(NonConstantValue::String(StringValue), span))
        }
        ChunkContentItem::NonBracket(token) if token.0 == NonBracketTokenKind::IntegerLiteral => {
            // The tokenizer's regex admits only optionally-signed digit runs, so the
            // conversion fails only when the value does not fit.
            let value = match token_text(text, span).parse::<i64>() {
                Ok(value) => value,
                Err(_) => return Err(WithSpan::new(ParseError::IntegerOutOfRange, span)),
            };
            peek.commit();
            Ok(WithSpan::new(NonConstantValue::Integer(IntegerValue(value)), span))
        }
        ChunkContentItem::NonBracket(token) if token.0 == NonBracketTokenKind::Identifier => {
            let value = match token_text(text, span) {
                "true" => NonConstantValue::Boolean(BooleanValue(true)),
                "false" => NonConstantValue::Boolean(BooleanValue(false)),
                "null" => NonConstantValue::Null(NullValue),
                _ => {
                    return Err(WithSpan::new(
                        ParseError::expected(
                            Expectation::Value,
                            Found::Token(NonBracketTokenKind::Identifier),
                        ),
                        span,
                    ));
                }
            };
            peek.commit();
            Ok(WithSpan::new(value, span))
        }
        ChunkContentItem::Group(group) if group.opening.item.0 == BracketKind::Brace => {
            let object = ObjectLiteral(parse_level_items(
                &group.children.item,
                Expectation::ObjectEntry,
                |chunk| parse_object_entry(text, chunk),
                ObjectEntry::Unparsed,
            ));
            peek.commit();
            Ok(WithSpan::new(NonConstantValue::Object(object), span))
        }
        other => Err(WithSpan::new(
            ParseError::expected(Expectation::Value, Found::from(other)),
            span,
        )),
    }
}
```

`token_text` in parse_iso_literal.rs becomes `pub(crate)`.

## Changes to selections.rs

Both selection structs gain the argument slot, wrapped for the two containers:

```rust
// from crates/isograph_parser/src/selections.rs
pub struct ScalarSelection {
    #[resolve_field(parent_variant = Scalar)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Scalar)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field(parent_variant = Scalar)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

pub struct ObjectSelection {
    #[resolve_field(parent_variant = Object)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Object)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field(parent_variant = Object)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field(parent_variant = Object)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

`parse_selection` threads `text` and consumes the arguments between the name and the selection set. Before:

```rust
// from crates/isograph_parser/src/selections.rs
fn parse_selection(chunk: &WithSpan<Chunk>) -> Result<Selection, WithSpan<ParseError>> {
    // ...
    let selection_set = consume_selection_set(&mut items);
    expect_chunk_end(&mut items, Expectation::Separator)?;
    Ok(match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection { reader_alias, name }),
    })
}
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
fn parse_selection(text: &str, chunk: &WithSpan<Chunk>) -> Result<Selection, WithSpan<ParseError>> {
    // ... the alias-and-name parse is unchanged ...
    let arguments = consume_argument_list(text, &mut items);
    let selection_set = consume_selection_set(text, &mut items);
    expect_chunk_end(&mut items, Expectation::Separator)?;
    Ok(match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            arguments,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection { reader_alias, name, arguments }),
    })
}
```

`consume_selection_set` and `expect_selection_set` gain the `text` parameter and pass `|chunk| parse_selection(text, chunk)` to `parse_level_items`; `parse_field` in parse_iso_literal.rs passes `text` through (its signature becomes `parse_field(text: &str, keyword: Span, items: &mut ChunkContents<'_>)`). The parse-fields.md test `arguments_do_not_parse_yet` is deleted; the suite below replaces it.

## The errors

The walk extends into arguments and values; an argument list's errors precede the nested selection set's, matching source order.

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn collect_selection_set_errors(
    selection_set: &SelectionSet,
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    for selection in &selection_set.0 {
        match &selection.item {
            Selection::Scalar(scalar) => {
                collect_argument_errors(&scalar.arguments, errors);
            }
            Selection::Object(object) => {
                collect_argument_errors(&object.arguments, errors);
                collect_selection_set_errors(&object.selection_set.item, errors);
            }
            Selection::Unparsed(unparsed) => errors.push(unparsed.reason),
        }
    }
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn collect_argument_errors(
    arguments: &Option<WithSpan<ArgumentList>>,
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    let Some(arguments) = arguments else {
        return;
    };
    for argument in &arguments.item.0 {
        match &argument.item {
            Argument::Named(named) => collect_value_errors(&named.value.item, errors),
            Argument::Unparsed(unparsed) => errors.push(unparsed.reason),
        }
    }
}

pub(crate) fn collect_value_errors(
    value: &NonConstantValue,
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    let NonConstantValue::Object(object) = value else {
        return;
    };
    for entry in &object.0 {
        match &entry.item {
            ObjectEntry::Named(named) => collect_value_errors(&named.value.item, errors),
            ObjectEntry::Unparsed(unparsed) => errors.push(unparsed.reason),
        }
    }
}
```

## The resolution surface

`IsographResolutionNode` gains:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ArgumentList(ArgumentListPath<'a>),
    NamedArgument(NamedArgumentPath<'a>),
    ArgumentName(ArgumentNamePath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableName(VariableNamePath<'a>),
    StringValue(StringValuePath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    NamedObjectEntry(NamedObjectEntryPath<'a>),
    ObjectEntryName(ObjectEntryNamePath<'a>),
```

## Generated code

The value enum delegates every variant with the parent passed through:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for NonConstantValue {
    type Parent<'a> = NonConstantValueParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        match self {
            NonConstantValue::Variable(inner) => inner.resolve(parent, position),
            NonConstantValue::String(inner) => inner.resolve(parent, position),
            NonConstantValue::Integer(inner) => inner.resolve(parent, position),
            NonConstantValue::Boolean(inner) => inner.resolve(parent, position),
            NonConstantValue::Null(inner) => inner.resolve(parent, position),
            NonConstantValue::Object(inner) => inner.resolve(parent, position),
        }
    }
}
```

A named argument mixes a bare descent (the name's parent is the argument's own path) with a wrapped one (the value's parent enum names its context):

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for NamedArgument {
    type Parent<'a> = ArgumentListPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.name.location.contains(position) {
            let new_parent = self.path(parent);
            return self.name.item.resolve(new_parent, position);
        }
        if self.value.location.contains(position) {
            let new_parent = <NonConstantValue as ::resolve_position::ResolvePosition>::Parent::Argument(self.path(parent).into());
            return self.value.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::NamedArgument(self.path(parent).into());
    }
}
```

A payload-carrying leaf resolves to itself, its unmarked payload never descended into:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for IntegerValue {
    type Parent<'a> = NonConstantValueParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        return Self::ResolvedNode::IntegerValue(self.path(parent).into());
    }
}
```

`ArgumentList`, `ObjectLiteral`, and the enums `Argument` and `ObjectEntry` expand like parse-fields.md's `SelectionSet` and `Selection`; `VariableUse` like `EntrypointDeclaration` (one marked field); `NamedObjectEntry` like `NamedArgument` with the `ObjectEntry` variant, whose payload boxes through `From<T> for Box<T>`; the remaining leaves like `EntityName`.

## Tests

Extending the parse_iso_literal.rs test module, with its existing helpers.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (test module)
    fn arguments_of(selection: &Selection) -> &WithSpan<ArgumentList> {
        let arguments = match selection {
            Selection::Scalar(scalar) => &scalar.arguments,
            Selection::Object(object) => &object.arguments,
            selection => panic!("expected a parsed selection, got {selection:?}"),
        };
        arguments.as_ref().expect("the fixture's selection carries arguments")
    }

    fn as_named_argument(argument: &Argument) -> &NamedArgument {
        match argument {
            Argument::Named(named) => named,
            argument => panic!("expected a named argument, got {argument:?}"),
        }
    }

    #[test]
    fn arguments_parse_on_scalar_and_object_selections() {
        let text = "field Query.Foo { pet(id: $petId) { name(shouted: true) } }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let outer_selection = &selections(&as_field(&parse).selection_set)[0].item;
        let outer = arguments_of(outer_selection);
        assert_eq!(outer.location, span_of(text, "(id: $petId)"));
        assert_eq!(as_named_argument(&outer.item.0[0].item).name.location, span_of(text, "id"));
        let object = as_object(outer_selection);
        let inner_selection = &selections(&object.selection_set)[0].item;
        let inner = arguments_of(inner_selection);
        assert_eq!(inner.location, span_of(text, "(shouted: true)"));
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"field Query.Foo { bar(a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null) }"#;
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let arguments = arguments_of(&selections(&as_field(&parse).selection_set)[0].item);
        let values: Vec<&NonConstantValue> = arguments
            .item
            .0
            .iter()
            .map(|argument| &as_named_argument(&argument.item).value.item)
            .collect();
        assert!(matches!(values[0], NonConstantValue::Variable(_)));
        assert!(matches!(values[1], NonConstantValue::String(_)));
        assert!(matches!(values[2], NonConstantValue::Integer(IntegerValue(42))));
        assert!(matches!(values[3], NonConstantValue::Integer(IntegerValue(-7))));
        assert!(matches!(values[4], NonConstantValue::Boolean(BooleanValue(true))));
        assert!(matches!(values[5], NonConstantValue::Boolean(BooleanValue(false))));
        assert!(matches!(values[6], NonConstantValue::Null(_)));
        assert_eq!(
            as_named_argument(&arguments.item.0[0].item).value.location,
            span_of(text, "$x")
        );
    }

    #[test]
    fn object_literal_values_nest() {
        let text = "field Query.Foo { bar(input: { id: 4, nested: { on: true } }) }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let arguments = arguments_of(&selections(&as_field(&parse).selection_set)[0].item);
        let value = &as_named_argument(&arguments.item.0[0].item).value;
        assert_eq!(value.location, span_of(text, "{ id: 4, nested: { on: true } }"));
        let object = match &value.item {
            NonConstantValue::Object(object) => object,
            value => panic!("expected an object literal, got {value:?}"),
        };
        assert_eq!(object.0.len(), 2);
        let nested = match &object.0[1].item {
            ObjectEntry::Named(named) => named,
            entry => panic!("expected a named entry, got {entry:?}"),
        };
        assert_eq!(nested.name.location, span_of(text, "nested"));
    }

    #[test]
    fn empty_argument_lists_parse() {
        let text = "field Query.Foo { bar() }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        assert_eq!(arguments_of(&selections(&as_field(&parse).selection_set)[0].item).item.0.len(), 0);
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_argument() {
        let text = "field Query.Foo { bar(a: 99999999999999999999, b: 1) }";
        let parse = parsed(text);
        let arguments = arguments_of(&selections(&as_field(&parse).selection_set)[0].item);
        let unparsed = match &arguments.item.0[0].item {
            Argument::Unparsed(unparsed) => unparsed,
            argument => panic!("expected an unparsed argument, got {argument:?}"),
        };
        assert_eq!(unparsed.reason.item, ParseError::IntegerOutOfRange);
        assert_eq!(unparsed.reason.location, span_of(text, "99999999999999999999"));
        as_named_argument(&arguments.item.0[1].item);
        assert_eq!(parse.item.errors(), vec![unparsed.reason]);
    }

    #[test]
    fn a_malformed_argument_degrades_that_argument_alone() {
        let text = "field Query.Foo { bar(a 1, b: 2) }";
        let parse = parsed(text);
        let arguments = arguments_of(&selections(&as_field(&parse).selection_set)[0].item);
        let unparsed = match &arguments.item.0[0].item {
            Argument::Unparsed(unparsed) => unparsed,
            argument => panic!("expected an unparsed argument, got {argument:?}"),
        };
        assert_eq!(
            unparsed.reason.item,
            expected(token(NonBracketTokenKind::Colon), Found::Token(IntegerLiteral))
        );
        assert_eq!(as_named_argument(&arguments.item.0[1].item).name.location, span_of(text, "b"));
    }

    #[test]
    fn a_non_value_identifier_is_an_error_at_the_value() {
        let text = "field Query.Foo { bar(a: yes) }";
        let parse = parsed(text);
        let arguments = arguments_of(&selections(&as_field(&parse).selection_set)[0].item);
        let unparsed = match &arguments.item.0[0].item {
            Argument::Unparsed(unparsed) => unparsed,
            argument => panic!("expected an unparsed argument, got {argument:?}"),
        };
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Value, Found::Token(Identifier))
        );
        assert_eq!(unparsed.reason.location, span_of(text, "yes"));
    }

    #[test]
    fn a_doubled_comma_between_arguments_is_an_unparsed_argument() {
        let text = "field Query.Foo { bar(a: 1,, b: 2) }";
        let parse = parsed(text);
        let arguments = arguments_of(&selections(&as_field(&parse).selection_set)[0].item);
        assert_eq!(arguments.item.0.len(), 3);
        let unparsed = match &arguments.item.0[1].item {
            Argument::Unparsed(unparsed) => unparsed,
            argument => panic!("expected an unparsed argument, got {argument:?}"),
        };
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Argument, Found::Token(Comma))
        );
    }

    #[test]
    fn value_positions_resolve_to_value_leaves() {
        let text = "field Query.Foo { bar(a: $petId, input: { id: 4 }) }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "petId")) {
            IsographResolutionNode::VariableName(name) => {
                match &name.parent.parent {
                    NonConstantValueParent::Argument(argument) => {
                        assert_eq!(argument.inner.name.location, span_of(text, "a"));
                    }
                    parent => panic!("expected an argument parent, got {parent:?}"),
                }
            }
            node => panic!("expected the variable name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "4")) {
            IsographResolutionNode::IntegerValue(value) => {
                assert_eq!(value.inner.0, 4);
                match &value.parent {
                    NonConstantValueParent::ObjectEntry(entry) => {
                        assert_eq!(entry.inner.name.location, span_of(text, "id"));
                    }
                    parent => panic!("expected an object-entry parent, got {parent:?}"),
                }
            }
            node => panic!("expected the integer leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableUse(_) => {}
            node => panic!("expected the variable use, got {node:?}"),
        }
    }
```

## Landing checklist

1. arguments.rs, the selections.rs and parse_iso_literal.rs and parse_error.rs changes, the resolution-node variants, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
