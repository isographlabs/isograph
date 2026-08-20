# Combined `parse_iso_literal` entry

The crate entry is one function over `&str`. It runs tokenize, match brackets, chunk, grammar. Matcher, chunker, and grammar errors are one `Vec<WithSpan<IsoLiteralError>>`. Grammar-stage tests call this function. Stage unit tests (tokenize, brackets, chunk, subparsers) keep `pub(crate)` internals.

Does not depend on type-annotation-null.md. Grammar `ParseError` keeps its name. `IsoLiteralError` wraps it. extract-iso-literals.md's `IsoLiteralError<H>` is `Host` plus these three variants.

## Types

Most important first.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum IsoLiteralError {
    #[error("{0}")]
    Parse(#[from] ParseError),
    #[error("{0}")]
    Bracket(#[from] BracketError),
    #[error("{0}")]
    Comma(#[from] CommaWithoutItem),
}

pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<IsoLiteralError>>,
    pub tokens: Vec<WithSpan<SemanticToken>>,
}

pub fn parse_iso_literal(text: &str) -> ParsedIsoLiteral {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors: Vec<WithSpan<IsoLiteralError>> = bracket_errors
        .into_iter()
        .map(|error| {
            let location = match error.reference() {
                BracketError::UnmatchedOpen(open) => open.location,
                BracketError::UnmatchedClose(close) => close.location,
            };
            error.to::<IsoLiteralError>().with_span(location)
        })
        .collect();
    errors.extend(comma_errors.into_iter().map(|error| {
        error.to::<IsoLiteralError>().with_span(error.0)
    }));
    let mut grammar_errors = Vec::new();
    let mut tokens = Vec::new();
    let item = parse_chunked_iso_literal(text, tree, &mut grammar_errors, &mut tokens);
    errors.extend(grammar_errors.into_iter().map(|error| {
        error.item.to::<IsoLiteralError>().with_span(error.location)
    }));
    ParsedIsoLiteral {
        item,
        errors,
        tokens,
    }
}

pub(crate) fn parse_chunked_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>>
```

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>>
```

That body is `parse_chunked_iso_literal`. `item: None` is still only the empty-literal case.

`errors` is matcher, then chunker, then grammar, in that order. `parse_chunked_iso_literal` still takes `Vec<WithSpan<ParseError>>`. Mapping to `IsoLiteralError::Parse` is at the combined entry.

`match_brackets` still returns `Vec<BracketError>`. `chunk` still returns `Vec<CommaWithoutItem>`.

`ParsedIsoLiteral` derives `Debug`. No `PartialEq`: the tree is compared field-wise in tests.

`IsoLiteralError` is the combined error. Grammar `ParseError` is the payload of `Parse`. Do not flatten `Expected` / `EmptyLiteral` / `UnmatchedOpen` onto one enum. `#[from]` is thiserror's `From` impl for each payload (`ParseError`, `BracketError`, `CommaWithoutItem`). Strum has no wrapping `From`.

## Change 1: `thiserror` on `BracketError` and `CommaWithoutItem`

`IsoLiteralError` wraps with `#[error("{0}")]`, so the inner types need `Display`. They are errors. Derive `Error`; do not write a manual `Display` or `std::error::Error` impl.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum BracketError {
    #[error("Unclosed {}", .0.item.0)]
    UnmatchedOpen(WithSpan<OpenBracket>),
    #[error("Unexpected {}", .0.item.0)]
    UnmatchedClose(WithSpan<CloseBracket>),
}
```

Before: `#[derive(Debug, PartialEq, Eq)]`, no `Error`, no `Display`. `OpenBracket` / `CloseBracket` are `pub struct OpenBracket(pub BracketKind)` / `pub struct CloseBracket(pub BracketKind)`. `BracketKind` already displays as `'('`, `'{'`, `'['`.

```rust
// from crates/isograph_parser/src/chunk.rs
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("A comma with no item before it.")]
pub struct CommaWithoutItem(pub Span);
```

Before: `#[derive(Copy, Clone, Debug, PartialEq, Eq)]`, no `Error`, no `Display`.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    #[test]
    fn unmatched_open_displays_the_kind() {
        let err = BracketError::UnmatchedOpen(
            OpenBracket(BracketKind::Brace).with_span(Span::new(0, 1)),
        );
        assert_eq!(err.to_string(), "Unclosed '{'");
    }

    #[test]
    fn unmatched_close_displays_the_kind() {
        let err = BracketError::UnmatchedClose(
            CloseBracket(BracketKind::Parenthesis).with_span(Span::new(0, 1)),
        );
        assert_eq!(err.to_string(), "Unexpected '('");
    }
```

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn comma_without_item_displays() {
        assert_eq!(
            CommaWithoutItem(Span::new(0, 1)).to_string(),
            "A comma with no item before it.",
        );
    }
```

lsp-parse-diagnostics.md Change 1 is this work. Implement once, here.

## Change 2: combined entry and grammar tests

`parse_iso_literal` / `parse_chunked_iso_literal` / `ParsedIsoLiteral` / `IsoLiteralError` as in Types.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<ParseError>>) {
        let parsed = parse_iso_literal(text);
        let errors = parsed
            .errors
            .into_iter()
            .map(|error| match error.item {
                IsoLiteralError::Parse(parse) => parse.with_span(error.location),
                IsoLiteralError::Bracket(_) | IsoLiteralError::Comma(_) => {
                    panic!("for literal {text:?}")
                }
            })
            .collect();
        (
            parsed.item.expect("the fixture is not an empty literal"),
            errors,
        )
    }

    fn parsed_with_errors(text: &str) -> ParsedIsoLiteral {
        parse_iso_literal(text)
    }

    fn parsed_with_tokens(text: &str) -> ParsedIsoLiteral {
        parse_iso_literal(text)
    }
```

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn parsed_with_tokens(text: &str) -> ParsedWithTokens {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let parse = parse_iso_literal(text, tree, &mut errors, &mut tokens);
        (parse, errors, bracket_errors, comma_errors, tokens)
    }
```

Call sites of `parsed_with_errors` / `parsed_with_tokens` that bind the tuple become field reads (`parsed.item`, `parsed.errors`, `parsed.tokens`). `ParsedWithErrors` and `ParsedWithTokens` aliases go. `parsed` still panics if a matcher or chunker variant is present.

`chunked` / `stream_of` used by `consume_description` unit tests stay on `pub(crate)` internals.

`arguments.rs` and `selections.rs` subparser tests stay on internals; they are not whole-literal parses. They keep asserting `bracket_errors` / `comma_errors` on the stage returns.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn empty_literal_is_none_with_empty_literal_error() {
        let parsed = parse_iso_literal("");
        assert!(parsed.item.is_none());
        assert!(parsed.errors.iter().any(|error| {
            error.item == IsoLiteralError::Parse(ParseError::EmptyLiteral)
        }));
    }

    #[test]
    fn a_stray_close_is_a_parse_error_and_the_declaration_parses() {
        let text = "entrypoint Query.foo)";
        let parsed = parse_iso_literal(text);
        assert!(parsed.item.is_some());
        assert!(parsed.errors.iter().any(|error| {
            matches!(
                error.item.reference(),
                IsoLiteralError::Bracket(BracketError::UnmatchedClose(close))
                    if close.item.0 == BracketKind::Parenthesis
                        && close.location == span_of(text, ")")
            ) && error.location == span_of(text, ")")
        }));
    }
```

`the_cut_removes_an_unmatched_bracket_and_the_declaration_parses` already covers the cut. This test asserts the combined entry kept the unmatched close instead of dropping it.

Who else calls the pipeline: extract-iso-literals.md and lsp-semantic-tokens.md `file_literals` call `parse_iso_literal(text)`. Those docs already use that call. `parsed.errors` is the one vec. extract-iso-literals.md `IsoLiteralError<H>` adds `Host`; its `Parse` / `Bracket` / `Comma` are these variants.

## Change 3: public surface

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
mod token_kind;
mod tokenize;
mod variables;

pub use arguments::{
    Argument, ArgumentList, ArgumentListParent, ArgumentListPath, ArgumentNameWrapper,
    ArgumentNameWrapperPath, ArgumentPath, ArgumentSlotPath, Boolean, BooleanValue,
    BooleanValuePath, IntegerValue, IntegerValuePath, ListLiteral, ListLiteralPath,
    ListLiteralValue, ListLiteralValuePath, ListLiteralValueSlotPath, NonConstantValue,
    NonConstantValueParent, NullValue, NullValuePath, ObjectEntry, ObjectEntryPath,
    ObjectEntrySlotPath, ObjectLiteral, ObjectLiteralPath, StringLiteralValueWrapper,
    StringLiteralValueWrapperPath, ValueKeyNameWrapper, ValueKeyNameWrapperPath,
    VariableNameWrapper, VariableNameWrapperParent, VariableNameWrapperPath, VariableUse,
    VariableUsePath,
};
pub use chunk::{
    Chunk, ChunkContentItem, ChunkContentItemParent, ChunkParent, ChunkPath, ChunkSeparator,
    ChunkSeparatorPath, ChunkedGroup, ChunkedGroupPath, ChunkedLevel, ChunkedLevelParent,
    ChunkedLevelPath, CloseBracketPath, CommaWithoutItem, ExtraChunks, NonBracketTokenPath,
    OpenBracketPath, SeparatorToken, Singleton, Slot, UnparsedChunkItems, UnparsedChunkItemsParent,
    UnparsedChunkItemsPath,
};
pub use directives::{
    IsographDirectiveNameWrapper, IsographDirectiveNameWrapperPath, IsographFieldDirective,
    IsographFieldDirectiveList, IsographFieldDirectiveListParent, IsographFieldDirectiveListPath,
    IsographFieldDirectivePath,
};
pub use isograph_resolution_node::IsographResolutionNode;
pub use matched_brackets::{BracketError, CloseBracket, NonBracketToken, OpenBracket};
pub use non_bracket_token::{BracketKind, NonBracketTokenKind};
pub use parse_error::{Expectation, Found, ParseError};
pub use parse_iso_literal::{
    Description, DescriptionPath, EntityNameWrapper, EntityNameWrapperParent,
    EntityNameWrapperPath, EntrypointDeclaration, EntrypointDeclarationPath, ExtraChunksPath,
    IsoLiteralItem, IsoLiteralParse, IsoLiteralParsePath, IsoLiteralSlotPath, ParsedIsoLiteral,
    IsoLiteralError, SelectableDeclaration, SelectableDeclarationPath, SelectableNameWrapper,
    SelectableNameWrapperParent, SelectableNameWrapperPath, parse_iso_literal,
};
pub use selections::{
    Selection, SelectionNameWrapper, SelectionNameWrapperPath, SelectionPath, SelectionSet,
    SelectionSetParent, SelectionSetPath, SelectionSlotPath,
};
pub use semantic_token::SemanticToken;
pub use variables::{
    ListTypeAnnotation, ListTypeAnnotationPath, NamedTypeAnnotation, NamedTypeAnnotationPath,
    TypeAnnotation, TypeAnnotationParent, VariableDeclaration, VariableDeclarationList,
    VariableDeclarationListPath, VariableDeclarationPath, VariableDeclarationSlotPath,
};
```

Before:

```rust
// from crates/isograph_parser/src/lib.rs
pub use arguments::*;
pub use chunk::*;
pub use directives::*;
pub use isograph_resolution_node::*;
pub use matched_brackets::*;
pub use non_bracket_token::*;
pub use parse_error::*;
pub use parse_iso_literal::*;
pub use selections::*;
pub use semantic_token::*;
pub use token_kind::*;
pub use tokenize::*;
pub use variables::*;
```

Do not `pub use` `tokenize`, `match_brackets`, `chunk`, `ItemCursor`, `ChunkStream`, `IsographLangTokenKind`, `TokenKindExtras`, `MatchedBrackets`, `BracketItem`, `Bracketed`, `SplitToken`, `BracketToken`, `SeparatorToken`, `parse_type_annotation`, `consume_description`, or other parse helpers. `mod tokenize` and friends stay private. `tokenize` / `match_brackets` / `chunk` become `pub(crate)`.
