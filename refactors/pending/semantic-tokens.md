# Semantic tokens

The grammar stage records a semantic token as it consumes each token or bracket. Recording is a side effect of `consume_token_if` / `require_token` / `consume_group_if` / `require_group`. Each of those methods takes the `SemanticToken` the call site is consuming. The tree does not mention tokens; they are a sibling of the tree, not a field on it.

Three shippable changes. The first folds every logos error kind into `NonBracketTokenKind::Error`. The second always constructs tokens into a `Vec`. The third makes the collector a type parameter so the parse can be constructed with a noop or a non-noop.

## What a token is

A `SemanticToken` is a role. `require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)` records `Keyword`. The same identifier kind records `Type` or `FieldName` at the call sites that consume those roles.

```rust
// from crates/isograph_parser/src/semantic_token.rs
use span::{Span, WithSpan};

use crate::{BracketKind, NonBracketTokenKind};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SemanticToken {
    Keyword,
    Type,
    FieldName,
    DirectiveName,
    Variable,
    Argument,
    Identifier,
    Integer,
    String,
    BooleanOrNull,
    Period,
    Colon,
    Equals,
    Comma,
    Parenthesis,
    Brace,
    Bracket,
    LineBreak,
    EndOfFile,
    Error,
}

impl SemanticToken {
    pub fn from_non_bracket(kind: NonBracketTokenKind) -> SemanticToken {
        match kind {
            NonBracketTokenKind::Identifier => SemanticToken::Identifier,
            NonBracketTokenKind::IntegerLiteral => SemanticToken::Integer,
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral => {
                SemanticToken::String
            }
            NonBracketTokenKind::Period => SemanticToken::Period,
            NonBracketTokenKind::Colon => SemanticToken::Colon,
            NonBracketTokenKind::Dollar => SemanticToken::Variable,
            NonBracketTokenKind::Equals => SemanticToken::Equals,
            NonBracketTokenKind::Exclamation => SemanticToken::Type,
            NonBracketTokenKind::At => SemanticToken::DirectiveName,
            NonBracketTokenKind::Comma => SemanticToken::Comma,
            NonBracketTokenKind::LineBreak => SemanticToken::LineBreak,
            NonBracketTokenKind::EndOfFile => SemanticToken::EndOfFile,
            NonBracketTokenKind::Error => SemanticToken::Error,
        }
    }

    pub fn from_bracket(kind: BracketKind) -> SemanticToken {
        match kind {
            BracketKind::Parenthesis => SemanticToken::Parenthesis,
            BracketKind::Brace => SemanticToken::Brace,
            BracketKind::Bracket => SemanticToken::Bracket,
        }
    }
}
```

`from_non_bracket` and `from_bracket` are the leftover mapping. Every `NonBracketTokenKind` and every `BracketKind` has a token. Grammar consume does not call them; the call site names the role.

Call sites and the leftover mapping, by variant:

- `Keyword`: `entrypoint`, `field`, `pointer`, `to`.
- `Type`: `Query` / `User` in `Type.fieldName`, a type annotation's name, `!`, and a type-list `[]` the grammar consumes.
- `FieldName`: `foo` in `Query.foo`, a selection name, a selection alias, an object-literal key.
- `DirectiveName`: `@` and the directive identifier. Leftover `@` is this via `from_non_bracket`.
- `Variable`: `$` and the variable identifier.
- `Argument`: an argument name.
- `Identifier`: leftover identifiers. Grammar consume names a role instead.
- `Integer`: an integer literal.
- `String`: a string or block string, including a description.
- `BooleanOrNull`: `true` / `false` / `null`.
- `Period`: `.` in `Type.fieldName`.
- `Colon`: alias, argument, type-annotation, and object-entry colons.
- `Equals`: a variable default.
- `Comma`: leftover and separator commas. Grammar consume does not target `Comma`.
- `Parenthesis`: `(` and `)`.
- `Brace`: `{` and `}`.
- `Bracket`: leftover `[]`. A type-list consume passes `Type` for both sides.
- `LineBreak`: leftover line breaks. Grammar consume does not target `LineBreak`.
- `EndOfFile`: the leftover mapping for that kind. `tokenize` does not emit it.
- `Error`: every error token.

