# parse-arguments: argument lists and values

`parse_name_colon_value` reads `name : value`. `parse_argument` builds a `NamedArgument`. `parse_object_entry` builds an `ObjectEntry`. `consume_argument_list` reads a paren group and runs `parse_each_chunk` on its interior with `parse_argument`. An object value is a brace group whose interior uses `parse_object_entry`. `parse_value` reads a variable, a string, an integer, a boolean, null, or an object.

`consume_argument_list` has no production caller. parse-selection-sets.md is the first. Tests in `arguments.rs` call `parse_each_chunk` on a list interior and call `consume_argument_list` on a paren group. Production-only unused items take `#[cfg_attr(not(test), expect(dead_code))]`.

`ArgumentListParent` has no variants. parse-selection-sets.md adds `Scalar` and `Object`. A parent value cannot be constructed until then, so this doc's tests assert parse structure and do not resolve from an `ArgumentList`.

## Grammar

```
<Identifier> : <value>
```

```
( <pairs> )             argument list
{ <pairs> }             object value
```

```
$ <Identifier>          a variable
"..."                   a string literal (interned source slice, quotes included)
42, -7                  i64
true, false             Boolean::{True, False}
null
{ <pairs> }             object value
```

## Change 1: `Separator(BracketKind)`, `Argument`, `Value`, `ObjectEntry`, `IntegerDoesNotFitI64`

Origin: `crates/isograph_parser/src/parse_error.rs` and `crates/isograph_parser/src/non_bracket_token.rs`. Delta: `Separator` carries the group's `BracketKind` so leftover can name the closer. `Argument`, `Value`, and `ObjectEntry` land for `parse_name_colon_value` and `parse_value`. `IntegerDoesNotFitI64` is the `parse::<i64>()` `Err` on an `IntegerLiteral` token. `BracketKind::closing` is the closer string.

```rust
// from crates/isograph_parser/src/parse_error.rs
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
    #[error("This declaration type is not supported yet.")]
    UnsupportedDeclarationType,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a comma, a line break, or {}", .0.closing())]
    Separator(BracketKind),
    #[error("an argument, like 'id: $id'")]
    Argument,
    #[error("a value, like $foo, 42, \"bar\", true, false, null, or an object literal")]
    Value,
    #[error("an object entry, like 'id: 4'")]
    ObjectEntry,
}
```

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
impl BracketKind {
    pub fn closing(self) -> &'static str {
        match self {
            BracketKind::Parenthesis => "')'",
            BracketKind::Brace => "'}'",
            BracketKind::Bracket => "']'",
        }
    }
}
```

Before:

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("a comma or line break")]
    Separator,
```

`BracketKind` has no `closing`. `ParseError` has no `IntegerDoesNotFitI64`.

Call sites that construct `Expectation::Separator` pass a `BracketKind`. `parse_each_chunk` leftover in `chunk.rs` tests is `Separator(BracketKind::Parenthesis)`. The `chunk_stream.rs` end-span test is the same.

```rust
// from crates/isograph_parser/src/chunk.rs
        let items = tree
            .item
            .parse_each_chunk(
                parent.cursor(),
                Separator(BracketKind::Parenthesis),
                parse_identifier,
            );
```

```rust
// from crates/isograph_parser/src/chunk.rs
            expected(Separator(BracketKind::Parenthesis), Found::Token(Identifier))
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
            cursor.expected(Expectation::Separator(BracketKind::Parenthesis)),
            expected(
                Expectation::Separator(BracketKind::Parenthesis),
                Found::EndOfChunk,
            )
```

Before those three call sites used `Separator` with no payload.

A missing pair name is `Expectation::Argument` in a paren list and `Expectation::ObjectEntry` in an object. Pair leftover is `Expectation::Separator` of that group's kind. `parse_each_chunk` does not report a trailing comma; chunking already absorbed it.

## Change 2: `#[from_container_parent]` on a struct field

