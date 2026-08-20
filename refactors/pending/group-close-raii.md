# Group close RAII

`consume_group_if` and `require_group` take a function that parses the inside. Commit records the open; that function runs against the group's children; a Drop guard records the close when it returns. The result is wrapped with the group's span. `record_group_close` is deleted. Lands before parse-selection-sets.md.

One shippable change.

## The API

```rust
// from crates/isograph_parser/src/chunk_stream.rs
/// Records `token` at `closing` when dropped, however the parse function exits, a panic included.
#[cfg_attr(not(test), expect(dead_code))]
struct RecordGroupClose<'c, 'a> {
    cursor: &'c mut ItemCursor<'a>,
    closing: Span,
    token: SemanticToken,
}

impl<'c, 'a> RecordGroupClose<'c, 'a> {
    fn cursor(&mut self) -> &mut ItemCursor<'a> {
        self.cursor
    }
}

impl Drop for RecordGroupClose<'_, '_> {
    fn drop(&mut self) {
        self.cursor.record(self.token, self.closing);
    }
}
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn consume_group_if<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Option<WithSpan<R>>;
    pub(crate) fn require_group<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Result<WithSpan<R>, ()>;
}
```

`require_group` stays `consume_group_if` or `Err(())`. The caller maps `Err` with `expected`.

`parse_inside` receives the parent cursor and the group's children. It does not receive the group. `consume_group_if` wraps `R` with the group's span.

`RecordGroupClose` is private to `chunk_stream.rs`. Only `consume_group_if` constructs it. Callers never see the guard and never record a close.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.peek()?;
        let item = peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                let location = item.location;
                peek.commit(token);
                group.with_span(location).wrap_some()
            }
            _ => None,
        }
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn record_group_close(&mut self, group: &ChunkedGroup, token: SemanticToken) {
        self.record(token, group.closing.location);
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()> {
        self.consume_group_if(kind, token).ok_or(())
    }
```

After. Origin: those three methods. Delta: `consume_group_if` / `require_group` take `parse_inside`; the success arm builds `RecordGroupClose`, runs `parse_inside` on the children, and wraps with the group's span; `record_group_close` is gone.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn consume_group_if<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Option<WithSpan<R>> {
        let peek = self.peek()?;
        let item = peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                let location = item.location;
                let closing = group.closing.location;
                let children = group.children.reference();
                peek.commit(token);
                let mut close = RecordGroupClose {
                    cursor: self,
                    closing,
                    token,
                };
                parse_inside(close.cursor(), children)
                    .with_span(location)
                    .wrap_some()
            }
            _ => None,
        }
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn require_group<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Result<WithSpan<R>, ()> {
        self.consume_group_if(kind, token, parse_inside).ok_or(())
    }
```

`commit` records the open. `Drop` records the close after `parse_inside` returns. A `?` inside `parse_inside` still drops the guard. Nested `consume_group_if` in `parse_inside` drops the inner guard before the outer one: inner close, then outer close.

A token after the group is consumed after `consume_group_if` returns, so after the close.

## Call sites

### `consume_argument_list` and `parse_value`

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn consume_argument_list(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<ArgumentList>> {
    let group = cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis)?;
    let list = ArgumentList(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Parenthesis),
        parse_argument,
    ));
    cursor.record_group_close(group.item, SemanticToken::Parenthesis);
    list.with_span(group.location).wrap_some()
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace) {
            let object = ObjectLiteral(group.item.children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_object_entry,
            ));
            cursor.record_group_close(group.item, SemanticToken::Brace);
            return NonConstantValue::Object(object).wrap_ok();
        }
```

After. Origin: those two sites. Delta: `parse_inside` builds the list or object from `children`; the group span is the `WithSpan` `consume_group_if` returns; `parse_value` is inside `spanning`, so it takes `.item`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn consume_argument_list(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<ArgumentList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        SemanticToken::Parenthesis,
        |cursor, children| {
            ArgumentList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_argument,
            ))
        },
    )
}
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(object) = cursor.consume_group_if(
            BracketKind::Brace,
            SemanticToken::Brace,
            |cursor, children| {
                ObjectLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_object_entry,
                ))
            },
        ) {
            return NonConstantValue::Object(object.item).wrap_ok();
        }
```

`consume_argument_list_reads_a_paren_group` still asserts `(`, argument, colon, `$`, name, `)`.