Open and close of one `BracketKind` share one token. `consume_group_if(kind, token)` records `token` on the open; `record_group_close(group, token)` records the same `token` on the close.

No `line_behavior` / `indent_change` on this type. Upstream puts both on `IsographSemanticToken` and the formatter walks that vec. Formatter metadata is a later change on this same type.

Tokens stay off the tree. Upstream stores `semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>` on each declaration (`entrypoint_declaration.rs`, `client_selectable_declaration.rs`). We do not.

## Origin: how isograph records during parse

Extracted from `crates/isograph_lang_parser/src/peekable_lexer.rs`. Every advance takes a legend constant and pushes it.

```rust
// from crates/isograph_lang_parser/src/peekable_lexer.rs (upstream)
pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<IsographLangTokenKind>,
    lexer: logos::Lexer<'source, IsographLangTokenKind>,
    source: &'source str,
    end_index_of_last_parsed_token: u32,
    offset: u32,
    semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>,
    pub text_source: TextSource,
}

impl<'source> PeekableLexer<'source> {
    fn parse_token(
        &mut self,
        isograph_semantic_token: IsographSemanticToken,
    ) -> WithEmbeddedLocation<IsographLangTokenKind> {
        let kind = self
            .lexer
            .next()
            .unwrap_or(IsographLangTokenKind::EndOfFile);
        self.end_index_of_last_parsed_token = self.current.span.end;
        let span = self.lexer_span();
        let parsed_token = std::mem::replace(&mut self.current, kind.with_span(span))
            .to_with_embedded_location(self.text_source);
        self.semantic_tokens
            .push(isograph_semantic_token.with_location(parsed_token.location));
        parsed_token
    }

    pub fn parse_token_of_kind(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithEmbeddedLocation<IsographLangTokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            self.parse_token(isograph_semantic_token).wrap_ok()
        } else {
            parse_token_kind_diagnostic(expected_kind, found.item, found.location).wrap_err()
        }
    }
}
```

Call sites pass the class:

```rust
// from crates/isograph_lang_parser/src/parse_iso_literal.rs (upstream)
let parent_type = tokens
    .parse_string_key_type(
        IsographLangTokenKind::Identifier,
        semantic_token_legend::ST_SERVER_OBJECT_TYPE,
    )?;
let dot = tokens
    .parse_token_of_kind(IsographLangTokenKind::Period, semantic_token_legend::ST_DOT)?;
```

Delta, common to the recording changes below:

- `require_token` / `consume_token_if` take the kind and the `SemanticToken`. `require_group` / `consume_group_if` take the `BracketKind` and the `SemanticToken`.
- Open and close share one token. Upstream splits `ST_OPEN_PAREN` / `ST_CLOSE_PAREN` (and the brace pair) for formatter metadata.
- One variant per role. Upstream's `ST_DIRECTIVE_AT` / `ST_DIRECTIVE` are both `DirectiveName`; `ST_VARIABLE_DOLLAR_DECLARATION` / `ST_VARIABLE_DOLLAR_USAGE` / `ST_VARIABLE` are `Variable`; `ST_KEYWORD_USE` / `ST_KEYWORD_DECLARATION` / `ST_TO` are `Keyword`; `ST_SERVER_OBJECT_TYPE` / `ST_TYPE_ANNOTATION` and `!` are `Type`; `ST_CLIENT_SELECTABLE_NAME` / `ST_SELECTION_NAME_OR_ALIAS` / `ST_SELECTION_NAME_OR_ALIAS_POST_COLON` / `ST_OBJECT_LITERAL_KEY` are `FieldName`; `ST_STRING_LITERAL` covers string and block string.
- The constructor does not push a dummy token and pop it. Upstream `PeekableLexer::new` does `parse_token(ST_COMMENT)` then `semantic_tokens.pop()`.
- A failed `require_token` records nothing. A successful consume that a later `?` discards stays recorded. There is no corrective pop.
- The declaration types do not grow a `semantic_tokens` field.

## 1. One `Error` kind