Origin: `get_resolve_field_info` and `new_parent_expr` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: a struct field may take `#[from_container_parent]`. The child's parent is `From::from(self.path(parent))`. Enum payloads keep the live emission.

Before, a struct field with that attribute is a compile error:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = from_container_parent {
                parse_from_container_parent(attr)?;
                return Error::new_spanned(
                    attr,
                    "`#[from_container_parent]` is an enum-payload attribute",
                )
                .to_compile_error()
                .wrap_err();
            }
            match parent_variant {
                Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                None => ParentConstruction::ContainerPath,
            }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    Transparent,
}
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::ContainerPath => quote!(self.path(parent)),
        ParentConstruction::EnumVariant(variant) => quote!(
            <#inner_type as ::resolve_position::ResolvePosition>::Parent::#variant(self.path(parent).into())
        ),
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    FromContainer,
    Transparent,
}
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = from_container_parent {
                parse_from_container_parent(attr)?;
                ParentConstruction::FromContainer
            } else {
                match parent_variant {
                    Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                    None => ParentConstruction::ContainerPath,
                }
            }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::ContainerPath => quote!(self.path(parent)),
        ParentConstruction::EnumVariant(variant) => quote!(
            <#inner_type as ::resolve_position::ResolvePosition>::Parent::#variant(self.path(parent).into())
        ),
        ParentConstruction::FromContainer => {
            quote!(::std::convert::From::from(self.path(parent)))
        }
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
```

`field_resolved_node_predicates` adds a `From` bound for `FromContainer` and does not add the `Parent` equality that `ContainerPath` uses:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        if matches!(info.parent_construction, ParentConstruction::ContainerPath) {
            predicates.push(quote! {
                #inner_type: ::resolve_position::ResolvePosition<
                    Parent<'a> = ::resolve_position::PositionResolutionPath<
                        &'a #struct_name #ty_generics,
                        #parent_type
                    >
                >
            });
        }
        if matches!(info.parent_construction, ParentConstruction::FromContainer) {
            predicates.push(quote! {
                <#inner_type as ::resolve_position::ResolvePosition>::Parent<'a>:
                    ::std::convert::From<
                        ::resolve_position::PositionResolutionPath<
                            &'a #struct_name #ty_generics,
                            #parent_type
                        >
                    >
            });
        }
```

Origin for the test: `crates/resolve_position_macros/tests/self_type_generics_pins.rs`. Delta: `E` is one type whose parent is an enum of the two slot paths, and `extra_tokens` takes `#[from_container_parent]`.

```rust
// from crates/resolve_position_macros/tests/from_container_parent_field.rs
#![expect(dead_code)]

use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    ListA(PathA<'a>),
    ListB(PathB<'a>),
    SlotA(SlotAPath<'a>),
    SlotB(SlotBPath<'a>),
    ChildA(PositionResolutionPath<&'a ChildA, SlotAPath<'a>>),
    ChildB(PositionResolutionPath<&'a ChildB, SlotBPath<'a>>),
    Extra(PositionResolutionPath<&'a Extra, ExtraParent<'a>>),
}

impl<'a> From<SlotAPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotAPath<'a>) -> Self {
        TestResolvedNode::SlotA(path)
    }
}

impl<'a> From<SlotBPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotBPath<'a>) -> Self {
        TestResolvedNode::SlotB(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListA(#[resolve_field] Vec<WithSpan<Slot<ChildA, Extra>>>);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListB(#[resolve_field] Vec<WithSpan<Slot<ChildB, Extra>>>);

type PathA<'a> = PositionResolutionPath<&'a ListA, ()>;
type PathB<'a> = PositionResolutionPath<&'a ListB, ()>;
type SlotAPath<'a> = PositionResolutionPath<&'a Slot<ChildA, Extra>, PathA<'a>>;
type SlotBPath<'a> = PositionResolutionPath<&'a Slot<ChildB, Extra>, PathB<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    resolved_node = TestResolvedNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<ChildA, Extra>, PathA<'a>),
        (<ChildB, Extra>, PathB<'a>),
    ]
)]
struct Slot<T, E> {
    #[resolve_field]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    #[from_container_parent]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotAPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildA;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotBPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildB;

#[derive(Debug)]
enum ExtraParent<'a> {
    SlotA(SlotAPath<'a>),
    SlotB(SlotBPath<'a>),
}

impl<'a> From<SlotAPath<'a>> for ExtraParent<'a> {
    fn from(path: SlotAPath<'a>) -> Self {
        ExtraParent::SlotA(path)
    }
}

impl<'a> From<SlotBPath<'a>> for ExtraParent<'a> {
    fn from(path: SlotBPath<'a>) -> Self {
        ExtraParent::SlotB(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ExtraParent<'a>, resolved_node = TestResolvedNode<'a>)]
struct Extra;

#[test]
fn leftover_parent_is_the_slot_path_through_from() {
    let list = ListA(
        Slot {
            item: ChildA.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(6, 7)) {
        TestResolvedNode::Extra(path) => match path.parent {
            ExtraParent::SlotA(slot) => {
                assert!(std::ptr::eq(slot.inner, list.0[0].item.reference()));
            }
            parent => panic!("expected ExtraParent::SlotA, got {parent:?}"),
        },
        node => panic!("expected Extra, got {node:?}"),
    }
    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::SlotA(_) => {}
        node => panic!("expected SlotA, got {node:?}"),
    }
}
```

