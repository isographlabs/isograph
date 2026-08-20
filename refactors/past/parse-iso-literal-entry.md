# Combined `parse_iso_literal` entry

The crate entry is one function over `&str`. Four stages: tokenize, match brackets, chunk, AST creation. The whole pipeline is parsing. AST creation is not parsing.

`ParseError` is the pipeline error. Matcher, chunker, and AST errors are one `Vec<WithSpan<ParseError>>`. Tokenize has no list: bad input is an `Error` token. AST-stage tests call this function. Stage unit tests (tokenize, brackets, chunk, subparsers) keep `pub(crate)` internals.

Does not depend on type-annotation-null.md. extract-iso-literals.md's `IsoLiteralError<H>` is `Host` plus this `ParseError`.

## Types

Most important first.

```rust
// from crates/isograph_parser/src/parse_error.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum AstError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Ast(#[from] AstError),
    #[error("{0}")]
    Bracket(#[from] BracketError),
    #[error("{0}")]
    Comma(#[from] CommaWithoutItem),
}
```

Before: `ParseError` is today's AST-stage enum (`Expected`, `EmptyLiteral`, `MultipleDeclarations`, `IntegerDoesNotFitI64`). That type is renamed to `AstError`. `ParseError` is the pipeline enum. `ParseError::expected` becomes `AstError::expected`. Every grammar-stage `ParseError` becomes `AstError`.

`#[from]` is thiserror's `From` for each payload. Strum has no wrapping `From`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<ParseError>>,
    pub tokens: Vec<WithSpan<SemanticToken>>,
}