`NonBracketTokenKind` has one error variant. Logos still has one variant per error regex. The split folds them.

Before:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonBracketTokenKind {
    Error,
    ErrorUnterminatedString,
    ErrorUnsupportedStringCharacter,
    ErrorUnterminatedBlockString,
    At,
    Colon,
    Dollar,
    EndOfFile,
    Equals,
    Exclamation,
    Identifier,
    IntegerLiteral,
    LineBreak,
    ErrorNumberLiteralLeadingZero,
    ErrorNumberLiteralTrailingInvalid,
    ErrorFloatLiteralMissingZero,
    Period,
    Comma,
    StringLiteral,
    BlockStringLiteral,
}
```

After:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
/// `IsographLangTokenKind` with the six bracket tokens unrepresentable. Every logos error
/// kind is `Error`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonBracketTokenKind {
    Error,
    At,
    Colon,
    Dollar,
    EndOfFile,
    Equals,
    Exclamation,
    Identifier,
    IntegerLiteral,
    LineBreak,
    Period,
    Comma,
    StringLiteral,
    BlockStringLiteral,
}
```

Before (`From<IsographLangTokenKind> for SplitToken` error arms):

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            IsographLangTokenKind::Error => SplitToken::NonBracket(NonBracketTokenKind::Error),
            IsographLangTokenKind::ErrorUnterminatedString => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnterminatedString)
            }
            IsographLangTokenKind::ErrorUnsupportedStringCharacter => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnsupportedStringCharacter)
            }
            IsographLangTokenKind::ErrorUnterminatedBlockString => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnterminatedBlockString)
            }
```

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorNumberLiteralLeadingZero)
            }
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid)
            }
            IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorFloatLiteralMissingZero)
            }
```

After:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            IsographLangTokenKind::Error
            | IsographLangTokenKind::ErrorUnterminatedString
            | IsographLangTokenKind::ErrorUnsupportedStringCharacter
            | IsographLangTokenKind::ErrorUnterminatedBlockString
            | IsographLangTokenKind::ErrorNumberLiteralLeadingZero
            | IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
            | IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::Error)
            }
```

Before (`From<NonBracketTokenKind> for IsographLangTokenKind` error arms):

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            NonBracketTokenKind::Error => IsographLangTokenKind::Error,
            NonBracketTokenKind::ErrorUnterminatedString => {
                IsographLangTokenKind::ErrorUnterminatedString
            }
            NonBracketTokenKind::ErrorUnsupportedStringCharacter => {
                IsographLangTokenKind::ErrorUnsupportedStringCharacter
            }
            NonBracketTokenKind::ErrorUnterminatedBlockString => {
                IsographLangTokenKind::ErrorUnterminatedBlockString
            }
```

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            NonBracketTokenKind::ErrorNumberLiteralLeadingZero => {
                IsographLangTokenKind::ErrorNumberLiteralLeadingZero
            }
            NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid => {
                IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
            }
            NonBracketTokenKind::ErrorFloatLiteralMissingZero => {
                IsographLangTokenKind::ErrorFloatLiteralMissingZero
            }
```

After:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            NonBracketTokenKind::Error => IsographLangTokenKind::Error,
```

`Display` still delegates to `IsographLangTokenKind::from(*self)`. `Found::Token(Error)` formats as `error`.

Before (the one grammar test that names a specific error kind):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    use NonBracketTokenKind::{
        At, Comma, Dollar, ErrorNumberLiteralTrailingInvalid, Identifier, Period,
    };
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(
                token(Identifier),
                Found::Token(ErrorNumberLiteralTrailingInvalid),
            ),
            span_of(numeric, "42."),
        );
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    use NonBracketTokenKind::{At, Comma, Dollar, Error, Identifier, Period};
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(token(Identifier), Found::Token(Error)),
            span_of(numeric, "42."),
        );
