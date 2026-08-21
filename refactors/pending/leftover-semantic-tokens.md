# Leftover semantic tokens

Leftover that the grammar did not consume is highlighted by walking the tree fields that hold it. `Slot.extra` holds unread remainder of a chunk and, after leftover-in-extra.md, a singleton's trailing separator. `Singleton.extra_chunks` holds whole extra chunks after the first. After the grammar parse of a chunk returns, this change records a semantic token for each leftover token in those fields.

When the grammar consumes a token, it records the role the call site names (`Keyword`, `Type`, `FieldName`, and the rest). Leftover uses `leftover_token`, which maps a token kind to `Content`, `Integer`, `String`, `Error`, or `Bracket`.

Unmatched brackets that the matcher cut never enter a chunk. They remain `BracketError` on the pipeline error list. They get a semantic token when semantic-tokens.md leftover fill-in walks `tokenize`.

## What the user sees

`entrypoint Query.foo bar` highlights `entrypoint` as Keyword, `Query` as Type, `.` as Period, `foo` as FieldName, and `bar` as Content.

`entrypoint $ $` highlights `entrypoint` as Keyword and each `$` as Content.

`entrypoint Query.foo,` highlights the comma as Content.

`entrypoint\nQuery.foo` highlights `entrypoint` as Keyword and extra-chunk `Query` / `.` / `foo` as Content.

`fieldd Query.foo { bar }` highlights `fieldd` as Keyword, `Query` / `.` / `foo` / `bar` as Content, and `{` `}` as Bracket.

`field Query.Foo to Pet!! { id }` highlights through `Pet` as today. The first `!` is consumed as nullability and is not recorded. The second `!` is Content. `{` `}` are Bracket. `id` is Content.

A leftover `{ bar }` records Bracket on `{` and `}`, Content on `bar`. Leftover never records `Type`. `$` and leftover identifiers are Content.

`entrypoint Query.foo @lazyLoad` is a directive. `@` and `lazyLoad` are DirectiveName.

## Dependencies

This change depends on leftover-in-extra.md (in past), which puts trailing separators and unread remainder in `Slot.extra`. Tests call `parsed` (parse-test-semantic-tokens). `parse_singleton` already builds `extra_chunks`. This change does not depend on four-trees.md or type-annotation-null.md.

## Extraction

Extracted from semantic-tokens.md leftover fill-in: `leftover_token` and the facts that leftover identifiers and punctuation are `Content`, leftover integers are `Integer`, leftover strings are `String`, leftover error tokens are `Error`, leftover brackets are `Bracket`, and line breaks and EOF record nothing.

The delta from that extraction is that leftover fill-in for extra and extra_chunks walks `Slot.extra` and `Singleton.extra_chunks`, not `tokenize`. `leftover_token` is `pub(crate)` so `chunk.rs` can call it. The leftover fill-in that remains in semantic-tokens.md is still a walk of `tokenize`, covering the matcher's cut.

## Sequential search skips unrecorded occurrences

`assert_semantic_tokens` finds each expected lexeme at or after the previous token's end. After `"Pet"` in `Pet!!`, `"!"` is the first bang. `parse_type_annotation` consumed that bang with `CursorPeek::advance` and did not record it. Leftover Content is the second bang.

Search skips an occurrence whose span is not in `actual`. `(SemanticToken::Content, "!")` after `"Pet"` is the second bang.

Before:

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let offset = text[search_from..]
            .find(pattern)
            .expect("the expected lexeme occurs in the fixture after the previous token");
        let start = search_from + offset;
        let end = start + pattern.len();
        expected_tokens.push(role.with_span(Span::from_usize(start, end)));
        search_from = end;
    }
```

After:

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let span = loop {
            let offset = text[search_from..]
                .find(pattern)
                .expect("the expected lexeme occurs as a recorded token after the previous token");
            let start = search_from + offset;
            let end = start + pattern.len();
            let span = Span::from_usize(start, end);
            search_from = end;
            if actual.iter().any(|token| token.location == span) {
                break span;
            }
        };
        expected_tokens.push(role.with_span(span));
    }
```

The `assert_eq` of `actual` against `expected_tokens` is unchanged. Duplicate recorded lexemes still resolve in source order. An unrecorded occurrence of the same lexeme is skipped.

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
    #[test]
    fn sequential_search_skips_an_unrecorded_occurrence() {
        let text = "Pet!!";
        let actual = [
            SemanticToken::GraphQLTypeName.with_span(Span::from_usize(0, 3)),
            SemanticToken::Content.with_span(Span::from_usize(4, 5)),
        ];
        assert_semantic_tokens(
            text,
            actual.as_slice(),
            &[
                (SemanticToken::GraphQLTypeName, "Pet"),
                (SemanticToken::Content, "!"),
            ],
        );
    }

    #[test]
    fn sequential_search_keeps_source_order_when_both_occurrences_are_recorded() {
        let text = "aa";
        let actual = [
            SemanticToken::Content.with_span(Span::from_usize(0, 1)),
            SemanticToken::Content.with_span(Span::from_usize(1, 2)),
        ];
        assert_semantic_tokens(
            text,
            actual.as_slice(),
            &[
                (SemanticToken::Content, "a"),
                (SemanticToken::Content, "a"),
            ],
        );
    }

    #[test]
    fn sequential_search_accepts_an_empty_expected_list_when_nothing_was_recorded() {
        assert_semantic_tokens("", &[], &[]);
    }