`cargo test -p resolve_position_macros` passes.

## Change 3: the `NamedArgument` and `ObjectEntry` pins, `arguments.rs`

Origin: `Slot` and `UnparsedChunkItems` in `crates/isograph_parser/src/chunk.rs` after generic-slot.md. Delta: two pins, leftover's parent is an enum of slot paths, `extra_tokens` takes `#[from_container_parent]`. `UnparsedChunkItemsPath` moves next to that enum.

`Slot<T, E>` has one `Parent`. The lists are vanilla: each pin's parent is that list's path, each vec is bare `#[resolve_field]`. The slot items are therefore two types, `NamedArgument` and `ObjectEntry`.

Bare `#[resolve_field]` passes `self.path(parent)`. `#[parent_variant(V)]` wraps that path in variant `V` of the child's `Parent` enum. The child's `Parent` is an enum when that child appears under more than one parent. `ChunkContentItem` appears under `Chunk` and under `UnparsedChunkItems`. `ArgumentName` and `NonConstantValue` appear under `NamedArgument` and under `ObjectEntry`.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[from_container_parent]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnparsedChunkItemsParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);

#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
}

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;

impl<'a> From<IsoLiteralSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::IsoLiteralSlot(path)
    }
}

impl<'a> From<NamedArgumentSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: NamedArgumentSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::NamedArgumentSlot(path)
    }
}