```

### Tests

- `SplitToken::from` on `Error`, `ErrorUnterminatedString`, `ErrorUnsupportedStringCharacter`, `ErrorUnterminatedBlockString`, `ErrorNumberLiteralLeadingZero`, `ErrorNumberLiteralTrailingInvalid`, and `ErrorFloatLiteralMissingZero` is `NonBracket(Error)`.
- `IsographLangTokenKind::from(NonBracketTokenKind::Error)` is `IsographLangTokenKind::Error`.
- `a_non_bracket_token_round_trips_through_the_split` still holds for `At`, `Identifier`, `StringLiteral`, and `Error`.
- `entrypoint 42.foo` is `Found::Token(Error)` at `42.`.

## 2. Always record into a `Vec`

No trait. No type parameter. The cursor holds `&mut Vec<WithSpan<SemanticToken>>`. Every parse constructs tokens.

Grammar functions stay `fn parse_entrypoint(cursor: &mut ItemCursor<'_>)`. They do not mention the vec. They pass a `SemanticToken` into each consume.

### The cursor

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
}

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a NonEmpty<WithSpan<ChunkContentItem>>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> Self {
        ChunkStream(ItemCursor {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
            text,
            tokens,
        })
    }
}

impl Chunk {
    pub(crate) fn stream<'a>(
        &'a self,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text, tokens)
    }
}
```

Before (`consume_token_if` / `consume_group_if`):

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        item.location.wrap_some()
    }

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                group.with_span(item.location).wrap_some()
            }
            _ => None,
        }
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<Span> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(found) if found.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        self.record(token, item.location);
        item.location.wrap_some()
    }

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                self.record(token, group.opening.location);
                group.with_span(item.location).wrap_some()
            }
            _ => None,
        }
    }

    pub(crate) fn record_group_close(&mut self, group: &ChunkedGroup, token: SemanticToken) {
        self.record(token, group.closing.location);
    }

    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.push(token.with_span(span));
    }
```

`require_token` / `require_group` stay `consume_*` or `Err(())`. They take the same `token` and inherit the side effect. `expected` only peeks and records nothing.

A consume that does not match records nothing.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<Span, ()> {
        self.consume_token_if(kind, token).ok_or(())
    }

    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()> {
        self.consume_group_if(kind, token).ok_or(())
    }
```

### Landed grammar call sites

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword)
        .wrap_err(),
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword)
        .wrap_err(),
    }
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
```

`fieldd Query.foo` records `Keyword` at `fieldd`, then fails. The identifier was in the keyword position.

### Source order for groups

`consume_group_if` records the opening and returns the group. The interior is a new cursor over `group.children`. The closing must be recorded after that interior, or the vec is `open, close, interior...`.

