# Type annotations are `Union([T, Null])`

A missing `!` is the union member `Null`. Surface syntax is unchanged (`Foo`, `Foo!`, `[Foo!]!`). `!` is still not a node; it is peeked and advanced, and it is not on the annotation span.

```
Foo       Union([Named(Foo), Null])
Foo!      Named(Foo)
[Foo]     Union([List(Union([Named(Foo), Null])), Null])
[Foo]!    List(Union([Named(Foo), Null]))
[Foo!]    Union([List(Named(Foo)), Null])
[Foo!]!   List(Named(Foo))
```

`TypeAnnotation` is never `Null`. A union member is never a nested `Union`. `UnionVariant` is `Named` or `List`. Those are the resolve nodes. `Null` is `NullPresence` on the union. `IsographResolutionNode` has `NamedTypeAnnotation`, `ListTypeAnnotation`, and `UnionTypeAnnotation`. It does not have `Null`. Clicking `Foo` in `$x: Foo` is Named → Union. `NullPresence::Present` is on that union.

Does not depend on parse-iso-literal-entry.md. One change.

## Types

Most important first.

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    Union(UnionTypeAnnotation),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnionTypeAnnotation {
    #[resolve_field]
    #[parent_variant(Union)]
    pub variants: Vec<WithSpan<UnionVariant>>,
    pub null: NullPresence,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnionTypeAnnotationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum UnionVariant {
    Named(
        #[parent_variant(Union)]
        NamedTypeAnnotation,
    ),
    List(
        #[parent_variant(Union)]
        Box<ListTypeAnnotation>,
    ),
}

#[derive(Debug, PartialEq, Eq)]
pub enum NullPresence {
    Present,
    Absent,
}

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Union(Box<UnionTypeAnnotationPath<'a>>),
}

pub type UnionTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a UnionTypeAnnotation, TypeAnnotationParent<'a>>;
```

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    Null(Box<NullTypeAnnotation>),
}

pub struct NullTypeAnnotation(
    #[resolve_field]
    #[parent_variant(Null)]
    pub WithSpan<TypeAnnotation>,
);

pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Null(Box<NullTypeAnnotationPath<'a>>),
}

pub type NullTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NullTypeAnnotation, TypeAnnotationParent<'a>>;
```

Delete `NullTypeAnnotation` and `NullTypeAnnotationPath`. `NamedTypeAnnotation` and `ListTypeAnnotation` are unchanged.

`UnionVariant` is all-delegate. `#[parent_variant(Union)]` builds `TypeAnnotationParent::Union(parent.into())`. `parent` is `UnionTypeAnnotationPath`. The `Union` payload is `Box<UnionTypeAnnotationPath>`. Std `From<T> for Box<T>`.

`NullPresence` is not `ResolvePosition`. It is not `#[resolve_field]`.

The derive emits `From<UnionTypeAnnotationPath<'a>> for IsographResolutionNode<'a>`. Do not write that `From`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ListTypeAnnotationPath, NamedTypeAnnotationPath, NonBracketTokenPath, NullValuePath,
    ObjectEntryPath, ObjectEntrySlotPath, ObjectLiteralPath, OpenBracketPath,
    SelectableDeclarationPath, SelectableNameWrapperPath, SelectionNameWrapperPath, SelectionPath,
    SelectionSetPath, SelectionSlotPath, StringLiteralValueWrapperPath, UnparsedChunkItemsPath,
    UnionTypeAnnotationPath, ValueKeyNameWrapperPath, VariableDeclarationListPath,
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    UnionTypeAnnotation(UnionTypeAnnotationPath<'a>),
```

Before: `NullTypeAnnotationPath` in the import list and `NullTypeAnnotation(NullTypeAnnotationPath<'a>)` on the enum. `NullValue` stays; it is the value `null`.

## Parse

`parse_named_or_list` returns a private enum that cannot be `Union` or `Null`.

```rust
// from crates/isograph_parser/src/variables.rs
enum NamedOrList {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

impl NamedOrList {
    fn into_type_annotation(self) -> TypeAnnotation {
        match self {
            NamedOrList::Named(named) => TypeAnnotation::Named(named),
            NamedOrList::List(list) => TypeAnnotation::List(list),
        }
    }

    fn into_union_variant(self) -> UnionVariant {
        match self {
            NamedOrList::Named(named) => UnionVariant::Named(named),
            NamedOrList::List(list) => UnionVariant::List(list),
        }
    }
}

pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<AstError>> {
    let core = parse_named_or_list(cursor)?;
    if let Some(peek) = cursor.peek()
        && matches!(
            peek.view().item.reference(),
            ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Exclamation))
        )
    {
        peek.advance();
        return core
            .item
            .into_type_annotation()
            .with_span(core.location)
            .wrap_ok();
    }
    let location = core.location;
    TypeAnnotation::Union(UnionTypeAnnotation {
        variants: core
            .item
            .into_union_variant()
            .with_span(location)
            .wrap_vec(),
        null: NullPresence::Present,
    })
    .with_span(location)
    .wrap_ok()
}

fn parse_named_or_list(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NamedOrList>, WithSpan<AstError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::GraphQLTypeName,
        ) {
            return NamedOrList::Named(NamedTypeAnnotation {
                name: name.interned().map(EntityNameWrapper),
            })
            .wrap_ok();
        }
        if let Some(parsed) = cursor.consume_group_if(
            BracketKind::Bracket,
            SemanticToken::GraphQLTypeName,
            |cursor, children| parse_bracket_interior_type(cursor, children),
        ) {
            let parsed = parsed.item?;
            return NamedOrList::List(
                ListTypeAnnotation {
                    inner: parsed.item,
                    extra: parsed.extra,
                }
                .boxed(),
            )
            .wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}
```

Before: `parse_named_or_list` returns `WithSpan<TypeAnnotation>` as `Named` or `List`. No bang wraps `TypeAnnotation::Null(NullTypeAnnotation(core).boxed())` at `core.location`. Bang peeks, advances, and returns `core` unchanged.

Parser-produced unions are one written type and `NullPresence::Present`. `NullPresence::Absent` is a non-null union of the variants. This syntax cannot write that.

`parse_bracket_interior_type` still calls `parse_type_annotation`, so `[Pet]`'s element is already `Union { variants: [Named(Pet)], null: Present }`.

Who calls: `consume_to_target` and `parse_variable_declaration` already call `parse_type_annotation`. No other call sites.

## Mental model

```rust
// from docs-website/docs/design-docs/mental-model.md
enum Wrapper {
    Entity(Entity),
    List(Box<Wrapper>),
    Union(UnionWrapper),
}

struct UnionWrapper {
    variants: Vec<WrapperVariant>,
    null: NullPresence,
}

enum WrapperVariant {
    Entity(Entity),
    List(Box<Wrapper>),
}

enum NullPresence {
    Present,
    Absent,
}
```

Before:

```rust
// from docs-website/docs/design-docs/mental-model.md
enum Wrapper {
    Entity(Entity),
    List(Box<Wrapper>),
    Null(Box<Wrapper>),
}
```

`List` is `[W]`. `Union { variants: [W], null: Present }` is `W | null`.

```text
Foo!      ->  Entity(Foo)
Foo       ->  Union { variants: [Entity(Foo)], null: Present }
[Foo!]!   ->  List(Entity(Foo))
[Foo]     ->  Union { variants: [List(Union { variants: [Entity(Foo)], null: Present })], null: Present }
[Foo!]    ->  Union { variants: [List(Entity(Foo))], null: Present }
[Foo]!    ->  List(Union { variants: [Entity(Foo)], null: Present })
```

Before:

```text
Foo!      ->  Foo
Foo       ->  Foo | null
[Foo!]!   ->  [Foo]
[Foo]     ->  [Foo | null] | null
```

and `Null` is `W | null`.

Selectable example `to User`:

```text
This creates the selectable `User.bestFriend` whose target is Union { variants: [Entity(User)], null: Present }.
```

Before: `whose target is the named entity `User``.

Upstream selectables:

```text
Query.user       ->  Union { variants: [Entity(User)], null: Present }
User.id          ->  Entity(ID)
User.name        ->  Entity(String)
User.friends     ->  List(Entity(User))
```

Before:

```text
Query.user       ->  User | null
User.id          ->  ID
User.name        ->  String
User.friends     ->  [User]
```

## parsing-plan

```text
- `TypeAnnotation` as `Named` / `List` / `Union` with `NullPresence` on the union (isograph `Scalar` / `Plural` / `Union` with `nullable: bool` on the union; i2 names the two cases `Present` / `Absent`). `!` is peeked and is not on the span. Leftover inside `[...]` stays on `ListTypeAnnotation`.
```

Before:

```text
- `TypeAnnotation` as `Named` / `List` with `!` on the span (not `TypeAnnotationDeclaration` as `Scalar` / `Union` / `Plural`): i2 stores the written form; isograph converts from `GraphQLTypeAnnotation`.
```

## Tests

In `parse_iso_literal.rs`. The test module's `use crate::{...}` list gains `UnionVariant` and `NullPresence`. Span tests that do not mention `Null` stay (`a_to_target_accepts_every_type_annotation_form`, `a_full_field` `Person!`, `a_bracketed_target_is_a_list_annotation` `[Pet!]!` is `List` of `Named` with location `[Pet!]`, `list_types_nest_with_non_null_markers`, `a_multi_line_variable_list_parses_in_the_demo_style`, `a_bang_resolves_to_the_variable_declaration`).

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        match target.item.reference() {
            TypeAnnotation::Union(union) => {
                assert_eq!(union.null, NullPresence::Present);
                assert_eq!(union.variants.len(), 1);
                match union.variants[0].item.reference() {
                    UnionVariant::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Owner"));
                    }
                    variant => panic!("expected Named, got {variant:?}"),
                }
                assert_eq!(union.variants[0].location, span_of(text, "Owner"));
            }
            annotation => panic!("expected Union, got {annotation:?}"),
        }
```

Before, `a_field_with_to_parses_the_target_type`:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        match target.item.reference() {
            TypeAnnotation::Null(null) => {
                assert_eq!(null.0.location, span_of(text, "Owner"));
                match null.0.item.reference() {
                    TypeAnnotation::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Owner"));
                    }
                    annotation => panic!("expected Named inside Null, got {annotation:?}"),
                }
            }
            annotation => panic!("expected Null, got {annotation:?}"),
        }
```

`a_named_target_without_bang_is_null_wrapped` becomes `a_named_target_without_bang_is_a_union_with_null`. Same match as Owner, for `Pet`.

`a_named_target_with_bang_is_not_null_wrapped` stays `TypeAnnotation::Named`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        match target.item.reference() {
            TypeAnnotation::Union(outer) => {
                assert_eq!(outer.null, NullPresence::Present);
                match outer.variants[0].item.reference() {
                    UnionVariant::List(list) => {
                        let inner = list.inner.as_ref().expect("the list holds a type");
                        match inner.item.reference() {
                            TypeAnnotation::Union(elem) => {
                                assert_eq!(elem.null, NullPresence::Present);
                                assert!(matches!(elem.variants[0].item, UnionVariant::Named(_)));
                            }
                            annotation => panic!("expected Union element, got {annotation:?}"),
                        }
                    }
                    variant => panic!("expected List, got {variant:?}"),
                }
            }
            annotation => panic!("expected Union list, got {annotation:?}"),
        }
```

Before, `a_list_target_maps_graphql_nullability`:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
            TypeAnnotation::Null(outer) => match outer.0.item.reference() {
                TypeAnnotation::List(list) => {
                    let inner = list.inner.as_ref().expect("the list holds a type");
                    match inner.item.reference() {
                        TypeAnnotation::Null(elem) => {
                            assert!(matches!(elem.0.item, TypeAnnotation::Named(_)));
                        }
                        annotation => panic!("expected Null element, got {annotation:?}"),
                    }
                }
                annotation => panic!("expected List, got {annotation:?}"),
            },
            annotation => panic!("expected Null list, got {annotation:?}"),
```

`a_variable_type_without_bang_is_null_wrapped` becomes `a_variable_type_without_bang_is_a_union_with_null`: `$x: ID` is `Union { variants: [Named(ID)], null: Present }`.

`a_nested_list_target_wraps_null_at_every_layer` becomes `a_nested_list_target_is_a_union_at_every_layer`. `[[Pet]]` is Union of List of Union of List of Union of Named, each union `Present`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        match target.item.reference() {
            TypeAnnotation::Union(outer) => {
                assert_eq!(outer.null, NullPresence::Present);
                match outer.variants[0].item.reference() {
                    UnionVariant::List(list) => {
                        let inner = list.inner.as_ref().expect("the list holds a type");
                        assert!(matches!(inner.item, TypeAnnotation::Named(_)));
                    }
                    variant => panic!("expected List, got {variant:?}"),
                }
            }
            annotation => panic!("expected Union list, got {annotation:?}"),
        }
```

Before, `a_nullable_list_of_non_null_named` (`[Pet!]`): `Null(List(Named))`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("the list holds a type");
                match inner.item.reference() {
                    TypeAnnotation::Union(elem) => {
                        assert_eq!(elem.null, NullPresence::Present);
                        assert!(matches!(elem.variants[0].item, UnionVariant::Named(_)));
                    }
                    annotation => panic!("expected Union element, got {annotation:?}"),
                }
            }
            annotation => panic!("expected List, got {annotation:?}"),
```

Before, `a_non_null_list_of_nullable_named` (`[Pet]!`): `List(Null(Named))`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
                EntityNameWrapperParent::NamedTypeAnnotation(named) => match named.parent {
                    TypeAnnotationParent::Union(union) => match union.as_ref().parent.reference() {
                        TypeAnnotationParent::SelectableDeclaration(_) => {}
                        parent => panic!("expected the field as type parent, got {parent:?}"),
                    },
                    parent => panic!("expected Union, got {parent:?}"),
                },
```

Before, `to_and_the_target_resolve_with_their_ancestry` on `Owner`: `TypeAnnotationParent::Null` then `SelectableDeclaration`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
                let named = match name.parent.reference() {
                    EntityNameWrapperParent::NamedTypeAnnotation(named) => named,
                    parent => panic!("expected a named type annotation, got {parent:?}"),
                };
                let inner_union = match named.parent.reference() {
                    TypeAnnotationParent::Union(union) => union.as_ref(),
                    parent => panic!("expected Union around Named, got {parent:?}"),
                };
                assert_eq!(inner_union.inner.null, NullPresence::Present);
                let list = match inner_union.parent.reference() {
                    TypeAnnotationParent::List(list) => list.as_ref(),
                    parent => panic!("expected a list parent, got {parent:?}"),
                };
                let outer_union = match list.parent.reference() {
                    TypeAnnotationParent::Union(union) => union.as_ref(),
                    parent => panic!("expected Union around List, got {parent:?}"),
                };
                assert_eq!(outer_union.inner.null, NullPresence::Present);
                match outer_union.parent.reference() {
                    TypeAnnotationParent::Variable(variable) => {
                        assert_eq!(variable.inner.name.location, span_of(text, "$pets"));
                    }
                    parent => panic!("expected the declared variable, got {parent:?}"),
                }
```

Before, `type_names_resolve_through_their_annotation_ancestry`: Named → Null → List → Null → Variable.

`to_as_a_target_type_name_parses` (`to to`): `Union { variants: [Named(to)], null: Present }` instead of `Null(Named(to))`.

`a_line_break_inside_a_list_type_does_not_attach_bang`: chunk 0 is `Union { variants: [Named(Pet)], null: Present }` instead of `Null(Named(Pet))`.
