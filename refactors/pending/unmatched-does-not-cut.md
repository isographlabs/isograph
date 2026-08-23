# Unmatched brackets do not cut later siblings

An unmatched `)` or an unclosed `(` in a closed group is a split in that level. Items after it stay in the tree, become their own chunks, and parse as their own selections. `field Query.Foo { a\n)\nb }` and `field Query.Foo { a\n(\nb }` both highlight `a` and `b` as `FieldName`. The unmatched token highlights as `Bracket`.

unclosed-group-recovery.md synthesizes a close so the unclosed group survives with its interior. That nests `b` inside the `(` of `{ a\n(\nb }`, so `b` is leftover `Content` of a failed selection, not a sibling `FieldName`. This change takes the unclosed group apart instead. `field Query.Foo { bar` (no close on the selection set) still takes the brace apart onto the root, so `bar` is leftover of the header; highlighting that `bar` as a selection remains unclosed-group-recovery.md.

## What the user sees

`field Query.Foo { a\n)\nb }`: `a` and `b` are field names, `)` is a bracket. The `)` is `UnmatchedClose`. Same with spaces: `field Query.Foo { a ) b }`.

`field Query.Foo { a\n(\nb }`: `a` and `b` are field names, `(` is a bracket. The `(` is `UnmatchedOpen`. The `}` still closes the selection set.

`field Query.Foo { ) b }`: `b` is a field name.

`field Query.Foo { bar(a: }`: `bar` and `a` are field names. The `(` never closed, so it is not an argument list; `a` is a sibling selection and `:` is leftover on that selection.

## Change 1: the matcher keeps unmatched items and does not cut

### Types

`BracketItem` gains unmatched open and close. `OpenBracket` / `CloseBracket` stay the payloads; the wrapping `WithSpan` on the item is the token's span. `Emission` is deleted. Errors are collected from the tree, so `parse_bracket_items` and `parse_bracketed` no longer take `emit`.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Raw(NonBracketToken),
    Bracketed(Bracketed),
    UnmatchedOpen(OpenBracket),
    UnmatchedClose(CloseBracket),
}
```

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed,
}

enum Emission {
    Emitting,
    Cut,
}
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed(UnclosedGroup),
}

struct UnclosedGroup {
    opening: WithSpan<OpenBracket>,
    children: Vec<WithSpan<BracketItem>>,
}
```

`BracketError` is unchanged. Its comment no longer says the tree cannot represent unmatched brackets.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// The matcher's errors, derived from unmatched items in the tree, in source order.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BracketError {
    /// An open bracket whose close never came.
    #[error("Unclosed {}", .0.item.0)]
    UnmatchedOpen(WithSpan<OpenBracket>),
    /// A close bracket no enclosing group owns.
    #[error("Unexpected {}", .0.item.0)]
    UnmatchedClose(WithSpan<CloseBracket>),
}
```

`strip_captured_line_breaks` still runs only on a closed group. An unclosed group's children keep leading line breaks; they sit at the enclosing level after take-apart.

### `errors`

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl MatchedBrackets {
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(self, &mut errors);
        errors
    }
}

fn collect_errors(level: &MatchedBrackets, errors: &mut Vec<BracketError>) {
    for item in &level.0 {
        match item.item.reference() {
            BracketItem::Raw(_) => {}
            BracketItem::Bracketed(group) => collect_errors(group.children.item.reference(), errors),
            BracketItem::UnmatchedOpen(open) => {
                errors.push(BracketError::UnmatchedOpen(
                    (*open).with_span(item.location),
                ));
            }
            BracketItem::UnmatchedClose(close) => {
                errors.push(BracketError::UnmatchedClose(
                    (*close).with_span(item.location),
                ));
            }
        }
    }
}
```