The group-plus-interior pattern in parsing-standards.md becomes one method on the cursor. The method reborrows `self.tokens` for the child cursors, then records the close with the same token the open used. The parent cursor is not used for anything else during the reborrow.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn parse_group_items<P, F>(
        &mut self,
        group: &ChunkedGroup,
        token: SemanticToken,
        parse_item: impl Fn(&mut ItemCursor<'_>, &mut F) -> Result<P, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        let text = self.text;
        let items = group
            .children
            .item
            .parse_items(text, self.tokens, parse_item, push_error);
        self.record_group_close(group, token);
        items
    }

    pub(crate) fn parse_group_singleton<T, F>(
        &mut self,
        group: &ChunkedGroup,
        token: SemanticToken,
        end: Expectation,
        extra_chunks: impl FnOnce(&WithSpan<Chunk>) -> WithSpan<ParseError>,
        parse: impl FnOnce(&mut ItemCursor<'_>, &mut F) -> Result<T, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        let text = self.text;
        let parsed = parse_singleton(
            group.children.reference(),
            text,
            self.tokens,
            end,
            extra_chunks,
            parse,
            push_error,
        );
        self.record_group_close(group, token);
        parsed
    }
}
```

`parse_group_items_with_trailing` is the same wrapper around `parse_items_with_trailing`, and lands with that function (parse-fields.md). It takes `token` and passes it to `record_group_close`.

Before (group interior):

```rust
// from crates/isograph_parser/src/selections.rs (parse-fields.md)
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    SelectionSet(
        group
            .item
            .children
            .item
            .parse_items_with_trailing(cursor.text(), parse_selection)
            .into_iter()
            .map(WithSpan::<SelectionSlot>::from)
            .collect(),
    )
    .with_span(group.location)
    .wrap_ok()
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
    let group = cursor
        .require_group(BracketKind::Brace, SemanticToken::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    SelectionSet(
        cursor
            .parse_group_items_with_trailing(group.item, SemanticToken::Brace, parse_selection)
            .into_iter()
            .map(WithSpan::<SelectionSlot>::from)
            .collect(),
    )
    .with_span(group.location)
    .wrap_ok()
```

`parse_value`'s brace arm, `consume_argument_list`, `consume_variable_declaration_list`, and `[...]` via `parse_group_singleton` are the same substitution. Type-list `[...]` passes `SemanticToken::Type` to both `consume_group_if` / `require_group` and `parse_group_singleton`. Those functions still take only the cursor besides `push_error`.

`parse_group_*` is not landed until `parse_items` is (parse-fields.md). This change lands `record_group_close` and the open-on-consume. parse-fields.md and parse-arguments.md / parse-variables.md use the helper in the snippets above.

### Pending grammar call sites

Each `require_*` / `consume_*` in a pending feature doc gains the `SemanticToken` argument. Delta from the listed origin, per call.

parse-fields.md (`parse_field`, origin `refactors/pending/parse-fields.md`):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
```

```rust
// from crates/isograph_parser/src/selections.rs
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Selection))?;
    let (reader_alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon, SemanticToken::Colon)
    {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
```

`require_selection_set` / `consume_selection_set` pass `SemanticToken::Brace` as in the group-interior snippet above.

parse-arguments.md (origin `refactors/pending/parse-arguments.md` and `refactors/pending/parsing-standards.md`):

```rust
// from crates/isograph_parser/src/arguments.rs
    let group = cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis)?;
```

```rust
// from crates/isograph_parser/src/arguments.rs
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Argument)
        .map_err(|()| cursor.expected(Expectation::Argument))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
```

```rust
// from crates/isograph_parser/src/arguments.rs
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::ObjectEntry))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(dollar) = cursor.consume_token_if(NonBracketTokenKind::Dollar, SemanticToken::Variable)
        {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
                .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        {
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        {
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::BooleanOrNull)
        {
```

```rust
// from crates/isograph_parser/src/arguments.rs
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace) {
```

`parse_constant_value` (parse-variables.md) is the same ladder without the `$` arm, with the same tokens on the remaining arms.

parse-variables.md (origin `refactors/pending/parse-variables.md`):

```rust
// from crates/isograph_parser/src/variables.rs
    let group = cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis)?;
```

```rust
// from crates/isograph_parser/src/variables.rs
    cursor
        .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::VariableDeclaration))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let type_annotation = parse_type_annotation(cursor, push_error)?;
    let default_value = match cursor.consume_token_if(NonBracketTokenKind::Equals, SemanticToken::Equals)
    {
```

```rust
// from crates/isograph_parser/src/variables.rs
        if let Some(name) =
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::Type)
        {
            cursor.consume_token_if(NonBracketTokenKind::Exclamation, SemanticToken::Type);
```

```rust
// from crates/isograph_parser/src/variables.rs
        if let Some(group) = cursor.consume_group_if(BracketKind::Bracket, SemanticToken::Type) {
            let inner = parse_bracket_interior_type(
                cursor.text(),
                group.item.children.reference(),
                push_error,
            )?;
            cursor.consume_token_if(NonBracketTokenKind::Exclamation, SemanticToken::Type);
```

The `[...]` interior goes through `parse_group_singleton(group, SemanticToken::Type, ...)`. Both sides of the type list are `Type`.

parse-descriptions.md (origin `refactors/pending/parse-descriptions.md`):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let span = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        .or_else(|| {
            cursor.consume_token_if(NonBracketTokenKind::BlockStringLiteral, SemanticToken::String)
        })?;
```

parse-pointers.md (origin `refactors/pending/parse-pointers.md`):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_pointer_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let variable_definitions = consume_variable_declaration_list(cursor, push_error);
    let to_keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::ToKeyword))?;
```

When directives land, `@name` is `consume_token_if(At, DirectiveName)` then `require_token(Identifier, DirectiveName)`. Until then leftover fill-in maps `@` to `DirectiveName` and the identifier to `Identifier`.

### Threading through the list helpers

`parse_chunk`, `parse_one_item`, `parse_singleton`, and (when it lands) `parse_items` take `&mut Vec<WithSpan<SemanticToken>>` and pass it to `stream`. Grammar closures still receive `&mut ItemCursor`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    parse_item: impl FnOnce(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
) -> (ChunkStream<'a>, Result<WithSpan<P>, WithSpan<ParseError>>) {
    let mut stream = chunk.item.stream(text);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}

fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<Slot<P, UnparsedChunkItems>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) = parse_chunk(chunk, text, |cursor| parse(cursor, push_error));
    /* unchanged match */
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_item(&level.item.0[0], text, end, parse, push_error);
    /* unchanged extra-chunks / comma */
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    parse_item: impl FnOnce(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
) -> (ChunkStream<'a>, Result<WithSpan<P>, WithSpan<ParseError>>) {
    let mut stream = chunk.item.stream(text, tokens);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}

fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<Slot<P, UnparsedChunkItems>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) =
        parse_chunk(chunk, text, tokens, |cursor| parse(cursor, push_error));
    /* unchanged match */
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_item(&level.item.0[0], text, tokens, end, parse, push_error);
    /* unchanged extra-chunks / comma */
}
```

`parse_items` (parsing-standards.md, lands in parse-fields.md) takes `tokens: &mut Vec<WithSpan<SemanticToken>>` and passes it to each `parse_one_item`. Sequential chunks: the previous `ChunkStream` is dropped before the next `stream` reborrows `tokens`.

### Entry point

`parse_iso_literal` takes the vec. Every call constructs tokens.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    singleton.with_span(location).wrap_some()
}
```

