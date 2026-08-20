# Null on `TypeAnnotation`

GraphQL `T` is `T | null`. GraphQL `T!` is `T`. The bang is not a node; it is the absence of `Null`. Mental model `Wrapper` is `Entity` / `List` / `Null`. `TypeAnnotation` gains `Null`.

```
Foo!      ->  Foo
Foo       ->  Foo | null
[Foo!]!   ->  [Foo]
[Foo]     ->  [Foo | null] | null
[Foo!]    ->  [Foo] | null
[Foo]!    ->  [Foo | null]
```

Does not depend on the combined `parse_iso_literal` entry. `TypeAnnotationParent::Variable` is `VariableDeclarationPath`.

One change: types, parse, resolve node, tests.

## Types

Most important first.

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    Null(Box<NullTypeAnnotation>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullTypeAnnotation(
    #[resolve_field]
    #[parent_variant(Null)]
    pub WithSpan<TypeAnnotation>,
);

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Null(Box<NullTypeAnnotationPath<'a>>),
}

pub type NullTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NullTypeAnnotation, TypeAnnotationParent<'a>>;
```

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
}
```

`NamedTypeAnnotation` and `ListTypeAnnotation` are unchanged. `!` is still recorded as `SemanticToken::GraphQLTypeName`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ListTypeAnnotationPath, NamedTypeAnnotationPath, NullTypeAnnotationPath,
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    NullTypeAnnotation(NullTypeAnnotationPath<'a>),
```

Before: no `NullTypeAnnotationPath` import and no `NullTypeAnnotation` variant.

`NullTypeAnnotation` uses the same `resolved_node = IsographResolutionNode<'a>` as `NamedTypeAnnotation` and `ListTypeAnnotation`. The derive emits `From<NullTypeAnnotationPath<'a>> for IsographResolutionNode<'a>`. Do not write that `From`.

## Parse

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    let core = parse_named_or_list(cursor)?;
    match cursor.consume_token_if(
        NonBracketTokenKind::Exclamation,
        SemanticToken::GraphQLTypeName,
    ) {
        Some(bang) => core
            .item
            .with_span(Span::new(core.location.start, bang.location.end))
            .wrap_ok(),
        None => {
            let location = core.location;
            TypeAnnotation::Null(NullTypeAnnotation(core).boxed())
                .with_span(location)
                .wrap_ok()
        }
    }
}

fn parse_named_or_list(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::GraphQLTypeName,
        ) {
            return TypeAnnotation::Named(NamedTypeAnnotation {
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
            return TypeAnnotation::List(
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

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::GraphQLTypeName,
        ) {
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::Named(NamedTypeAnnotation {
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
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::List(
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

`parse_named_or_list` is that body without the bang consume. `parse_type_annotation` wraps `Null` when there is no bang, and extends the core span over the bang when there is one.

`parse_bracket_interior_type` still calls `parse_type_annotation`, so `[Pet]`'s element is already `Null(Named(Pet))`.

Who calls: `consume_to_target` and `parse_variable_declaration` already call `parse_type_annotation`. No other call sites.

## Tests

In `parse_iso_literal.rs`. Existing span tests stay (`a_to_target_accepts_every_type_annotation_form`, `a_full_field` `Person!`, `list_types_nest_with_non_null_markers` `[Pet!]!` is still `List` of `Named` whose inner span is `Pet!`).

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_named_target_without_bang_is_null_wrapped() {
        let text = "field Query.Foo to Pet { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to Pet");
        assert_eq!(target.location, span_of(text, "Pet"));
        match target.item.reference() {
            TypeAnnotation::Null(null) => {
                assert_eq!(null.0.location, span_of(text, "Pet"));
                match null.0.item.reference() {
                    TypeAnnotation::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Pet"));
                    }
                    annotation => panic!("expected Named inside Null, got {annotation:?}"),
                }
            }
            annotation => panic!("expected Null, got {annotation:?}"),
        }
    }

    #[test]
    fn a_named_target_with_bang_is_not_null_wrapped() {
        let text = "field Query.Foo to Pet! { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to Pet!");
        assert_eq!(target.location, span_of(text, "Pet!"));
        match target.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Pet"));
            }
            annotation => panic!("expected Named, got {annotation:?}"),
        }
    }

    #[test]
    fn a_list_target_maps_graphql_nullability() {
        let text = "field Query.Foo to [Pet] { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet]");
        match target.item.reference() {
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
        }
    }

    #[test]
    fn a_non_null_list_of_non_null_named_is_list_of_named() {
        let text = "field Query.Foo to [Pet!]! { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet!]!");
        match target.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("the list holds a type");
                assert!(matches!(inner.item, TypeAnnotation::Named(_)));
                assert_eq!(inner.location, span_of(text, "Pet!"));
            }
            annotation => panic!("expected List, got {annotation:?}"),
        }
    }

    #[test]
    fn a_second_bang_is_leftover() {
        let text = "field Query.Foo to Pet!! { id }";
        let (parse, errors) = parsed(text);
        as_selectable(parse.reference());
        let second_bang = Span::new(span_of(text, "Pet!!").start + 4, span_of(text, "Pet!!").start + 5);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(NonBracketTokenKind::Exclamation))
                .with_span(second_bang)
                .wrap_vec(),
        );
    }