`match_brackets` drops the `errors` vec and the sort. It builds the tree, then returns `tree.item.errors()`.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub(crate) fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
    let mut tokens = tokens.into_iter().safe_peekable();
    let mut enclosing_stack = Stack::new();
    let items = parse_bracket_items(&mut tokens, &mut enclosing_stack);
    let tree = MatchedBrackets(items).with_span(Span::new(0, literal_length));
    let errors = tree.item.errors();
    (tree, errors)
}
```

### `parse_bracket_items` / `parse_bracketed`

An unmatched close is an item and the loop continues. An unclosed group becomes `UnmatchedOpen` plus its children at the enclosing level.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
fn parse_bracket_items(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    while let Some(peek) = tokens.peek() {
        match SplitToken::from(peek.view().item) {
            SplitToken::NonBracket(kind) => {
                let token = peek.commit();
                items.push(BracketItem::Raw(NonBracketToken(kind)).with_span(token.location));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                let token = peek.commit();
                let opening = OpenBracket(kind).with_span(token.location);
                match parse_bracketed(tokens, enclosing_stack, opening) {
                    ParsedGroup::Closed(group) => {
                        let span = Span::join(group.opening.location, group.closing.location);
                        items.push(BracketItem::Bracketed(group).with_span(span));
                    }
                    ParsedGroup::Unclosed(UnclosedGroup { opening, children }) => {
                        items.push(
                            BracketItem::UnmatchedOpen(opening.item).with_span(opening.location),
                        );
                        items.extend(children);
                    }
                }
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing_stack.all().contains(kind.reference()) {
                    break;
                }
                let token = peek.commit();
                items.push(
                    BracketItem::UnmatchedClose(CloseBracket(kind)).with_span(token.location),
                );
            }
        }
    }
    items
}

fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    opening: WithSpan<OpenBracket>,
) -> ParsedGroup {
    let children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_bracket_items(tokens, enclosing_stack)
    });
    match tokens.peek() {
        Some(peek)
            if SplitToken::from(peek.view().item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            let token = peek.commit();
            let closing = CloseBracket(opening.item.0).with_span(token.location);
            let interior = Span::between(opening.location, closing.location);
            let mut children = children;
            strip_captured_line_breaks(&mut children);
            ParsedGroup::Closed(Bracketed {
                opening,
                children: MatchedBrackets(children).with_span(interior),
                closing,
            })
        }
        _ => ParsedGroup::Unclosed(UnclosedGroup { opening, children }),
    }
}
```

### Matcher tests

Add `unmatched_open` / `unmatched_close` helpers next to `raw` / `group`.

`{ foo, bar) }` children are `foo`, comma, `bar`, `UnmatchedClose(Parenthesis)` at `)`. Rename `a_stray_close_is_a_raw_item_inside_the_brace` to `a_stray_close_is_an_unmatched_item_after_the_preceding_tokens`.

`foo { ( }` brace children are one `UnmatchedOpen(Parenthesis)` at `(`. Rename `an_unclosed_open_is_a_raw_item_inside_the_brace` to `an_unclosed_open_is_an_unmatched_item_inside_the_brace`.

`foo { (} )` root is `foo`, the brace, `UnmatchedClose(Parenthesis)` at `)`. Brace children are `UnmatchedOpen(Parenthesis)` at `(`. Root length 3.

`foo { ( } }` root is `foo`, the brace, `UnmatchedClose(Brace)` at the last `}`. Brace children are `UnmatchedOpen(Parenthesis)`. Root length 3.

`foo { bar(a: }` brace children are `bar`, `UnmatchedOpen(Parenthesis)` at `(`, `a`, `:`.

`a { b { c }` root is `a`, `UnmatchedOpen(Brace)` at the first `{`, `b`, the inner `{ c }` group.

`( } )` parenthesis children are `UnmatchedClose(Brace)` at `}`.

`{ a\n)\nb }` brace children are `a`, line break, `UnmatchedClose(Parenthesis)`, line break, `b`.

`{ a\n(\nb }` brace children are `a`, line break, `UnmatchedOpen(Parenthesis)`, line break, `b`.