```

## `leftover_token`

```rust
// from crates/isograph_parser/src/semantic_token.rs
use prelude::Postfix;

use crate::{NonBracketTokenKind, SplitToken};

// Only leftover fill-in: extra, extra_chunks, the matcher's cut.
pub(crate) fn leftover_token(kind: SplitToken) -> Option<SemanticToken> {
    match kind {
        SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral) => {
            SemanticToken::Integer.wrap_some()
        }
        SplitToken::NonBracket(
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
        ) => SemanticToken::String.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::Error) => SemanticToken::Error.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => SemanticToken::Content.wrap_some(),
        SplitToken::Bracket(_) => SemanticToken::Bracket.wrap_some(),
    }
}
```

The function is `pub(crate)`.

```rust
// from crates/isograph_parser/src/lib.rs
pub(crate) use semantic_token::leftover_token;
```

Facts:

- `leftover_token` on `Identifier`, `At`, `Exclamation`, `Dollar`, `Period`, `Colon`, `Equals`, and `Comma` is `Content`.
- `leftover_token` on `IntegerLiteral` is `Integer`. On `StringLiteral` and `BlockStringLiteral` is `String`. On `Error` is `Error`.
- `leftover_token` on each `BracketToken` is `Bracket`.
- `leftover_token` on `LineBreak` and `EndOfFile` is `None`.

## Record leftover

The walkers push a leftover role for each span that is not already in `tokens`. A failed chunk may clone contents that the prefix already committed. `entrypoint Query.` consumes every content item, then fails; leftover-in-extra.md puts the whole contents in extra, including `entrypoint` / `Query` / `.` which the grammar already recorded as `Keyword` / `Type` / `Period`. `record_leftover_span` skips a span that is already in `tokens`.

```rust
// from crates/isograph_parser/src/chunk.rs
fn record_leftover_item(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    item: &WithSpan<ChunkContentItem>,
) {
    match item.item.reference() {
        ChunkContentItem::NonBracket(token) => {
            record_leftover_span(
                tokens,
                leftover_token(SplitToken::NonBracket(token.0)),
                item.location,
            );
        }
        ChunkContentItem::Group(group) => {
            record_leftover_span(
                tokens,
                leftover_token(SplitToken::Bracket(BracketToken::Open(group.opening.item.0))),
                group.opening.location,
            );
            for chunk in group.children.item.0.iter() {
                record_leftover_chunk(tokens, chunk.item.reference());
            }
            record_leftover_span(
                tokens,
                leftover_token(SplitToken::Bracket(BracketToken::Close(group.closing.item.0))),
                group.closing.location,
            );
        }
    }
}

fn record_leftover_chunk(tokens: &mut Vec<WithSpan<SemanticToken>>, chunk: &Chunk) {
    for item in chunk.contents.iter() {
        record_leftover_item(tokens, item);
    }
}

fn record_leftover_extra(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    extra: &Option<WithSpan<UnparsedChunkItems>>,
) {
    if let Some(extra) = extra {
        for item in extra.item.0.iter() {
            record_leftover_item(tokens, item);
        }
    }
}

fn record_leftover_span(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    role: Option<SemanticToken>,
    span: Span,
) {
    if let Some(role) = role
        && tokens.iter().all(|recorded| recorded.location != span)
    {
        tokens.push(role.with_span(span));
    }
}
```

`record_leftover_chunk` walks `chunk.contents`. leftover-in-extra.md folds a singleton trailing separator into extra as `ChunkContentItem::NonBracket` items, so walking extra records that comma. Extra chunks keep their trailing separator on the chunk; `record_leftover_chunk` walks those chunks' `contents`. A trailing line break maps to `None` from `leftover_token`.

`chunk.rs` adds `leftover_token`, `SplitToken`, and `BracketToken` to its `use crate::{...}` list. `SplitToken` and `BracketToken` are already `pub(crate)` from `lib.rs`.

## `parse_one_chunk` records extra

leftover-in-extra.md's `parse_one_chunk` returns a `Slot`. `extra` is unread remainder, and, when leftover is not `Expectation::Separator(_)`, leftover-in-extra.md's `extra_plus_trailing_separator` appends the trailing separator tokens. That function returns the `Slot` from its `match result`. This change binds the match to `slot`, records leftover on `extra`, and returns `slot`. The Ok and Err arms are leftover-in-extra.md's after.

```rust
// from crates/isograph_parser/src/chunk.rs
    record_leftover_extra(stream.tokens(), &slot.item.extra);
    slot
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn tokens(&mut self) -> &mut Vec<WithSpan<SemanticToken>> {
        &mut self.0.tokens
    }
