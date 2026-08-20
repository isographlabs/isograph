# `parse_each_chunk` test harness

`consume_selection_set`, `consume_argument_list`, `consume_variable_declaration_list`, object interiors, and list interiors all go through `parse_each_chunk`. That is the right extraction. This change does not touch those call sites.

Tests that feed a list interior have no enclosing parse. They copy `span_of`, `parsed_items`, and a dummy parent cursor (`match_brackets(tokenize("x"), 1)` then `chunk` then `stream`) in `arguments.rs` and `selections.rs`. `chunk.rs` `parsed_each` is the same dummy cursor, hardcoded to `parse_identifier` and `Separator(Parenthesis)`.

One shippable change. The helper lives in `isograph_parser` under `#[cfg(test)]`. `parse_each_chunk` and `ItemCursor` are `pub(crate)`. `crates/tests` stays empty.

No user-facing change.

parse-test-semantic-tokens.md lists `parsed_items` in `arguments.rs` and `selections.rs` and `parsed_each` in `chunk.rs`. After this, that parameter goes on `parsed_items` in `parsed_items.rs`. `parsed_pairs`, `parsed_selections`, and `parsed_each` pass it through. `parsed_argument_list` stays local and still asserts its own tokens.

## Helper

```rust
// from crates/isograph_parser/src/parsed_items.rs
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, CommaWithoutItem, Expectation, SemanticToken, Slot, UnparsedChunkItems, chunk,
    match_brackets, tokenize,
};

pub(crate) type ParsedItems<P> = (
    Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
    Vec<WithSpan<AstError>>,
    Vec<CommaWithoutItem>,
    Vec<WithSpan<SemanticToken>>,
);

pub(crate) fn parsed_items<P>(
    text: &str,
    leftover: Expectation,
    parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
) -> ParsedItems<P> {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    assert!(bracket_errors.is_empty(), "for literal {text:?}");
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let dummy = {
        let (brackets, bracket_errors) = match_brackets(tokenize("x"), 1);
        assert!(bracket_errors.is_empty());
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        tree
    };
    let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
    let items = tree
        .item
        .parse_each_chunk(parent.cursor(), leftover, parse_item);
    (items, errors, comma_errors, tokens)
}

/// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
/// cannot silently shift, and one that fails loudly when it stops being unique.
pub(crate) fn span_of(text: &str, pattern: &str) -> Span {
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
```

`parse_each_chunk` takes the parent cursor so nested lists share `text` / `tokens` / `errors`. A test that feeds a list interior has no parent parse. The helper builds a dummy one-chunk stream from `"x"` and streams the fixture text through it. `parse_each_chunk` never reads the dummy chunk's contents. It calls `parent.stream_chunk` on each fixture chunk.

The dummy `"x"` always has one chunk. Empty fixtures (`""`, whitespace, line breaks) still build the dummy; `parse_each_chunk` on the fixture level returns `[]`.

```rust
// from crates/isograph_parser/src/lib.rs
mod arguments;
mod chunk;
mod chunk_stream;
mod directives;
mod isograph_resolution_node;
mod matched_brackets;
mod non_bracket_token;
mod parse_error;
mod parse_iso_literal;
mod selections;
mod semantic_token;
mod string_value;
mod token_kind;
mod tokenize;
mod variables;

#[cfg(test)]
mod parsed_items;
```