```

Existing matches on `TypeAnnotation::Named` for no-bang types, and resolve ancestry that skipped `Null`:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_field_with_to_parses_the_target_type() {
        let text = "field Pet.BestFriend to Owner { id }";
        // ...
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
    }

    fn to_as_a_target_type_name_parses() {
        let text = "field Query.Foo to to { bar }";
        // ...
        match as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes a target")
            .item
            .reference()
        {
            TypeAnnotation::Null(null) => match null.0.item.reference() {
                TypeAnnotation::Named(named) => {
                    assert_eq!(named.name.item, EntityNameWrapper("to".intern().to()));
                }
                annotation => panic!("expected Named inside Null, got {annotation:?}"),
            },
            annotation => panic!("expected Null, got {annotation:?}"),
        }
    }

    fn to_and_the_target_resolve_with_their_ancestry() {
        let text = "field Pet.BestFriend to Owner { id }";
        // ...
        match parse.resolve((), span_of(text, "Owner")) {
            IsographResolutionNode::EntityNameWrapper(name) => match name.parent {
                EntityNameWrapperParent::NamedTypeAnnotation(named) => match named.parent {
                    TypeAnnotationParent::Null(null) => match null.as_ref().parent.reference() {
                        TypeAnnotationParent::SelectableDeclaration(_) => {}
                        parent => panic!("expected the field as type parent, got {parent:?}"),
                    },
                    parent => panic!("expected Null, got {parent:?}"),
                },
                parent => panic!("expected a named type annotation, got {parent:?}"),
            },
            node => panic!("expected the type name leaf, got {node:?}"),
        }
    }

    fn a_line_break_inside_a_list_type_does_not_attach_bang() {
        let text = "field Query.Foo($pets: [Pet\n!]) { bar }";
        // ...
        match declared.type_.item.reference() {
            TypeAnnotation::Null(outer) => match outer.0.item.reference() {
                TypeAnnotation::List(list) => {
                    let inner = list.inner.as_ref().expect("chunk 0 parsed Pet");
                    match inner.item.reference() {
                        TypeAnnotation::Null(elem) => {
                            assert!(matches!(elem.0.item, TypeAnnotation::Named(_)));
                            assert_eq!(elem.0.location, span_of(text, "Pet"));
                        }
                        annotation => panic!("expected Null element, got {annotation:?}"),
                    }
                }
                annotation => panic!("expected List, got {annotation:?}"),
            },
            annotation => panic!("expected Null list, got {annotation:?}"),
        }
    }

    fn type_names_resolve_through_their_annotation_ancestry() {
        let text = "field Query.Foo($pets: [Pet]) { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "Pet")) {
            IsographResolutionNode::EntityNameWrapper(name) => {
                let named = match name.parent.reference() {
                    EntityNameWrapperParent::NamedTypeAnnotation(named) => named,
                    parent => panic!("expected a named type annotation, got {parent:?}"),
                };
                let inner_null = match named.parent.reference() {
                    TypeAnnotationParent::Null(null) => null.as_ref(),
                    parent => panic!("expected Null around Named, got {parent:?}"),
                };
                let list = match inner_null.parent.reference() {
                    TypeAnnotationParent::List(list) => list.as_ref(),
                    parent => panic!("expected a list parent, got {parent:?}"),
                };
                let outer_null = match list.parent.reference() {
                    TypeAnnotationParent::Null(null) => null.as_ref(),
                    parent => panic!("expected Null around List, got {parent:?}"),
                };
                match outer_null.parent.reference() {
                    TypeAnnotationParent::Variable(variable) => {
                        assert_eq!(variable.inner.name.location, span_of(text, "$pets"));
                    }
                    parent => panic!("expected the declared variable, got {parent:?}"),
                }
            }
            node => panic!("expected the type name leaf, got {node:?}"),
        }
    }
```

`a_bang_resolves_to_the_annotation` (`ID!`) is still `NamedTypeAnnotation` covering `!`. `list_types_nest_with_non_null_markers` and `a_bracketed_target_is_a_list_annotation` (`[Pet!]!`) stay `List` of `Named`. `a_multi_line_variable_list_parses_in_the_demo_style` (`ID !`) stays `Named`.

Degenerate: `$x: ID` is `Null(Named)`. `$x: ID!` is `Named`. `to [[Pet]]` is `Null(List(Null(List(Null(Named)))))`. `to [Pet!]` is `Null(List(Named))`. `to [Pet]!` is `List(Null(Named))`.