```

`parse_each_chunk` goes through `parse_one_chunk`. Leftover in a list item (`foo bar`, a failed `.` chunk) is extra and is recorded here.

## `parse_singleton` records extra_chunks

`parse_singleton` already builds `extra_chunks` from chunks after the first. After that value is built, this change walks each extra chunk and records leftover on its contents.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    let extra_chunks = (level.item.len() > 1).then(|| {
        errors.push(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    Singleton { item, extra_chunks }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    let extra_chunks = (level.item.len() > 1).then(|| {
        errors.push(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    if let Some(extra) = extra_chunks.as_ref() {
        for chunk in extra.item.0.iter() {
            record_leftover_chunk(tokens, chunk.item.reference());
        }
    }
    Singleton { item, extra_chunks }
```

`entrypoint\nQuery.foo`: chunk 0 is `entrypoint` (fails), chunk 1 is `Query.foo` in `extra_chunks`, `Query` / `.` / `foo` are `Content`.

## Tests

Every leftover fixture that already has a token list gains leftover roles in consume order. Tests keep calling `parsed` / `parsed_with_errors` / `assert_no_declaration` / `parsed_each` / `parsed_selections` / `parsed_pairs`. `tokens_after_a_complete_entrypoint_are_leftover` is the leftover-`bar` test.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn tokens_after_a_complete_entrypoint_are_leftover() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "foo"),
                (SemanticToken::Content, "bar"),
            ],
        );
        as_entrypoint(parse.reference());
        assert!(parsed_item(parse.reference()).is_some());
        assert!(first_slot(parse.reference()).extra.as_ref().is_some());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "bar"))
                .wrap_vec(),
        );
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn leftover_dollars_after_entrypoint_are_extra() {
        let text = "entrypoint $ $";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Content, "$"),
                (SemanticToken::Content, "$"),
            ],
        );
        // extra and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_trailing_comma_after_an_entrypoint_is_extra() {
        let text = "entrypoint Query.foo,";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "foo"),
                (SemanticToken::Content, ","),
            ],
        );
        // extra and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn leftover_then_a_trailing_comma_are_both_extra() {
        let text = "entrypoint Query.foo bar,";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "foo"),
                (SemanticToken::Content, "bar"),
                (SemanticToken::Content, ","),
            ],
        );
        // extra and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_failed_form_puts_unread_remainder_in_extra() {
        let text = "entrypoint Foo.$ asdf";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Foo"),
                (SemanticToken::Period, "."),
                (SemanticToken::Content, "$"),
                (SemanticToken::Content, "asdf"),
            ],
        );
        // extra and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_selection_set_on_an_entrypoint_is_leftover() {
        let text = "entrypoint Query.foo { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "foo"),
                (SemanticToken::Bracket, "{"),
                (SemanticToken::Content, "bar"),
                (SemanticToken::Bracket, "}"),
            ],
        );
        // error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_second_contentful_chunk_is_multiple_declarations() {
        let text = "entrypoint Query.foo\nfield User.name";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "foo"),
                (SemanticToken::Content, "field"),
                (SemanticToken::Content, "User"),
                (SemanticToken::Content, "."),
                (SemanticToken::Content, "name"),
            ],
        );
        // extra_chunks and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_failed_first_chunk_is_reported_even_when_a_second_exists() {
        let text = "entrypoint\nQuery.foo";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Content, "Query"),
                (SemanticToken::Content, "."),
                (SemanticToken::Content, "foo"),
            ],
        );
        // extra_chunks and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn an_unknown_keyword_is_an_error_at_the_keyword() {
        let text = "fieldd Query.foo { bar }";
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Token(Identifier)),
            span_of(text, "fieldd"),
            &[
                (SemanticToken::Keyword, "fieldd"),
                (SemanticToken::Content, "Query"),
                (SemanticToken::Content, "."),
                (SemanticToken::Content, "foo"),
                (SemanticToken::Bracket, "{"),
                (SemanticToken::Content, "bar"),
                (SemanticToken::Bracket, "}"),
            ],
        );
    }