## Change 2: chunking splits on unmatched items and drops them

Unmatched items are not `ChunkContentItem`. In a `ChunkedLevel` they end the current chunk and are consumed with following line breaks, the same absorption as `CommaWithoutItem`, without a `CommaWithoutItem` error. The next content item opens a new chunk. The root is unpartitioned, so unmatched items are omitted and the following items stay in the sequence.

`to_content` becomes `content_of` and returns `None` for unmatched.

```rust
// from crates/isograph_parser/src/chunk.rs
fn content_of(
    item: &WithSpan<BracketItem>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<ChunkContentItem> {
    match item.item.reference() {
        BracketItem::Raw(token) => ChunkContentItem::NonBracket(*token).wrap_some(),
        BracketItem::Bracketed(group) => {
            ChunkContentItem::Group(chunk_group(group, errors)).wrap_some()
        }
        BracketItem::UnmatchedOpen(_) | BracketItem::UnmatchedClose(_) => None,
    }
}
```

`chunk`:

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn chunk(
    tree: &WithSpan<MatchedBrackets>,
) -> (WithSpan<ChunkedRoot>, Vec<CommaWithoutItem>) {
    let mut errors = Vec::new();
    let contents = tree
        .item
        .0
        .iter()
        .filter_map(|item| {
            content_of(item, &mut errors).map(|content| content.with_span(item.location))
        })
        .collect();
    (ChunkedRoot(contents).with_span(tree.location), errors)
}
```

`as_content` calls `content_of` and also returns `None` when `separator_of` is `Some`, as today.

`absorb_chunk` first-item loop treats unmatched like a leading comma: commit, `drain_dropped_boundary`, retry. After that loop the peek is content; `content_of` is `Some`. If it is `None`, commit and `return absorb_chunk(items, errors)` so the match on `BracketItem` stays exhaustive without `unreachable`.

```rust
// from crates/isograph_parser/src/chunk.rs
        match separator_of(peek.view()) {
            Some(SeparatorToken::Comma) => {
                let comma = peek.commit().location;
                drain_dropped_boundary(items);
                return Absorbed::CommaWithoutItem(comma).wrap_some();
            }
            Some(SeparatorToken::LineBreak) => {
                peek.commit();
            }
            None => match peek.view().item.reference() {
                BracketItem::UnmatchedOpen(_) | BracketItem::UnmatchedClose(_) => {
                    peek.commit();
                    drain_dropped_boundary(items);
                }
                _ => break peek,
            },
        }
