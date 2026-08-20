# Combined `parse_iso_literal` entry

The crate entry is one function over `&str`. It runs tokenize, match brackets, chunk, grammar. The three error lists live on the return value, so a caller cannot drop a channel without ignoring a named field. Grammar-stage tests call this function. Stage unit tests (tokenize, brackets, chunk, subparsers) keep `pub(crate)` internals.

Does not depend on type-annotation-null.md or the `VariableDeclarationOrUsage` restructure. Public types keep their current names (`VariableDeclarationOrUsage`, `VariableDeclarationOrUsageList`, no `NullTypeAnnotation`).

## Types

Most important first.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<ParseError>>,
    pub bracket_errors: Vec<BracketError>,
    pub comma_errors: Vec<CommaWithoutItem>,
    pub tokens: Vec<WithSpan<SemanticToken>>,
}

pub fn parse_iso_literal(text: &str) -> ParsedIsoLiteral {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let item = parse_chunked_iso_literal(text, tree, &mut errors, &mut tokens);
    ParsedIsoLiteral {
        item,
        errors,
        bracket_errors,
        comma_errors,
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

`ParsedIsoLiteral` derives `Debug`. No `PartialEq`: `BracketError` and the tree are compared field-wise in tests.

## Change 1: `Display` on `BracketError` and `CommaWithoutItem`

Same impls as lsp-parse-diagnostics.md Change 1. Implement once, here. That doc's Change 1 is this work.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
use std::fmt;

impl fmt::Display for BracketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BracketError::UnmatchedOpen(open) => {
                write!(f, "Unclosed {}", open.item.0)
            }
            BracketError::UnmatchedClose(close) => {
                write!(f, "Unexpected {}", close.item.0)
            }
        }
    }
}

impl std::error::Error for BracketError {}
```

Before: `BracketError` has no `Display`. `OpenBracket` / `CloseBracket` are `pub struct OpenBracket(pub BracketKind)` / `pub struct CloseBracket(pub BracketKind)`. `BracketKind` already displays as `'('`, `'{'`, `'['`.

```rust
// from crates/isograph_parser/src/chunk.rs
use std::fmt;

impl fmt::Display for CommaWithoutItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A comma with no item before it.")
    }
}

impl std::error::Error for CommaWithoutItem {}
```

Before: `CommaWithoutItem` has no `Display`.

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
            "A comma with no item before it."
        );
    }
```

## Change 2: combined entry and grammar tests

`parse_iso_literal` / `parse_chunked_iso_literal` / `ParsedIsoLiteral` as in Types.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<ParseError>>) {
        let parsed = parse_iso_literal(text);
        assert!(parsed.bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(parsed.comma_errors, vec![], "for literal {text:?}");
        (
            parsed.item.expect("the fixture is not an empty literal"),
            parsed.errors,
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

Call sites of `parsed_with_errors` / `parsed_with_tokens` that bind the tuple become field reads (`parsed.item`, `parsed.errors`, `parsed.bracket_errors`, `parsed.comma_errors`, `parsed.tokens`). `ParsedWithErrors` and `ParsedWithTokens` aliases go. `parsed` still asserts `bracket_errors` empty and `comma_errors` empty.

`chunked` / `stream_of` used by `consume_description` unit tests stay on `pub(crate)` internals.

`arguments.rs` and `selections.rs` subparser tests stay on internals; they are not whole-literal parses.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn empty_literal_is_none_with_empty_literal_error() {
        let parsed = parse_iso_literal("");
        assert!(parsed.item.is_none());
        assert!(parsed.errors.iter().any(|error| {
            error.item == ParseError::EmptyLiteral
        }));
        assert!(parsed.bracket_errors.is_empty());
        assert_eq!(parsed.comma_errors, vec![]);
    }

    #[test]
    fn a_stray_close_is_on_bracket_errors_and_the_declaration_parses() {
        let parsed = parse_iso_literal("entrypoint Query.foo)");
        assert!(parsed.item.is_some());
        assert_eq!(parsed.bracket_errors.len(), 1);
        match parsed.bracket_errors[0].reference() {
            BracketError::UnmatchedClose(close) => {
                assert_eq!(close.item.0, BracketKind::Parenthesis);
            }
            error => panic!("expected UnmatchedClose, got {error:?}"),
        }
    }
```

`the_cut_removes_an_unmatched_bracket_and_the_declaration_parses` already covers the cut. This test asserts the combined entry kept `bracket_errors` instead of dropping them.

Who else calls the pipeline: extract-iso-literals.md and lsp-semantic-tokens.md `file_literals` call `parse_iso_literal(text)`. Those docs already use that call.

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
    TypeAnnotation, TypeAnnotationParent, VariableDeclarationOrUsage,
    VariableDeclarationOrUsageList, VariableDeclarationOrUsageListPath,
    VariableDeclarationOrUsagePath, VariableDeclarationOrUsageSlotPath,
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