```
entrypoint Query.foo
```

records, in source order:

- `SemanticToken::Keyword` at `entrypoint`
- `SemanticToken::Type` at `Query`
- `SemanticToken::Period` at `.`
- `SemanticToken::FieldName` at `foo`

`entrypoint Foo.$ asdf` records `Keyword` at `entrypoint`, `Type` at `Foo`, `Period` at `.`, then fails at `$`. Those three tokens stay. There is no pop.

Leftover items (`asdf`) and separator commas are not consumed, so they are not recorded. Positions in leftover still resolve through `UnparsedChunkItems`. Highlighting them is the leftover fill-in change.

### Layering

Every byte of the literal is classified by at most one of these. Diagnostics stay a third channel:

1. Recorded: a token or bracket the grammar consumed, classified by the `SemanticToken` the call site passed.
2. Lexical fill-in (later): a token no consume covered (leftover, a separator comma, text in the matcher's cut, an `Error` token). Classification is `from_non_bracket` / `from_bracket`.
3. Diagnostics: the matcher's vec, chunking's `CommaWithoutItem` vec, and `push_error`. An `Error` token is highlighting. It does not replace a diagnostic.

So `foo ( asfd`: `foo` is recorded as `FieldName` when that consume ran; `(` is an unmatched-open diagnostic and is not in the tree; `asfd` sits in the cut and, after fill-in, highlights as `Identifier`. After fill-in the `(` highlights as `Parenthesis`.

### Tests

No snapshots. Facts:

- `from_non_bracket` on each `NonBracketTokenKind` is the mapping in `What a token is`. `Error` is `Error`. `Comma` is `Comma`. `At` is `DirectiveName`. `Dollar` is `Variable`. `Exclamation` is `Type`. `StringLiteral` and `BlockStringLiteral` are `String`.
- `from_bracket` on `Parenthesis` / `Brace` / `Bracket` is `Parenthesis` / `Brace` / `Bracket`.
- `require_token(Identifier, Keyword)` on `entrypoint` yields `[Keyword @ entrypoint]`.
- `require_token(Identifier, Type)` on `Query` yields `[Type @ Query]`.
- `require_token(Identifier, FieldName)` on `foo` yields `[FieldName @ foo]`.
- `require_token(Period, Period)` when the next item is an identifier records nothing and returns `Err(())`.
- `consume_token_if` that does not match records nothing.
- `consume_group_if(Brace, Brace)` on `{ bar }` records `Brace` at `{`. `record_group_close(group, Brace)` then records `Brace` at `}`.
- `parse_iso_literal` on `entrypoint Query.foo` fills the four tokens above and the same tree as today.
- A failed first chunk that consumed a prefix (`entrypoint Foo.$`) still has the prefix tokens (`Keyword`, `Type`, `Period`).
- `fieldd Query.foo` records `Keyword` at `fieldd`.
- `expected` does not record.

`stream_of` in `chunk_stream.rs` tests takes the vec:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn stream_of<'a>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> ChunkStream<'a> {
        first_chunk(tree).stream(text, tokens)
    }
```

Existing consume tests pass `&mut Vec::new()` and a `SemanticToken` on every `consume_*` / `require_*`. They do not assert on the vec. Collecting tests pass a vec and assert its contents. Matching tests that do not care about the role pass `from_non_bracket(kind)` / `from_bracket(kind)`.

### Docs this change amends

- parsing-standards.md: `ItemCursor` gains `tokens: &'a mut Vec<WithSpan<SemanticToken>>`. `consume_token_if` / `require_token` take `(NonBracketTokenKind, SemanticToken)`. `consume_group_if` / `require_group` take `(BracketKind, SemanticToken)`. `Chunk::stream`, `parse_chunk`, `parse_one_item`, `parse_singleton`, `parse_items` take the vec. Catalog adds `record`, `record_group_close`, `parse_group_items` / `parse_group_singleton`. The group-plus-interior listing becomes the helper. The value ladder, `$name`, alias, and `to` listings pass the tokens in Pending grammar call sites.
- parse-entrypoint.md: `parse_iso_literal` takes `tokens: &mut Vec<WithSpan<SemanticToken>>`. The keyword / type / period / field-name consumes pass the tokens above.
- parse-fields.md, parse-arguments.md, parse-variables.md, parse-descriptions.md, parse-pointers.md: each consume listed above.
- parsing-plan.md: tokens are recorded during parse into a vec. `require_token` takes the role.

## 3. Construct with a noop or a non-noop

The collector becomes a type parameter. Two implementors, by design; this seam is the whole of what the trait exists for. The parse is constructed with one or the other.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub trait SemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span);
}