### `require_selection_set` and `consume_selection_set`

Before:

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    let group = cursor
        .require_group(BracketKind::Brace, SemanticToken::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    let set = SelectionSet(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_selection,
    ));
    cursor.record_group_close(group.item, SemanticToken::Brace);
    set.with_span(group.location).wrap_ok()
}

fn consume_selection_set(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<SelectionSet>> {
    let group = cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace)?;
    let set = SelectionSet(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_selection,
    ));
    cursor.record_group_close(group.item, SemanticToken::Brace);
    set.with_span(group.location).wrap_some()
}
```

After. Origin: parse-selection-sets.md's Change 2 listings (those two functions). Delta: `parse_inside` builds the `SelectionSet` from `children`; `map_err` stays on `require_group`.

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    cursor
        .require_group(
            BracketKind::Brace,
            SemanticToken::Brace,
            |cursor, children| {
                SelectionSet(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_selection,
                ))
            },
        )
        .map_err(|()| cursor.expected(Expectation::SelectionSet))
}

fn consume_selection_set(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<SelectionSet>> {
    cursor.consume_group_if(
        BracketKind::Brace,
        SemanticToken::Brace,
        |cursor, children| {
            SelectionSet(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_selection,
            ))
        },
    )
}
```

### `consume_variable_declaration_list` and the list-type arm

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationList>> {
    let group = cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis)?;
    let list = VariableDeclarationList(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Parenthesis),
        parse_variable_declaration,
    ));
    cursor.record_group_close(group.item, SemanticToken::Parenthesis);
    list.with_span(group.location).wrap_some()
}
```

```rust
// from crates/isograph_parser/src/variables.rs
        if let Some(group) =
            cursor.consume_group_if(BracketKind::Bracket, SemanticToken::GraphQLTypeName)
        {
            let inner = parse_bracket_interior_type(cursor, group.item.children.reference())?;
            cursor.record_group_close(group.item, SemanticToken::GraphQLTypeName);
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::List(ListTypeAnnotation { inner: inner.item, extra_tokens: inner.extra_tokens }.boxed())
                .wrap_ok();
        }
```

After. Origin: parse-variables.md Change 3 listings (those two). Delta: `parse_inside` is the interior parse; the `!` after `[...]` stays outside so it records after the close; `parse_inside` returns `Result`, so the `WithSpan` is unwrapped before `?`.

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        SemanticToken::Parenthesis,
        |cursor, children| {
            VariableDeclarationList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_variable_declaration,
            ))
        },
    )
}
```

```rust
// from crates/isograph_parser/src/variables.rs
        if let Some(inner) = cursor.consume_group_if(
            BracketKind::Bracket,
            SemanticToken::GraphQLTypeName,
            |cursor, children| parse_bracket_interior_type(cursor, children),
        ) {
            let inner = inner.item?;
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::List(ListTypeAnnotation { inner: inner.item, extra_tokens: inner.extra_tokens }.boxed())
                .wrap_ok();
        }
```

## Tests

A call that only checks presence passes `|_, _| ()`. The return is `Option<WithSpan<()>>` / `Result<WithSpan<()>, ()>`; `.location` is the group span.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        assert_eq!(
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace),
            None
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_some(),
        );
        assert_eq!(
            cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis),
            None
        );
        let group = cursor
            .consume_group_if(BracketKind::Brace, SemanticToken::Brace)
            .expect("the next item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(group.item.opening.item.0, BracketKind::Brace);
        assert_eq!(
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace),
            None
        );
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        let group = cursor
            .require_group(BracketKind::Brace, SemanticToken::Brace)
            .expect("the first item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(
            cursor.require_group(BracketKind::Brace, SemanticToken::Brace),
            ().wrap_err(),
        );
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        cursor
            .consume_group_if(BracketKind::Brace, SemanticToken::Brace)
            .expect("the group is present");
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        stream
            .cursor()
            .require_group(BracketKind::Brace, SemanticToken::Brace)
            .expect("the chunk is a brace group");
```

After. Origin: those four tests. Delta: each call takes `parse_inside`; location comes from the returned `WithSpan`; the opening-kind assertion is gone.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        assert_eq!(
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
            None
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_some(),
        );
        assert_eq!(
            cursor.consume_group_if(
                BracketKind::Parenthesis,
                SemanticToken::Parenthesis,
                |_, _| ()
            ),
            None
        );
        let group = cursor
            .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the next item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
            None
        );
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        let group = cursor
            .require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the first item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(
            cursor.require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
            ().wrap_err(),
        );
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        cursor
            .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the group is present");
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        stream
            .cursor()
            .require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the chunk is a brace group");
```