impl<'a> From<ObjectEntrySlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ObjectEntrySlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ObjectEntrySlot(path)
    }
}
```

`UnparsedChunkItems` keeps `#[parent_variant(Unparsed)]`. That variant is on `ChunkContentItemParent`. The path type inside it is `UnparsedChunkItemsPath`, whose parent type is now `UnparsedChunkItemsParent`.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkContentItemParent<'a> {
    Chunk(ChunkPath<'a>),
    Unparsed(UnparsedChunkItemsPath<'a>),
}
```

Before: `UnparsedChunkItemsPath` is `PositionResolutionPath<&'a UnparsedChunkItems, IsoLiteralSlotPath<'a>>`. After: `PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>`. The `Unparsed` variant stays.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, IsoLiteralSlotPath<'a>>;
```

`IsoLiteralItem` and `EntrypointDeclaration` keep `parent_type = IsoLiteralSlotPath<'a>`. `parse_iso_literal.rs` drops the `UnparsedChunkItemsPath` alias.

The generated `extra_tokens` arm (the item arm still passes `self.path(parent)`):

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
        for item in self.extra_tokens.iter() {
            if item.location.contains(position) {
                let new_parent = ::std::convert::From::from(self.path(parent));
                return item.item.resolve(new_parent, position);
            }
        }
```

```rust
// from crates/isograph_parser/src/arguments.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, Expectation, Found, IsographResolutionNode, NonBracketTokenKind, ParseError,
    SemanticToken, Slot, UnparsedChunkItems,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListParent, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field]
    pub Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectLiteral(
    #[resolve_field]
    pub Vec<WithSpan<Slot<ObjectEntry, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedArgumentSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedArgument {
    #[resolve_field]
    #[parent_variant(NamedArgument)]
    pub name: WithSpan<ArgumentName>,
    #[resolve_field]
    #[parent_variant(NamedArgument)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectEntrySlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectEntry {
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub name: WithSpan<ArgumentName>,
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

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

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse(#[resolve_field] pub WithSpan<VariableNameWrapper>);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringValue(common_lang_types::StringLiteralValue);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BooleanValue(pub Boolean);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullValue;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentNameParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentName(common_lang_types::FieldArgumentName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableUsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(common_lang_types::VariableName);

#[derive(Debug)]
pub enum ArgumentListParent {}

#[derive(Debug)]
pub enum ArgumentNameParent<'a> {
    NamedArgument(NamedArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
}

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    NamedArgument(Box<NamedArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent>;

pub type ObjectLiteralPath<'a> =
    PositionResolutionPath<&'a ObjectLiteral, NonConstantValueParent<'a>>;

pub type NamedArgumentSlotPath<'a> = PositionResolutionPath<
    &'a Slot<NamedArgument, UnparsedChunkItems>,
    ArgumentListPath<'a>,
>;

pub type ObjectEntrySlotPath<'a> = PositionResolutionPath<
    &'a Slot<ObjectEntry, UnparsedChunkItems>,
    ObjectLiteralPath<'a>,
>;

pub type NamedArgumentPath<'a> =
    PositionResolutionPath<&'a NamedArgument, NamedArgumentSlotPath<'a>>;

pub type ObjectEntryPath<'a> = PositionResolutionPath<&'a ObjectEntry, ObjectEntrySlotPath<'a>>;

pub type VariableUsePath<'a> = PositionResolutionPath<&'a VariableUse, NonConstantValueParent<'a>>;

pub type StringValuePath<'a> = PositionResolutionPath<&'a StringValue, NonConstantValueParent<'a>>;

pub type IntegerValuePath<'a> = PositionResolutionPath<&'a IntegerValue, NonConstantValueParent<'a>>;

pub type BooleanValuePath<'a> = PositionResolutionPath<&'a BooleanValue, NonConstantValueParent<'a>>;

pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, NonConstantValueParent<'a>>;

pub type ArgumentNamePath<'a> = PositionResolutionPath<&'a ArgumentName, ArgumentNameParent<'a>>;

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableUsePath<'a>>;
```

`string_key_newtype!` already implements `From<StringKey>` for `FieldArgumentName`, `VariableName`, and `StringLiteralValue`. The parser wrappers do not add a second `From`. Construction is `name.interned().map(ArgumentName)` and `name.interned().map(VariableNameWrapper)`.

`NonConstantValueParent` variants are boxed to break `NamedArgumentPath` / `ObjectEntryPath` through `ObjectLiteral` back to a pair. A position on `$` answers `VariableUse`. `VariableNameWrapper` has one parent, `VariableUsePath`, so that field is bare `#[resolve_field]`.

`ArgumentList` and `ObjectLiteral` vecs are bare `#[resolve_field]`. `NamedArgument`'s parent is `NamedArgumentSlotPath`. `ObjectEntry`'s parent is `ObjectEntrySlotPath`. `ArgumentName` and `NonConstantValue` have two parents, so those fields take `#[parent_variant]`.

`Slot`'s unmatched arm is `on_unmatched_span = from_path`: `self.path(parent).to()`. Each pin has a `From` into `IsographResolutionNode`. The root pin is live. These pins add two more.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl<'a> From<IsoLiteralSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        IsographResolutionNode::IsoLiteralSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
impl<'a> From<NamedArgumentSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: NamedArgumentSlotPath<'a>) -> Self {
        IsographResolutionNode::NamedArgumentSlot(path)
    }
}

impl<'a> From<ObjectEntrySlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ObjectEntrySlotPath<'a>) -> Self {
        IsographResolutionNode::ObjectEntrySlot(path)
    }
}
```

Every other new node uses the derive's `struct_name` unmatched arm, `IsographResolutionNode::#name(self.path(parent).to())`. Those variants are listed below. They do not take a `From`.