```

`field Query.Foo to Pet!! { id }`. Sequential search skips the unrecorded first bang. `(SemanticToken::Content, "!")` is the second bang. `{ id }` is leftover Bracket / Content / Bracket. The error assertion stays on `second_bang`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_second_bang_is_leftover() {
        let text = "field Query.Foo to Pet!! { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "field"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "Foo"),
                (SemanticToken::Keyword, "to"),
                (SemanticToken::GraphQLTypeName, "Pet"),
                (SemanticToken::Content, "!"),
                (SemanticToken::Bracket, "{"),
                (SemanticToken::Content, "id"),
                (SemanticToken::Bracket, "}"),
            ],
        );
        as_selectable(parse.reference());
        let second_bang = Span::new(
            span_of(text, "Pet!!").start + 4,
            span_of(text, "Pet!!").start + 5,
        );
        assert_eq!(
            errors,
            expected(
                EndOfDeclaration,
                Found::Token(NonBracketTokenKind::Exclamation)
            )
            .with_span(second_bang)
            .wrap_vec(),
        );
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_line_break_inside_a_list_type_does_not_attach_bang() {
        let text = "field Query.Foo($pets: [Pet\n!]) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "field"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "Foo"),
                (SemanticToken::Parenthesis, "("),
                (SemanticToken::Variable, "$"),
                (SemanticToken::Variable, "pets"),
                (SemanticToken::Colon, ":"),
                (SemanticToken::GraphQLTypeName, "["),
                (SemanticToken::GraphQLTypeName, "Pet"),
                (SemanticToken::Content, "!"),
                (SemanticToken::GraphQLTypeName, "]"),
                (SemanticToken::Parenthesis, ")"),
                (SemanticToken::Brace, "{"),
                (SemanticToken::FieldName, "bar"),
                (SemanticToken::Brace, "}"),
            ],
        );
        // type and error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_directive_after_the_description_is_leftover() {
        let text = "field Query.Foo \"x\" @component { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "field"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "Foo"),
                (SemanticToken::String, "\"x\""),
                (SemanticToken::Content, "@"),
                (SemanticToken::Content, "component"),
                (SemanticToken::Bracket, "{"),
                (SemanticToken::Content, "bar"),
                (SemanticToken::Bracket, "}"),
            ],
        );
        // leftover error assertions stay
    }
```

```rust
// from crates/isograph_parser/src/chunk.rs
    fn leftover_after_a_list_item_keeps_the_item() {
        let text = "foo bar";
        let (items, errors, comma_errors) = parsed_each(
            text,
            &[
                (SemanticToken::FieldName, "foo"),
                (SemanticToken::Content, "bar"),
            ],
        );
        // item and error assertions stay
    }

    fn a_failed_list_chunk_is_none_and_the_next_chunk_still_parses() {
        let text = ".\nfoo";
        let (items, errors, comma_errors) = parsed_each(
            text,
            &[
                (SemanticToken::Content, "."),
                (SemanticToken::FieldName, "foo"),
            ],
        );
        // item and error assertions stay
    }
```

Remaining leftover fixtures gain leftover roles the same way. Failed prefixes keep the committed roles and record leftover on unread remainder. Leftover groups are Bracket / interior leftover / Bracket. Leftover integers are Integer. Leftover strings are String. Leftover `@name` that the grammar did not consume as a directive is Content at `@` and Content at the identifier.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    // a_pointer_keyword_is_not_a_declaration
    "pointer Pet.BestFriend to Owner { id }"
    (Keyword, "pointer"), (Content, "Pet"), (Content, "."), (Content, "BestFriend"),
    (Content, "to"), (Content, "Owner"), (Bracket, "{"), (Content, "id"), (Bracket, "}")

    // each_missing_entrypoint_part_reports_at_its_position, numeric
    "entrypoint 42.foo"
    (Keyword, "entrypoint"), (Content, "42."), (Content, "foo")

    // a_non_to_identifier_is_not_consumed_as_to
    "field Query.Foo Owner { id }"
    (Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"),
    (Content, "Owner"), (Bracket, "{"), (Content, "id"), (Bracket, "}")

    // a_to_after_a_directive_is_leftover
    "field Query.Foo @component to Pet { id }"
    (Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"),
    (DirectiveName, "@"), (DirectiveName, "component"),
    (Content, "to"), (Content, "Pet"), (Bracket, "{"), (Content, "id"), (Bracket, "}")
```

```rust
// from crates/isograph_parser/src/selections.rs
    // leftover_after_a_selection_keeps_the_item
    "bar baz\nqux"
    (FieldName, "bar"), (Content, "baz"), (FieldName, "qux")

    // a_period_where_a_selection_should_start_is_a_selection_error
    "...UserAvatar"
    (Content, "."), (Content, "."), (Content, "."), (Content, "UserAvatar")

    // an_orphaned_group_after_a_line_break_is_a_failed_selection
    "bar\n{ baz }"
    (FieldName, "bar"), (Bracket, "{"), (Content, "baz"), (Bracket, "}")
```