```

Content phase already stops when `as_content` is `None`. Unmatched is not a `SeparatorToken`, so it ends the content phase and is not absorbed as a trailing separator. The next `absorb_chunk` sees it as the first item and drops it.

### Chunk tests

`a ) b` root contents are `a` and `b`. Rename `an_unmatched_close_rides_inside_a_chunk_and_errors_stay_on_the_bracket_tree` to `an_unmatched_close_at_root_does_not_drop_the_following_item`.

`foo { bar` root contents are `foo` and `bar`. `an_unclosed_brace_demotes_to_raw_items_at_the_top` already names take-apart; the assertion gains `bar`.

`a_dropped_close_and_the_text_after_it_resolve_to_the_root_level`: `)` still resolves to `ChunkedRoot`. `b` resolves to `NonBracketToken` at `b`. Rename to `a_dropped_close_resolves_to_the_root_level`.

`a_dropped_open_inside_a_matched_brace_resolves_to_the_interior_level` is unchanged: `(` is dropped in chunking, position answers the brace interior.

New, through `chunk` (not `chunked`, which requires empty bracket errors):

`{ a ) b }`: brace interior two chunks, `a` then `b`.

`{ a\n)\nb }`: brace interior two chunks, `a` then `b`.

`{ a\n(\nb }`: brace interior two chunks, `a` then `b`.

`{ ) b }`: one chunk `b`.

`{ a ) }`: one chunk `a`.

`{ (\nb }`: one chunk `b`.

`{ a ) ) b }`: two chunks, `a` then `b`.

`foo { bar(a: }`: brace interior two chunks, `bar` then `a:`.

## Change 3: leftover `Bracket` tokens and grammar tests

Unmatched items never enter a chunk, so parse never `commit`s them. After `parse_chunked_iso_literal`, record `leftover_token` for each `BracketError` span that is not already in `tokens`, then sort by `location.start` so `assert_semantic_tokens` sees source order.

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn record_bracket_error_tokens(
    tokens: &mut Vec<WithSpan<IsographSemanticToken>>,
    errors: &[BracketError],
) {
    for error in errors {
        let (kind, location) = match error {
            BracketError::UnmatchedOpen(open) => (
                SplitToken::Bracket(BracketToken::Open(open.item.0)),
                open.location,
            ),
            BracketError::UnmatchedClose(close) => (
                SplitToken::Bracket(BracketToken::Close(close.item.0)),
                close.location,
            ),
        };
        record_leftover_span(tokens, leftover_token(kind), location);
    }
}
```

`record_leftover_span` stays as it is. `leftover_token` on a bracket is `Bracket`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str) -> ParsedIsoLiteral {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors: Vec<WithSpan<ParseError>> = bracket_errors
        .iter()
        .map(|error| {
            let location = match error {
                BracketError::UnmatchedOpen(open) => open.location,
                BracketError::UnmatchedClose(close) => close.location,
            };
            error.clone().to::<ParseError>().with_span(location)
        })
        .collect();
    errors.extend(
        comma_errors
            .into_iter()
            .map(|error| error.to::<ParseError>().with_span(error.0)),
    );
    let mut ast_errors = Vec::new();
    let mut tokens = Vec::new();
    let item = parse_chunked_iso_literal(text, tree, &mut ast_errors, &mut tokens);
    errors.extend(
        ast_errors
            .into_iter()
            .map(|error| error.item.to::<ParseError>().with_span(error.location)),
    );
    record_bracket_error_tokens(&mut tokens, &bracket_errors);
    tokens.sort_by_key(|token| token.location.start);
    ParsedIsoLiteral {
        item,
        errors,
        tokens,
    }
}
```

`leftover_token`'s comment names extra, leftover chunks, and unmatched brackets.

Export `record_bracket_error_tokens` from `lib.rs` next to the other `pub(crate) use chunk::{...}` names.

semantic-tokens.md leftover fill-in that walks `tokenize` still runs later. Spans already in the vec are skipped.

### Parse tests

`the_cut_removes_an_unmatched_bracket_and_the_declaration_parses` keeps parsing the entrypoint. Expected tokens gain `(IsographSemanticToken::Bracket, ")")` or `"("`. Rename to `an_unmatched_bracket_after_the_declaration_does_not_drop_the_declaration`.

`a_stray_close_is_a_parse_error_and_the_declaration_parses` gains the same `Bracket` token for `)`.

New tests use `parsed_with_errors`. Each asserts two selections, `FieldName` on both names, `Bracket` on the unmatched token, and the `ParseError::Bracket` at that token.

`field Query.Foo { a\n)\nb }`: selections `a`, `b`. Tokens: `field`, `Query`, `.`, `Foo`, `{`, `a`, `)`, `b`, `}`.

`field Query.Foo { a ) b }`: same tokens, spaces instead of line breaks.

`field Query.Foo { a\n(\nb }`: selections `a`, `b`. Tokens: `field`, `Query`, `.`, `Foo`, `{`, `a`, `(`, `b`, `}`.

`field Query.Foo { ) b }`: one selection `b`.

`field Query.Foo { a ) ) b }`: selections `a`, `b`. Two `Bracket` `)` tokens in source order.

`field Query.Foo { bar(a: }`: selections `bar`, `a`. `a`'s extra is `:`. Tokens include `FieldName` `bar`, `Bracket` `(`, `FieldName` `a`, `Content` `:`.