## `arguments.rs`

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
    use crate::{
        AstError, CommaWithoutItem, Found, NonBracketTokenKind, SemanticToken, chunk,
        match_brackets, tokenize,
    };

    type ParsedItems<P> = (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<AstError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
    ) -> ParsedItems<P> {
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
```

After: those three items are gone. `parsed_pairs` and the direct `parsed_items` calls keep the same arguments. `parsed_argument_list` stays local: it streams the fixture's first chunk into `consume_argument_list`, not `parse_each_chunk`.

```rust
// from crates/isograph_parser/src/arguments.rs
    use crate::{
        AstError, Found, NonBracketTokenKind, SemanticToken, chunk, match_brackets,
        parsed_items::{parsed_items, span_of},
        tokenize,
    };

    fn parsed_pairs(text: &str) -> ParsedPairs {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
    }
```

Direct `parsed_items` calls stay:

- doubled comma between pairs: `parse_argument`, `Separator(Parenthesis)`
- list interiors (`1, $x, true`, trailing comma, leftover, doubled comma): `parse_list_literal_value`, `Separator(Bracket)`
- nested list/object and empty list interiors: `parse_non_constant_value` mapped to the inner item, `Expectation::Value`

## `selections.rs`

The `ParsedItems` alias, `parsed_items` dummy cursor, and `span_of` are the arguments.rs before listing. Delta: none. After: those three items are gone.

```rust
// from crates/isograph_parser/src/selections.rs
    use crate::{
        AstError, Found, NonBracketTokenKind, SemanticToken,
        parsed_items::{parsed_items, span_of},
    };

    fn parsed_selections(text: &str) -> ParsedSelections {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
    }
```

Direct `parsed_items` calls stay: leading comma and doubled comma (`parse_selection`, `Separator(Brace)`). `chunk`, `match_brackets`, `tokenize`, and `CommaWithoutItem` drop from this module's imports.

## `chunk.rs`

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    /// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
    /// cannot silently shift, and one that fails loudly when it stops being unique.
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

    type ParsedEach = (
        Vec<WithSpan<Slot<Span, UnparsedChunkItems>>>,
        Vec<WithSpan<AstError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    fn parsed_each(text: &str) -> ParsedEach {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let dummy = chunked("x");
        let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
        let items = tree.item.parse_each_chunk(
            parent.cursor(),
            Separator(BracketKind::Parenthesis),
            parse_identifier,
        );
        (items, errors, comma_errors, tokens)
    }
```

After: local `span_of` and `ParsedEach` are gone. `parsed_each` is the identifier leftover binding. `chunked` stays for chunk-structure tests.

```rust
// from crates/isograph_parser/src/chunk.rs
    use crate::{
        AstError, BracketError, BracketKind, Expectation, Found, NonBracketTokenKind,
        SemanticToken, chunk_stream::ItemCursor, match_brackets,
        parsed_items::{ParsedItems, parsed_items, span_of},
        tokenize,
    };

    fn parsed_each(text: &str) -> ParsedItems<Span> {
        parsed_items(
            text,
            Separator(BracketKind::Parenthesis),
            parse_identifier,
        )
    }
```

`parsed_each` call sites stay: `parse_each_chunk_on_an_empty_level_is_no_slots_and_no_errors`, `parse_each_chunk_parses_one_identifier_per_chunk`, `a_list_trailing_comma_is_not_a_parse_each_chunk_diagnostic`, `leftover_after_a_list_item_keeps_the_item`, `a_failed_list_chunk_is_none_and_the_next_chunk_still_parses`, `a_line_break_is_a_list_separator`, `a_comma_without_item_is_chunkings_error_and_the_item_parses`.

## Other `span_of` copies

The other test modules that define the same `span_of` import it. Local copies go.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    use crate::{
        ArgumentListParent, AstError, BracketError, BracketKind, ChunkContentItemParent,
        DECLARATION_KEYWORD, Expectation, Found, IntegerValue, IsographDirectiveNameWrapper,
        IsographFieldDirectiveListParent, IsographResolutionNode, NonBracketTokenKind,
        NonConstantValue, NonConstantValueParent, ObjectEntry, ParseError, Selection,
        SelectionNameWrapper, SelectionSet, SelectionSetParent, Slot, TypeAnnotation,
        TypeAnnotationParent, UnparsedChunkItems, UnparsedChunkItemsParent, VariableDeclaration,
        VariableDeclarationList, VariableDeclarationOrUsageParent, VariableNameWrapper, chunk,
        match_brackets, parsed_items::span_of, tokenize,
    };
```

The local `span_of` in that module is deleted. `chunked` and `stream_of` stay.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    use crate::{
        AstError, BracketKind, Chunk, ChunkedLevel, Expectation, Found, NonBracketTokenKind,
        SemanticToken, chunk, match_brackets, parsed_items::span_of, tokenize,
    };
```

The local `span_of` in that module is deleted. `chunked`, `first_chunk`, and `token_text` stay. `token_text` still calls `span_of`.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    use crate::{parsed_items::span_of, tokenize};
```

The local `span_of` in that module is deleted, including its doc comment (that comment is on the shared function). `tree` and `well_formed` stay.