pub struct CollectedSemanticTokens(pub Vec<WithSpan<SemanticToken>>);

pub struct NoSemanticTokens;

impl CollectedSemanticTokens {
    pub fn new() -> CollectedSemanticTokens {
        CollectedSemanticTokens(Vec::new())
    }
}

impl NoSemanticTokens {
    pub fn new() -> NoSemanticTokens {
        NoSemanticTokens
    }
}

impl SemanticTokens for CollectedSemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.0.push(token.with_span(span));
    }
}

impl SemanticTokens for NoSemanticTokens {
    fn record(&mut self, _token: SemanticToken, _span: Span) {}
}
```

`record` takes `(SemanticToken, Span)`, not a pre-built `WithSpan`. `CollectedSemanticTokens` constructs the `WithSpan` inside `record`. `NoSemanticTokens::record` is empty, so that monomorphization does not build a `WithSpan`.

### Cursor and helpers

`ItemCursor` / `ChunkStream` gain `TTokens`. The bound lives on the `impl`, not the struct. Every `&mut Vec<WithSpan<SemanticToken>>` from change 2 becomes `&mut TTokens`. Grammar functions become generic; their bodies do not mention `TTokens`. Consume signatures stay `(kind, token)`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
}

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>>
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a, TTokens> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut TTokens,
}

pub(crate) struct ChunkStream<'a, TTokens>(ItemCursor<'a, TTokens>);

impl<'a, TTokens: SemanticTokens> ItemCursor<'a, TTokens> {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.record(token, span);
    }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint<TTokens: SemanticTokens>(
    cursor: &mut ItemCursor<'_, TTokens>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>>
```

`parse_iso_literal_item`, `parse_selection`, `parse_value`, and the rest of the grammar functions are the same substitution.