Before:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
use crate::{
    ChunkPath, ChunkSeparatorPath, ChunkedGroupPath, ChunkedLevelPath, ClientFieldNamePath,
    CloseBracketPath, EntityNamePath, EntrypointDeclarationPath, ExtraChunksPath,
    IsoLiteralParsePath, IsoLiteralSlotPath, NonBracketTokenPath, OpenBracketPath,
    UnparsedChunkItemsPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the grammar tree's, with the chunk
/// tree's still surfacing inside unparsed regions.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    Singleton(IsoLiteralParsePath<'a>),
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
    UnparsedChunkItems(UnparsedChunkItemsPath<'a>),
    ExtraChunks(ExtraChunksPath<'a>),
    ChunkedLevel(ChunkedLevelPath<'a>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    ChunkedGroup(ChunkedGroupPath<'a>),
    Chunk(ChunkPath<'a>),
    ChunkSeparator(ChunkSeparatorPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
```

After. Origin: that file. Delta: the use list and the argument/value variants.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
use crate::{
    ArgumentListPath, ArgumentNamePath, BooleanValuePath, ChunkPath, ChunkSeparatorPath,
    ChunkedGroupPath, ChunkedLevelPath, ClientFieldNamePath, CloseBracketPath, EntityNamePath,
    EntrypointDeclarationPath, ExtraChunksPath, IntegerValuePath, IsoLiteralParsePath,
    IsoLiteralSlotPath, NamedArgumentPath, NamedArgumentSlotPath, NonBracketTokenPath,
    NullValuePath, ObjectEntryPath, ObjectEntrySlotPath, ObjectLiteralPath, OpenBracketPath,
    StringValuePath, UnparsedChunkItemsPath, VariableNameWrapperPath, VariableUsePath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the grammar tree's, with the chunk
/// tree's still surfacing inside unparsed regions.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    Singleton(IsoLiteralParsePath<'a>),
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
    UnparsedChunkItems(UnparsedChunkItemsPath<'a>),
    ExtraChunks(ExtraChunksPath<'a>),
    ChunkedLevel(ChunkedLevelPath<'a>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    ChunkedGroup(ChunkedGroupPath<'a>),
    Chunk(ChunkPath<'a>),
    ChunkSeparator(ChunkSeparatorPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    NamedArgument(NamedArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
    ArgumentName(ArgumentNamePath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableNameWrapper(VariableNameWrapperPath<'a>),
    StringValue(StringValuePath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
#[cfg_attr(not(test), expect(dead_code))]
fn parse_name_colon_value(
    cursor: &mut ItemCursor<'_>,
    name_token: SemanticToken,
    missing_name: Expectation,
) -> Result<(WithSpan<ArgumentName>, WithSpan<NonConstantValue>), WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_value(cursor)?;
    (
        name.interned().map(ArgumentName),
        value,
    )
        .wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<NamedArgument, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::Argument, Expectation::Argument)?;
    NamedArgument { name, value }.wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_object_entry(
    cursor: &mut ItemCursor<'_>,
) -> Result<ObjectEntry, WithSpan<ParseError>> {
    let (name, value) = parse_name_colon_value(
        cursor,
        SemanticToken::ObjectKey,
        Expectation::ObjectEntry,
    )?;
    ObjectEntry { name, value }.wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn consume_argument_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<ArgumentList>> {
    let group = cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis)?;
    let list = ArgumentList(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Parenthesis),
        parse_argument,
    ));
    cursor.record_group_close(group.item, SemanticToken::Parenthesis);
    list.with_span(group.location).wrap_some()
}

#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn parse_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if cursor
            .consume_token_if(NonBracketTokenKind::Dollar, SemanticToken::Variable)
            .is_some()
        {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
                .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
            return NonConstantValue::Variable(VariableUse(
                name.interned().map(VariableNameWrapper),
            ))
            .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        {
            return NonConstantValue::String(span.interned().map(StringValue).item)
                .wrap_ok();
        }
        if let Some(span) = cursor
            .consume_token_if(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        {
            let value = match span.token_text().parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64.with_span(span.location).wrap_err();
                }
            };
            return NonConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::BooleanOrNull,
        ) {
            return match span.token_text() {
                "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => NonConstantValue::Null(NullValue).wrap_ok(),
                _ => ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                .with_span(span.location)
                .wrap_err(),
            };
        }
        if let Some(group) =
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace)
        {
            let object = ObjectLiteral(group.item.children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_object_entry,
            ));
            cursor.record_group_close(group.item, SemanticToken::Brace);
            return NonConstantValue::Object(object).wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

`lib.rs` adds `mod arguments;` and `pub use arguments::*;`.

An entrypoint leftover token still answers `NonBracketToken`. `leftover_after_an_entrypoint_resolves_to_the_leftover_token` and `a_gap_after_the_item_resolves_to_the_slot` keep passing.

## Tests

```rust
// from crates/isograph_parser/src/arguments.rs
    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    ) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let dummy = {
            let (brackets, _) = match_brackets(tokenize("x"), 1);
            chunk(brackets.reference()).0
        };
        let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
        let items = tree
            .item
            .parse_each_chunk(parent.cursor(), leftover, parse_item);
        (items, errors, comma_errors, tokens)
    }

    fn parsed_pairs(
        text: &str,
    ) -> (
        Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<WithSpan<SemanticToken>>,
    ) {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
    }

    fn parsed_argument_list(
        text: &str,
    ) -> (
        Option<WithSpan<ArgumentList>>,
        Vec<WithSpan<ParseError>>,
        Vec<WithSpan<SemanticToken>>,
    ) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let mut stream = tree.item.0[0]
            .item
            .stream(text, &mut tokens, &mut errors);
        let list = consume_argument_list(stream.cursor());
        (list, errors, tokens)
    }

    fn as_argument(slot: &Slot<NamedArgument, UnparsedChunkItems>) -> &NamedArgument {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named argument")
    }

    fn as_entry(slot: &Slot<ObjectEntry, UnparsedChunkItems>) -> &ObjectEntry {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected an object entry")
    }

    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    #[test]
    fn pairs_parse_as_name_colon_value() {
        let text = "id: $petId, shouted: true";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert_eq!(
            as_argument(items[0].item.reference()).name.item,
            ArgumentName("id".intern().to())
        );
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, "$petId")
        );
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "shouted")
        );
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null"#;
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        let values: Vec<&NonConstantValue> = items
            .iter()
            .map(|slot| as_argument(slot.item.reference()).value.item.reference())
            .collect();
        assert!(matches!(values[0], NonConstantValue::Variable(_)));
        assert!(matches!(values[1], NonConstantValue::String(_)));
        assert!(matches!(values[2], NonConstantValue::Integer(IntegerValue(42))));
        assert!(matches!(values[3], NonConstantValue::Integer(IntegerValue(-7))));
        assert!(matches!(
            values[4],
            NonConstantValue::Boolean(BooleanValue(Boolean::True))
        ));
        assert!(matches!(
            values[5],
            NonConstantValue::Boolean(BooleanValue(Boolean::False))
        ));
        assert!(matches!(values[6], NonConstantValue::Null(_)));
    }

    #[test]
    fn object_values_use_braces() {
        let text = "input: { id: 4, nested: { on: true } }";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        let value = as_argument(items[0].item.reference()).value.reference();
        assert_eq!(
            value.location,
            span_of(text, "{ id: 4, nested: { on: true } }")
        );
        let object = match value.item.reference() {
            NonConstantValue::Object(object) => object,
            value => panic!("expected an object, got {value:?}"),
        };
        assert_eq!(object.0.len(), 2);
        assert_eq!(
            as_entry(object.0[1].item.reference()).name.location,
            span_of(text, "nested")
        );
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_pairs() {
        for text in ["", "   ", "\n"] {
            let (items, errors, _) = parsed_pairs(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_trailing_comma_after_a_pair_is_not_a_parse_error() {
        let text = "id: 1,";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_argument(items[0].item.reference()).name.item,
            ArgumentName("id".intern().to())
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_pair() {
        let text = "a: 99999999999999999999, b: 1";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        as_argument(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "99999999999999999999")
        }));
    }

    #[test]
    fn a_malformed_pair_degrades_that_pair_alone() {
        let text = "a 1, b: 2";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Token(NonBracketTokenKind::Colon),
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
        }));
    }

    #[test]
    fn a_non_value_identifier_is_an_error_at_the_value() {
        let text = "a: yes";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "yes")
        }));
    }

    #[test]
    fn a_pair_that_does_not_start_with_a_name_is_an_argument_error() {
        let text = "42: 1";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Argument,
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
                && error.location == span_of(text, "42")
        }));
    }

    #[test]
    fn leftover_after_a_pair_keeps_the_item() {
        let text = "id: $x junk";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Separator(BracketKind::Parenthesis),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "junk")
        }));
    }

    #[test]
    fn a_doubled_comma_between_pairs_is_chunkings_error() {
        let text = "a: 1,, b: 2";
        let (items, errors, comma_errors, _) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn consume_argument_list_reads_a_paren_group() {
        let text = "(id: $petId)";
        let (list, errors, tokens) = parsed_argument_list(text);
        let list = list.expect("the fixture opens with a paren group");
        assert_eq!(errors, vec![]);
        assert_eq!(list.location, span_of(text, "(id: $petId)"));
        assert_eq!(list.item.0.len(), 1);
        assert_eq!(
            as_argument(list.item.0[0].item.reference()).name.item,
            ArgumentName("id".intern().to())
        );
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Parenthesis.with_span(span_of(text, "(")),
                SemanticToken::Argument.with_span(span_of(text, "id")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::Variable.with_span(span_of(text, "$")),
                SemanticToken::Variable.with_span(span_of(text, "petId")),
                SemanticToken::Parenthesis.with_span(span_of(text, ")")),
            ],
        );
    }

    #[test]
    fn an_empty_paren_group_is_zero_pairs() {
        let text = "()";
        let (list, errors, _) = parsed_argument_list(text);
        let list = list.expect("the fixture opens with a paren group");
        assert_eq!(list.item.0.len(), 0);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn a_pair_records_argument_colon_and_value_tokens() {
        let text = "id: $petId";
        let (_, errors, tokens) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Argument.with_span(span_of(text, "id")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::Variable.with_span(span_of(text, "$")),
                SemanticToken::Variable.with_span(span_of(text, "petId")),
            ],
        );
    }
```

## Landing checklist

1. `Separator(BracketKind)`, `closing`, `Argument`, `Value`, `ObjectEntry`, `IntegerDoesNotFitI64`. Existing `Separator` call sites take a `BracketKind`. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Struct-field `#[from_container_parent]`, the macro test. `cargo test -p resolve_position_macros` passes.
3. The `NamedArgument` and `ObjectEntry` pins, `UnparsedChunkItemsParent`, `arguments.rs`, the resolution-node variants, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
4. Move this doc to refactors/past.