Recording. Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn consume_group_if_records_the_open_and_record_group_close_records_the_close() {
        let text = "{ bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            let cursor = stream.cursor();
            let group = cursor
                .consume_group_if(BracketKind::Brace, SemanticToken::Brace)
                .expect("the chunk is a brace group");
            cursor.record_group_close(group.item, SemanticToken::Brace);
        }
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Brace.with_span(span_of(text, "{")),
                SemanticToken::Brace.with_span(span_of(text, "}")),
            ],
        );
    }
```

After. Origin: that test. Delta: the function name drops `record_group_close`; `parse_inside` is `|_, _| ()`; `{}` is the empty case; a failed `require_group` records nothing.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn consume_group_if_records_the_open_and_the_close() {
        for text in ["{ bar }", "{}"] {
            let tree = chunked(text);
            let mut tokens = Vec::new();
            let mut errors = Vec::new();
            {
                let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
                stream
                    .cursor()
                    .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
                    .expect("the chunk is a brace group");
            }
            assert_eq!(
                tokens,
                vec![
                    SemanticToken::Brace.with_span(span_of(text, "{")),
                    SemanticToken::Brace.with_span(span_of(text, "}")),
                ],
                "for literal {text:?}",
            );
        }
    }

    fn require_group_err_records_nothing() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(
                stream.cursor().require_group(
                    BracketKind::Brace,
                    SemanticToken::Brace,
                    |_, _| ()
                ),
                ().wrap_err(),
            );
        }
        assert_eq!(tokens, vec![]);
    }
```

## Docs this change amends

### parsing-standards.md

`ItemCursor` listing: `consume_group_if` / `require_group` take `parse_inside` as above; `record_group_close` is gone.

"A group is one item. `require_group` and `consume_group_if` return it in one call. The interior is parsed by calling `parse_each_chunk` or `parse_singleton` on `group.children`."

becomes

"A group is one item. `require_group` and `consume_group_if` take a function that parses the inside. They commit the open, run that function against the group's children, wrap the result with the group's span, and record the close when the function returns."

Span sources: "or the `WithSpan` from `require_group` or `consume_group_if`" stays; that `WithSpan` is the return, not a `ChunkedGroup`.

Function shapes: `consume_*` for a group is "Match: `commit`, run `parse_inside` on the children, record close, `Some` of that result with the group's span. Else: `None`."

"A group plus its interior" snippets, after. Origin: those two listings. Delta: `parse_inside` receives `children` and returns the interior value; the group span is the return.

```rust
    cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace, |cursor, children| {
        children.item.parse_each_chunk(
            cursor,
            Expectation::Separator(BracketKind::Brace),
            parse_item,
        )
    })
```

```rust
    cursor
        .require_group(BracketKind::Brace, SemanticToken::Brace, |cursor, children| {
            children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_item,
            )
        })
        .map_err(|()| cursor.expected(expectation))?
```

`parse_value`'s object arm becomes the after in Call sites above.

Catalog: "Group interior: `require_group` / `consume_group_if` with a function that parses the inside; close is recorded when that function returns."

### semantic-tokens.md

"Open and close of one `BracketKind` share one token. `consume_group_if(kind, token)` records `token` on the open; `record_group_close(group, token)` records the same `token` on the close."

becomes

"Open and close of one `BracketKind` share one token. `consume_group_if(kind, token, parse_inside)` records `token` on the open, runs `parse_inside`, and records the same `token` on the close when `parse_inside` returns."

### parse-selection-sets.md, parse-variables.md, parse-arrays.md

The listings in Call sites replace the current ones. parse-arrays.md's `consume_group_if(BracketKind::Bracket)` takes `parse_inside`; the list parse moves inside it. Its token argument is whatever that doc already owes.

## Landing checklist

1. `RecordGroupClose`, the new `consume_group_if` / `require_group` signatures, `record_group_close` deleted, arguments.rs call sites, chunk_stream.rs tests. `cargo test -p isograph_parser` passes.
2. Amend parsing-standards.md, semantic-tokens.md, parse-selection-sets.md, parse-variables.md, parse-arrays.md as above.
3. Move this doc to refactors/past.