`Chunk::stream`, `parse_chunk`, `parse_one_item`, `parse_singleton`, `parse_items`, and `parse_group_*` are generic over `TTokens: SemanticTokens` and take `&mut TTokens`. Closures receive `&mut ItemCursor<'_, TTokens>`.

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P, TTokens: SemanticTokens>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut TTokens,
    parse_item: impl FnOnce(&mut ItemCursor<'a, TTokens>) -> Result<P, WithSpan<ParseError>>,
) -> (
    ChunkStream<'a, TTokens>,
    Result<WithSpan<P>, WithSpan<ParseError>>,
) {
    let mut stream = chunk.item.stream(text, tokens);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}
```

### Entry point

`parse_iso_literal` is generic. The caller constructs the collector.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal<TTokens: SemanticTokens>(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
    tokens: &mut TTokens,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    singleton.with_span(location).wrap_some()
}
```

Non-noop:

```rust
let mut tokens = CollectedSemanticTokens::new();
let tree = parse_iso_literal(text, root, push_error, &mut tokens);
// tokens.0 is the vec
```

Noop:

```rust
let tree = parse_iso_literal(text, root, push_error, &mut NoSemanticTokens::new());
```

The tree is the same either way.

### Tests

The change-2 facts still hold against `CollectedSemanticTokens`. Added:

- `parse_iso_literal` with `NoSemanticTokens::new()` returns the same tree as with `CollectedSemanticTokens::new()`.
- `stream_of` is generic. Existing consume tests that do not assert tokens pass `&mut NoSemanticTokens::new()`. Collecting tests pass `&mut CollectedSemanticTokens::new()` and assert on `.0`.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn stream_of<'a, TTokens: SemanticTokens>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut TTokens,
    ) -> ChunkStream<'a, TTokens> {
        first_chunk(tree).stream(text, tokens)
    }
```

### Docs this change amends

- parsing-standards.md: `ItemCursor` / `ChunkStream` gain `TTokens`. List helpers are generic over `TTokens: SemanticTokens`. Grammar functions take `ItemCursor<'_, TTokens>`. Catalog replaces the vec with `SemanticTokens`, `CollectedSemanticTokens::new`, `NoSemanticTokens::new`.
- parse-entrypoint.md: `parse_iso_literal` is generic over `TTokens`.
- parse-fields.md, parse-arguments.md, parse-variables.md, parse-descriptions.md, parse-pointers.md: each `parse_*` gains `TTokens: SemanticTokens`.
- spanless-parsing.md: the cheap pass is `NoSemanticTokens::new()` plus `NoSpan`.

## Later changes

Each is independently shippable.

### Leftover fill-in

A walk over `tokenize(text)` that emits `from_non_bracket` / `from_bracket` for every token whose span is not already in the collected vec, in source order. Separators, leftover items, the matcher's cut, and `Error` tokens get those tokens. The collected vec stays sorted by span. This is the LSP layer, not the parser's consume path. It runs against `CollectedSemanticTokens`.

### Formatter metadata

`SemanticToken` gains `line_behavior` and `indent_change`, the fields on upstream `IsographSemanticToken`. The formatter walks `Vec<WithSpan<SemanticToken>>` the way `crates/isograph_lsp/src/format.rs` walks the upstream vec.

### Cheap pass

spanless-parsing.md's `TSpan = NoSpan` parse is constructed with `NoSemanticTokens::new()`. The compile path:

```rust
let tree = parse_iso_literal::<NoSpan>(
    text,
    root,
    push_error,
    &mut NoSemanticTokens::new(),
);
if /* push_error was called, or tree.has_errors() */ {
    let mut tokens = CollectedSemanticTokens::new();
    let spanned = parse_iso_literal::<Span>(text, root, push_error, &mut tokens);
    report(spanned);
}
```

The LSP path always constructs with `CollectedSemanticTokens::new()` and `TSpan = Span`. One function, two instantiations of each parameter, same consume-time recording. The two trees cannot disagree about structure.

`TSpan` is the tree's parameter. `TTokens` is the cursor's. They are chosen together at the entry point by which values are constructed. `require_token` names neither parameter.