pub fn parse_iso_literal(text: &str) -> ParsedIsoLiteral {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors: Vec<WithSpan<ParseError>> = bracket_errors
        .into_iter()
        .map(|error| {
            let location = match error.reference() {
                BracketError::UnmatchedOpen(open) => open.location,
                BracketError::UnmatchedClose(close) => close.location,
            };
            error.to::<ParseError>().with_span(location)
        })
        .collect();
    errors.extend(comma_errors.into_iter().map(|error| {
        error.to::<ParseError>().with_span(error.0)
    }));
    let mut ast_errors = Vec::new();
    let mut tokens = Vec::new();
    let item = parse_chunked_iso_literal(text, tree, &mut ast_errors, &mut tokens);
    errors.extend(ast_errors.into_iter().map(|error| {
        error.item.to::<ParseError>().with_span(error.location)
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
    errors: &mut Vec<WithSpan<AstError>>,
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

`errors` is matcher, then chunker, then AST, in that order. `parse_chunked_iso_literal` takes `Vec<WithSpan<AstError>>`. Mapping to `ParseError::Ast` is at the combined entry.

`match_brackets` still returns `Vec<BracketError>`. `chunk` still returns `Vec<CommaWithoutItem>`.

`ParsedIsoLiteral` derives `Debug`. No `PartialEq`: the tree is compared field-wise in tests.

Do not flatten `Expected` / `EmptyLiteral` / `UnmatchedOpen` onto one enum.

## Change 1: rename `ParseError` to `AstError`

Today's `ParseError` is the AST-stage enum. Rename it to `AstError` in `parse_error.rs` and every grammar-stage use (`cursor.expected`, `report_error`, tests that match `ParseError::EmptyLiteral`, `ParseError::expected(...)`). Independently shippable. No wrapping enum yet.

## Change 2: `thiserror` on `BracketError` and `CommaWithoutItem`

`ParseError` wraps with `#[error("{0}")]`, so the inner types need `Display`. They are errors. Derive `Error`; do not write a manual `Display` or `std::error::Error` impl.

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

## Change 3: combined entry and AST-stage tests

`parse_iso_literal` / `parse_chunked_iso_literal` / `ParsedIsoLiteral` / `ParseError` as in Types.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<AstError>>) {
        let parsed = parse_iso_literal(text);
        let errors = parsed
            .errors
            .into_iter()
            .map(|error| match error.item {
                ParseError::Ast(ast) => ast.with_span(error.location),
                ParseError::Bracket(_) | ParseError::Comma(_) => {
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
            error.item == ParseError::Ast(AstError::EmptyLiteral)
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
                ParseError::Bracket(BracketError::UnmatchedClose(close))
                    if close.item.0 == BracketKind::Parenthesis
                        && close.location == span_of(text, ")")
            ) && error.location == span_of(text, ")")
        }));
    }
```

`the_cut_removes_an_unmatched_bracket_and_the_declaration_parses` already covers the cut. This test asserts the combined entry kept the unmatched close instead of dropping it.

Who else calls the pipeline: extract-iso-literals.md and lsp-semantic-tokens.md `file_literals` call `parse_iso_literal(text)`. Those docs already use that call. `parsed.errors` is the one vec. extract-iso-literals.md `IsoLiteralError<H>` is `Host` plus this `ParseError`.

## Change 4: public surface

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
    VariableDeclarationOrUsage, VariableDeclarationOrUsageParent, VariableDeclarationOrUsagePath,
    VariableNameWrapper, VariableNameWrapperPath, VariableUse, VariableUsePath,
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
pub use parse_error::{AstError, Expectation, ExpectedFound, Found, ParseError};
pub use parse_iso_literal::{
    Description, DescriptionPath, EntityNameWrapper, EntityNameWrapperParent,
    EntityNameWrapperPath, EntrypointDeclaration, EntrypointDeclarationPath, ExtraChunksPath,
    IsoLiteralItem, IsoLiteralParse, IsoLiteralParsePath, IsoLiteralSlotPath, ParsedIsoLiteral,
    SelectableDeclaration, SelectableDeclarationPath, SelectableNameWrapper,
    SelectableNameWrapperParent, SelectableNameWrapperPath, parse_iso_literal,
};
pub use selections::{
    Selection, SelectionNameWrapper, SelectionNameWrapperPath, SelectionPath, SelectionSet,
    SelectionSetParent, SelectionSetPath, SelectionSlotPath,
};
pub use semantic_token::SemanticToken;
pub use variables::{
    ListTypeAnnotation, ListTypeAnnotationPath, NamedTypeAnnotation, NamedTypeAnnotationPath,
    NullTypeAnnotation, NullTypeAnnotationPath, TypeAnnotation, TypeAnnotationParent,
    VariableDeclaration, VariableDeclarationList, VariableDeclarationListPath,
    VariableDeclarationPath, VariableDeclarationSlotPath,
};

pub(crate) use arguments::{
    consume_argument_list, parse_name_colon, parse_non_constant_value, parse_variable_name,
};
pub(crate) use chunk::{chunk, parse_singleton};
pub(crate) use directives::consume_directives;
pub(crate) use matched_brackets::{BracketItem, Bracketed, MatchedBrackets, match_brackets};
pub(crate) use non_bracket_token::{BracketToken, SplitToken};
pub(crate) use parse_error::DECLARATION_KEYWORD;
pub(crate) use selections::consume_selection_set;
pub(crate) use token_kind::IsographLangTokenKind;
pub(crate) use tokenize::tokenize;
pub(crate) use variables::{consume_variable_declaration_list, parse_type_annotation};
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

Do not `pub use` `tokenize`, `match_brackets`, `chunk`, `ItemCursor`, `ChunkStream`, `IsographLangTokenKind`, `TokenKindExtras`, `MatchedBrackets`, `BracketItem`, `Bracketed`, `SplitToken`, `BracketToken`, `parse_type_annotation`, `consume_description`, or other parse helpers. `mod tokenize` and friends stay private. `tokenize` / `match_brackets` / `chunk` become `pub(crate)`. Crate-internal callers keep `use crate::tokenize` via the `pub(crate) use` block.
