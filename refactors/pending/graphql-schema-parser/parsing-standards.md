# GraphQL schema parser: standards

`crates/graphql_schema_parser` parses one GraphQL schema file into the per-definition results in ast.md. order.md is the work list for this crate. integration.md is how `graphql_network_protocol` reads the results. This file is the pipeline, the tokenizer, strings, the cursor, errors, and recovery that every grammar function in order.md uses. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Spec: GraphQL, September 2025 edition (https://spec.graphql.org/September2025/). Section numbers below are that edition's.

Origin: i2 `crates/isograph_parser`. The tokenizer, `TokenText`, the cursor's method names and shapes, `spanning`, the error types, `intern_block_string_value`, and `span_of` are copied from there; each copy names its file and states its delta. Nothing comes from `relay-crates` except `intern`, which the `common_lang_types` string keys are built on. There is no formatter.

## Pipeline

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
pub fn parse_schema(text: &str) -> ParsedSchema
```

`text` is one schema file. Two stages run:

1. `tokenize(text)` (logos) returns `Vec<WithSpan<GraphQLTokenKind>>`.
2. Grammar consume: a `TokenCursor` over that slice parses top-level definitions one after another. A definition that fails becomes an `Err` entry and the cursor skips to the next definition (Recovery below).

The iso parser also matches brackets and chunks. Those stages exist so that an error stays inside its bracket group or chunk and the rest of the literal still parses. The schema parser recovers only between top-level definitions, which needs a bracket depth counter inside one skip loop and no bracket tree. Nothing would read a bracket tree or a chunk tree, so neither stage exists here.

Every file is parsed as a spec 3.1 `TypeSystemExtensionDocument`: definitions and extensions in any order, in any file. Config names schema files with one glob list. There is no main schema file and no separate extensions entry point.

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
pub fn parse_schema(text: &str) -> ParsedSchema {
    let items = tokenize(text);
    let mut cursor = TokenCursor::new(items.as_slice(), text);
    ParsedSchema {
        definitions: std::iter::from_fn(|| {
            (!cursor.at_end()).then(|| parse_definition_or_extension_or_skip(&mut cursor))
        })
        .collect(),
    }
}
```

`ParsedSchema` (ast.md) holds one entry per top-level definition or extension, in source order. Origin: iso `ParsedIsoLiteral { item, errors, tokens }` (`crates/isograph_parser/src/parse_iso_literal.rs`). Delta: each definition carries its own result; there is no file-wide error list and no `tokens` (Later below).

An empty file, or a file of only whitespace and comments, has no entries. Spec 3.1 writes `TypeSystemDefinitionOrExtension+`, so the spec rejects that file; here an empty file contributes nothing and reports nothing, because one glob list can match an empty placeholder file and an error would have no definition to belong to.

Caller: `parsed_schema_file` in `graphql_network_protocol` (integration.md). The unit tests in this crate are the only other caller.

AGENTS.md says no other crate depends on `graphql_schema_parser`. That sentence changes when `graphql_network_protocol` takes the dependency; the AGENTS.md edit is separate from these docs.

## Later

Semantic tokens, `ResolvePosition` on the AST, and diagnostics for schema files come with the first LSP feature on `.graphql` files. Until then nothing reads them, so this crate records no semantic tokens, derives no `ResolvePosition`, and has no resolution node. Every source node keeps its `WithSpan`: the `Err` entries' spans are read by `entity_named` (integration.md), and go-to-definition will read the name spans.

## Tokenizer

```rust
// from crates/graphql_schema_parser/src/tokenize.rs
pub(crate) fn tokenize(text: &str) -> Vec<WithSpan<GraphQLTokenKind>> {
    let mut lexer = GraphQLTokenKind::lexer(text);
    let mut tokens = Vec::new();
    while let Some(kind) = lexer.next() {
        let kind = lexer.extras.error_token.take().unwrap_or(kind);
        tokens.push(kind.with_span(lexer.span().to()));
    }
    tokens
}
```

Copied from iso `crates/isograph_parser/src/tokenize.rs`. Delta: `GraphQLTokenKind` in place of `IsographLangTokenKind`; the parameter is the file, not a literal.

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
#[derive(Default)]
pub struct TokenKindExtras {
    pub(crate) error_token: Option<GraphQLTokenKind>,
}

#[derive(Logos, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, strum::Display)]
#[logos(extras = TokenKindExtras)]
pub enum GraphQLTokenKind {
    #[regex(r"[ \t\r\n,\ufeff]+|#[^\r\n]*", logos::skip)]
    #[error]
    #[strum(to_string = "error")]
    Error,

    #[strum(to_string = "unterminated string")]
    ErrorUnterminatedString,
    #[strum(to_string = "unsupported character in string")]
    ErrorUnsupportedStringCharacter,
    #[strum(to_string = "unterminated block string")]
    ErrorUnterminatedBlockString,

    #[token("&")]
    #[strum(to_string = "ampersand ('&')")]
    Ampersand,
    #[token("@")]
    #[strum(to_string = "at symbol ('@')")]
    At,
    #[token("}")]
    #[strum(to_string = "closing brace ('}}')")]
    CloseBrace,
    #[token("]")]
    #[strum(to_string = "closing bracket (']')")]
    CloseBracket,
    #[token(")")]
    #[strum(to_string = "closing parenthesis (')')")]
    CloseParenthesis,
    #[token(":")]
    #[strum(to_string = "colon (':')")]
    Colon,
    #[token("=")]
    #[strum(to_string = "equals ('=')")]
    Equals,
    #[token("!")]
    #[strum(to_string = "exclamation mark ('!')")]
    Exclamation,

    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)")]
    #[strum(to_string = "floating point value (e.g. '3.14')")]
    FloatLiteral,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    #[strum(to_string = "non-variable identifier (e.g. 'x' or 'Foo')")]
    Identifier,

    #[regex("-?(0|[1-9][0-9]*)")]
    #[strum(to_string = "integer value (e.g. '0' or '42')")]
    IntegerLiteral,
    #[regex("-?0[0-9]+(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?")]
    #[strum(to_string = "unsupported number (int or float) literal")]
    ErrorNumberLiteralLeadingZero,
    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?[.a-zA-Z_]")]
    #[strum(to_string = "unsupported number (int or float) literal")]
    ErrorNumberLiteralTrailingInvalid,
    #[regex("-?(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+)")]
    #[strum(to_string = "unsupported number (int or float) literal")]
    ErrorFloatLiteralMissingZero,

    #[token("{")]
    #[strum(to_string = "open brace ('{{')")]
    OpenBrace,
    #[token("[")]
    #[strum(to_string = "open bracket ('[')")]
    OpenBracket,
    #[token("(")]
    #[strum(to_string = "open parenthesis ('(')")]
    OpenParenthesis,
    #[token("|")]
    #[strum(to_string = "pipe ('|')")]
    Pipe,

    #[token("\"", lex_string)]
    #[strum(to_string = "string literal (e.g. '\"...\"')")]
    StringLiteral,
    #[token("\"\"\"", lex_block_string)]
    #[strum(to_string = "block string (e.g. '\"\"\"hi\"\"\"')")]
    BlockStringLiteral,
}
```

Copied from iso `crates/isograph_parser/src/token_kind.rs` `IsographLangTokenKind`. Delta:

- The skip regex is the spec's `Ignored` (2.1.6): `UnicodeBOM`, `Whitespace` (2.1.1, tab and space only, so form feed is no longer skipped and lexes as `Error`), `LineTerminator` (2.1.2), `Comment` (2.1.3, `#` to the end of the line), and `Comma` (2.1.4). Iso skips `[ \t\f\ufeff]+` and lexes line breaks and commas as tokens because they separate iso chunks.
- `LineBreak`, `Comma`, `Dollar`, `Period`, and `EndOfFile` are removed. No schema production reads them. `$`, `.`, and `...` (spec 2.1.7 punctuators the type system grammar never uses) lex as `Error`. The end of input is `Found::EndOfFile`, not a token.
- `Ampersand` (`implements A & B`, spec 3.6) and `Pipe` (union members, spec 3.8; directive locations, spec 3.13) are added.
- The number regexes are iso's. They implement spec 2.10.1 `IntValue` and 2.10.2 `FloatValue`, including the `[lookahead != {Digit, ., NameStart}]` rule (`1.`, `1a`, `0x` are `ErrorNumberLiteralTrailingInvalid`; `01` is `ErrorNumberLiteralLeadingZero`; `.5` is `ErrorFloatLiteralMissingZero`).
- `Identifier` is spec 2.1.8 `Name`.

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
#[derive(Logos, Debug)]
pub enum StringToken {
    #[error]
    Error,

    #[token("\\\"")]
    EscapedQuote,
    #[token("\\\\")]
    EscapedBackslash,
    #[token("\\/")]
    EscapedSlash,
    #[token("\\b")]
    EscapedBackspace,
    #[token("\\f")]
    EscapedFormFeed,
    #[token("\\n")]
    EscapedLineFeed,
    #[token("\\r")]
    EscapedCarriageReturn,
    #[token("\\t")]
    EscapedTab,

    #[regex(r#"\\u[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]"#, fixed_width_code_unit)]
    EscapedUnicode(u16),

    #[regex(r#"\\u\{[0-9A-Fa-f]+\}"#, variable_width_escape)]
    EscapedVariableWidthUnicode(VariableWidthEscape),

    #[token("\"")]
    Quote,

    #[regex(r#"\n|\r|\r\n"#)]
    LineTerminator,

    #[regex(r#"[^"\\\r\n]+"#)]
    StringCharacters,
}

/// The value of a `\u{...}` escape.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VariableWidthEscape {
    Scalar(char),
    NotAScalar,
}

fn fixed_width_code_unit(lexer: &mut Lexer<'_, StringToken>) -> Option<u16> {
    u16::from_str_radix(&lexer.slice()[2..], 16).ok()
}

fn variable_width_escape(lexer: &mut Lexer<'_, StringToken>) -> VariableWidthEscape {
    let slice = lexer.slice();
    u32::from_str_radix(&slice[3..slice.len() - 1], 16)
        .ok()
        .and_then(char::from_u32)
        .map_or(VariableWidthEscape::NotAScalar, VariableWidthEscape::Scalar)
}
```

Copied from iso `StringToken`. Delta, all from spec 2.10.4:

- `EscapedCharacter` splits into the eight escapes of the `EscapedCharacter` table, so the decoder maps each variant to its scalar value with no fallback arm.
- `EscapedUnicode` carries its code unit. A 4-digit hex slice always fits `u16`; logos treats a `None` callback result as `Error`.
- `EscapedVariableWidthUnicode` is new: the `\u{` HexDigit+ `}` form. Its payload is `NotAScalar` when the digits overflow `u32` or name a surrogate or a value above U+10FFFF; the decoder reports that.
- `StringCharacters` widens from `[\u0009\u0020\u0021\u0023-\u005B\u005D-\uFFFF]+` to `[^"\\\r\n]+`. Spec 2.1 `SourceCharacter` is any Unicode scalar value, and `StringCharacter` is a `SourceCharacter` but not `"`, `\`, or `LineTerminator`. Iso's range rejected supplementary characters (an emoji in a string) and control characters. A Rust `&str` holds no surrogates, so the negated class is exactly the spec set.

`BlockStringToken` is copied from iso unchanged. Its `Other` arm and its `Error` arm are treated the same in `lex_block_string`, so a supplementary or control character in a block string already lexes.

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
fn lex_string(lexer: &mut Lexer<'_, GraphQLTokenKind>) -> bool {
    let remainder = lexer.remainder();
    let mut string_lexer = StringToken::lexer(remainder);
    let mut first_error: Option<GraphQLTokenKind> = None;
    while let Some(string_token) = string_lexer.next() {
        match string_token {
            StringToken::Quote => {
                lexer.bump(string_lexer.span().end);
                return match first_error {
                    None => true,
                    Some(kind) => {
                        lexer.extras.error_token = kind.wrap_some();
                        false
                    }
                };
            }
            StringToken::LineTerminator => {
                return fail_with(
                    lexer,
                    string_lexer.span().start,
                    first_error.unwrap_or(GraphQLTokenKind::ErrorUnterminatedString),
                );
            }
            StringToken::EscapedQuote
            | StringToken::EscapedBackslash
            | StringToken::EscapedSlash
            | StringToken::EscapedBackspace
            | StringToken::EscapedFormFeed
            | StringToken::EscapedLineFeed
            | StringToken::EscapedCarriageReturn
            | StringToken::EscapedTab
            | StringToken::EscapedUnicode(_)
            | StringToken::EscapedVariableWidthUnicode(_)
            | StringToken::StringCharacters => {}
            StringToken::Error => {
                if first_error.is_none() {
                    first_error = GraphQLTokenKind::ErrorUnsupportedStringCharacter.wrap_some();
                }
            }
        }
    }
    fail_with(
        lexer,
        lexer.remainder().len(),
        first_error.unwrap_or(GraphQLTokenKind::ErrorUnterminatedString),
    )
}
```

Copied from iso `lex_string`. Delta: `GraphQLTokenKind`; the accepted arm lists the split escapes and the new variable-width escape. `fail_with` and `lex_block_string` are copied from iso with `GraphQLTokenKind` in place of `IsographLangTokenKind` and no other change.

Logos picks the longest match, so `""""""` is one empty `BlockStringLiteral` and not three `StringLiteral`s (spec 2.10.4: "the source `""""""` can only be interpreted as a single empty block string").


## String values

A `StringLiteral` or `BlockStringLiteral` token's span includes its quotes. The stored value is the decoded interior. Both kinds are spec 2.10.4 `StringValue` and are accepted wherever the spec says `StringValue`: descriptions (spec 2.2 `Description : StringValue`) and constant values.

```rust
// from crates/graphql_schema_parser/src/string_value.rs
pub(crate) fn decode_string_value(
    interior: TokenText<'_>,
) -> Result<String, WithSpan<ParseError>> {
    let mut pieces = StringToken::lexer(interior.text()).spanned().peekable();
    let mut value = String::new();
    while let Some((piece, range)) = pieces.next() {
        let location = interior.sub_span(range.clone());
        match piece {
            StringToken::EscapedQuote => value.push('"'),
            StringToken::EscapedBackslash => value.push('\\'),
            StringToken::EscapedSlash => value.push('/'),
            StringToken::EscapedBackspace => value.push('\u{0008}'),
            StringToken::EscapedFormFeed => value.push('\u{000C}'),
            StringToken::EscapedLineFeed => value.push('\n'),
            StringToken::EscapedCarriageReturn => value.push('\r'),
            StringToken::EscapedTab => value.push('\t'),
            StringToken::EscapedUnicode(first) => {
                let mut units = NonEmpty::new(first.with_span(location));
                while let Some((StringToken::EscapedUnicode(unit), range)) =
                    pieces.next_if(|(piece, _)| matches!(piece, StringToken::EscapedUnicode(_)))
                {
                    units.push(unit.with_span(interior.sub_span(range)));
                }
                value.push_str(&decode_code_units(&units)?);
            }
            StringToken::EscapedVariableWidthUnicode(VariableWidthEscape::Scalar(scalar)) => {
                value.push(scalar)
            }
            StringToken::EscapedVariableWidthUnicode(VariableWidthEscape::NotAScalar) => {
                return ParseError::InvalidUnicodeEscape.with_span(location).wrap_err();
            }
            StringToken::StringCharacters
            | StringToken::Quote
            | StringToken::LineTerminator
            | StringToken::Error => value.push_str(&interior.text()[range]),
        }
    }
    value.wrap_ok()
}

fn decode_code_units(units: &NonEmpty<WithSpan<u16>>) -> Result<String, WithSpan<ParseError>> {
    let location = Span::join(units.first().location, units.last().location);
    char::decode_utf16(units.iter().map(|unit| unit.item))
        .collect::<Result<String, _>>()
        .map_err(|_| ParseError::InvalidUnicodeEscape.with_span(location))
}
```

Spec 2.10.4 Static Semantics. A run of consecutive fixed-width escapes is decoded as UTF-16, so `"\uD83D\uDCA9"` (a surrogate pair) is U+1F4A9, the same value as `"\u{1F4A9}"`. An unpaired surrogate (`"\uDEAD"`) or a variable-width escape that is not a Unicode scalar value (`"\u{110000}"`, `"\u{D800}"`) is `InvalidUnicodeEscape`; for a fixed-width run the error spans the run. The last arm copies text verbatim, which makes the decoder total over any `&str`; a `StringLiteral` interior holds no `Quote`, `LineTerminator`, or `Error` piece because `lex_string` ended or failed there.

`TokenText::sub_span(range: Range<usize>) -> Span` is `Span::new(self.location.start + range.start as u32, self.location.start + range.end as u32)`: a byte range inside `text()` as a span in the file.

```rust
// from crates/graphql_schema_parser/src/string_value.rs
pub(crate) fn intern_block_string_value<T: From<intern::string_key::StringKey>>(
    interior: &str,
) -> T {
    clean_block_string(&interior.replace("\\\"\"\"", "\"\"\""))
        .intern()
        .to()
}

fn clean_block_string(source: &str) -> String {
    let common_indent = get_common_indent(source);

    let mut formatted_lines = block_string_lines(source)
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                line.to_string()
            } else {
                line.chars().skip(common_indent).collect::<String>()
            }
        })
        .collect::<VecDeque<String>>();

    while formatted_lines
        .front()
        .is_some_and(|line| line_is_whitespace(line))
    {
        formatted_lines.pop_front();
    }
    while formatted_lines
        .back()
        .is_some_and(|line| line_is_whitespace(line))
    {
        formatted_lines.pop_back();
    }

    let lines_vec: Vec<String> = formatted_lines.into_iter().collect();
    lines_vec.join("\n")
}

fn get_common_indent(source: &str) -> usize {
    let lines = block_string_lines(source).skip(1);
    let mut common_indent: Option<usize> = None;
    for line in lines {
        if let Some((first_index, _)) = line.match_indices(is_not_whitespace).next()
            && common_indent.is_none_or(|indent| first_index < indent)
        {
            common_indent = first_index.wrap_some()
        }
    }
    common_indent.unwrap_or(0)
}

fn block_string_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .split("\r\n")
        .flat_map(|line| line.split(['\n', '\r']))
}
```

Copied from iso `crates/isograph_parser/src/string_value.rs`; `line_is_whitespace` and `is_not_whitespace` are copied unchanged. Delta:

- `\"""` becomes `"""` before dedenting (spec 2.10.4 `BlockStringCharacter :: \"""`). Iso keeps the backslash.
- Lines split at every spec `LineTerminator`, a lone CR included, through `block_string_lines`. Iso uses `str::lines`, which does not split at a lone CR. `str::lines` also drops a final empty line; the trailing whitespace-only lines are removed either way, so the result differs only at a lone CR.

The algorithm is spec 2.10.4 `BlockStringValue()`: the first line keeps its indent, the smallest indent of the other non-blank lines is removed from them, blank leading and trailing lines are removed, and lines join with U+000A.

```rust
// from crates/graphql_schema_parser/src/string_value.rs
pub(crate) fn consume_string_value_if<T: From<intern::string_key::StringKey>>(
    cursor: &mut TokenCursor<'_>,
) -> Result<Option<WithSpan<T>>, WithSpan<ParseError>> {
    if let Some(string) = cursor.consume_token_if(GraphQLTokenKind::StringLiteral) {
        let value = decode_string_value(string.exclude_ends(1))?;
        return value
            .intern()
            .to::<T>()
            .with_span(string.location)
            .wrap_some()
            .wrap_ok();
    }
    cursor
        .consume_token_if(GraphQLTokenKind::BlockStringLiteral)
        .map(|block| {
            intern_block_string_value::<T>(block.exclude_ends(3).text()).with_span(block.location)
        })
        .wrap_ok()
}
```

Origin: iso `consume_description` (`crates/isograph_parser/src/parse_iso_literal.rs`), the two `consume_token_if` calls. Delta: generic over the interned type; no semantic token; the plain string is decoded (iso interns the raw interior). Callers: `consume_description` (`DescriptionValue`) and the string arm of `parse_constant_value` (`StringLiteralValue`).


## `TokenText`

Copied from iso `crates/isograph_parser/src/chunk_stream.rs` with the same fields and `text`, `exclude_ends`, and `interned`. Delta: `sub_span` (String values above).

## `TokenCursor`

```rust
// from crates/graphql_schema_parser/src/token_cursor.rs
/// A position in the token slice a `TokenCursor` reads.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenIndex(usize);

/// Sequential reader of a file's tokens. Parameter of a parse function.
pub(crate) struct TokenCursor<'a> {
    items: &'a [WithSpan<GraphQLTokenKind>],
    position: TokenIndex,
    /// The end of the last token this cursor advanced past (the first token's start before
    /// any). An `Expected(_, EndOfFile)` error uses this offset.
    previous_end: u32,
    text: &'a str,
}

impl<'a> TokenCursor<'a> {
    pub(crate) fn new(items: &'a [WithSpan<GraphQLTokenKind>], text: &'a str) -> Self {
        TokenCursor {
            items,
            position: TokenIndex(0),
            previous_end: match items.first() {
                Some(item) => item.location.start,
                None => 0,
            },
            text,
        }
    }

    pub(crate) fn peek(&self) -> Option<WithSpan<GraphQLTokenKind>> {
        self.peek_nth(0)
    }

    pub(crate) fn peek_nth(&self, n: usize) -> Option<WithSpan<GraphQLTokenKind>> {
        self.items.get(self.position.0 + n).copied()
    }

    pub(crate) fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    pub(crate) fn position(&self) -> TokenIndex {
        self.position
    }

    /// The tokens from `start` up to the cursor; empty when `start` is not behind it.
    pub(crate) fn consumed_since(&self, start: TokenIndex) -> &'a [WithSpan<GraphQLTokenKind>] {
        self.items
            .get(start.0..self.position.0)
            .unwrap_or_default()
    }

    /// From the start of the token at `start` to the end of the last consumed token;
    /// zero-width at `previous_end` when nothing was consumed since `start`.
    pub(crate) fn span_since(&self, start: TokenIndex) -> Span {
        match self.consumed_since(start).first() {
            Some(first) => Span::new(first.location.start, self.previous_end),
            None => Span::new(self.previous_end, self.previous_end),
        }
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn advance(&mut self) {
        if let Some(item) = self.peek() {
            self.previous_end = item.location.end;
            self.position = TokenIndex(self.position.0 + 1);
        }
    }

    pub(crate) fn consume_token_if(&mut self, kind: GraphQLTokenKind) -> Option<TokenText<'a>> {
        let item = self.peek().filter(|item| item.item == kind)?;
        self.advance();
        TokenText {
            location: item.location,
            text: self.text,
        }
        .wrap_some()
    }

    pub(crate) fn require_token(&mut self, kind: GraphQLTokenKind) -> Result<TokenText<'a>, ()> {
        self.consume_token_if(kind).ok_or(())
    }

    /// The next token as `K` when it is an identifier whose text parses as `K`. Consumes
    /// nothing.
    pub(crate) fn peek_identifier_of<K: FromStr>(&self) -> Option<WithSpan<K>> {
        let item = self
            .peek()
            .filter(|item| item.item == GraphQLTokenKind::Identifier)?;
        K::from_str(&self.text[item.location.as_usize_range()])
            .ok()
            .map(|keyword| keyword.with_span(item.location))
    }

    pub(crate) fn consume_identifier_of<K: FromStr>(&mut self) -> Option<WithSpan<K>> {
        let keyword = self.peek_identifier_of::<K>()?;
        self.advance();
        keyword.wrap_some()
    }

    pub(crate) fn consume_keyword_if(&mut self, word: &'static str) -> Option<Span> {
        let item = self.peek().filter(|item| {
            item.item == GraphQLTokenKind::Identifier
                && &self.text[item.location.as_usize_range()] == word
        })?;
        self.advance();
        item.location.wrap_some()
    }

    pub(crate) fn require_keyword(&mut self, word: &'static str) -> Result<Span, ()> {
        self.consume_keyword_if(word).ok_or(())
    }

    pub(crate) fn expected(&self, expected: Expectation) -> WithSpan<ParseError> {
        match self.peek() {
            None => ParseError::expected(expected, Found::EndOfFile)
                .with_span(Span::new(self.previous_end, self.previous_end)),
            Some(item) => {
                ParseError::expected(expected, Found::Token(item.item)).with_span(item.location)
            }
        }
    }

    pub(crate) fn spanning<T, E>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<WithSpan<T>, E> {
        let start = match self.peek() {
            Some(item) => item.location.start,
            None => self.previous_end,
        };
        let before = self.previous_end;
        let value = parse(self)?;
        let span = if self.previous_end == before {
            Span::new(before, before)
        } else {
            Span::new(start, self.previous_end)
        };
        value.with_span(span).wrap_ok()
    }
}
```

Origin: iso `ItemCursor` (`crates/isograph_parser/src/chunk_stream.rs`). `consume_token_if`, `require_token`, `expected`, `text`, and `spanning` keep iso's names and shapes; `spanning` is iso's body including the zero-width rule (a parse that consumed nothing has the zero-width span at `previous_end`). Parse functions take only `&mut TokenCursor` and never a vec. Delta:

- No `tokens` and no `errors` field. Iso's cursor holds `&'a mut Vec` of semantic tokens and of errors because iso records tokens and reports errors from inside nested chunk parses. Here nothing records semantic tokens (Later above), and a definition's first error is its result, returned through `?` to the loop that stores it (Recovery below). So `consume_token_if` and `require_token` take no semantic-token parameter, and there is no `report_error`.
- `items` is a token slice plus `TokenIndex`, not a `SafePeekable` over chunk content. Recovery needs a two-token lookahead (`peek_nth(1)`), the tokens a failed definition consumed (`consumed_since`), and that definition's extent (`span_since`).
- `expected` takes `&self` and reports `Found::EndOfFile` where iso reports `Found::EndOfChunk`.
- `spanning` is generic over the error type, so a definition parse that returns `DefinitionError` (Recovery below) uses it too.
- No `CursorPeek` (with no recording, `consume_token_if` is iso's `CursorPeek::advance` as well as its `commit`) and no group methods (`consume_group_if`, `require_group`, `stream_chunk`); with no bracket tree a group is an open token, items, and a close token (Groups below).
- `peek_identifier_of`, `consume_identifier_of`, `consume_keyword_if`, and `require_keyword` are new (Keyword dispatch below). `K: FromStr` is a std trait; every `K` is an enum deriving `strum::EnumString`.

`require_*` is `consume_*` or `Err(())`. The caller maps `Err` with `expected`: `cursor.require_token(GraphQLTokenKind::Colon).map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Colon)))?`. A failed require consumes nothing.

## Keyword dispatch

When the next identifier selects among several forms, the parse function asks for the identifier as an enum before consuming it:

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
let keyword = cursor
    .consume_identifier_of::<DefinitionKeyword>()
    .ok_or_else(|| cursor.expected(expectation))?;
```

An unknown word is not consumed, and the error's span is that word (it is still the peeked token when `expected` runs). Iso's `parse_iso_literal_item` consumes the identifier first and then reports at its span; here the word is never taken as a keyword.

```rust
// from crates/graphql_schema_parser/src/keyword.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum DefinitionKeyword {
    Schema,
    Scalar,
    Type,
    Interface,
    Union,
    Enum,
    Input,
    Directive,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum ExtensionKeyword {
    Schema,
    Scalar,
    Type,
    Interface,
    Union,
    Enum,
    Input,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum BooleanOrNullKeyword {
    True,
    False,
    Null,
}
```

`DirectiveLocation` and `OperationType` (ast.md) are the other `K`s. `DefinitionKeyword` is spec 3 `TypeSystemDefinition` with 3.4 `TypeDefinition` flattened. `ExtensionKeyword` is spec 3.1 `TypeSystemExtension` with 3.4.3 `TypeExtension` flattened; it has no `Directive` because the spec has no directive extension. `extend` is `consume_keyword_if("extend")`.

## Groups

Spec groups are `{ X+ }` (fields, input fields, enum values, root operation types), `( X+ )` (arguments definitions, directive arguments), `[ X* ]` (list values), and `{ X* }` (object values).

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
/// `open item+ close`, or no items when `open` is absent.
fn parse_optional_nonempty_group<T>(
    cursor: &mut TokenCursor<'_>,
    open: GraphQLTokenKind,
    close: GraphQLTokenKind,
    parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    match cursor.consume_token_if(open) {
        None => Vec::new().wrap_ok(),
        Some(_) => parse_nonempty_group_after_open(cursor, close, parse_item),
    }
}

/// `item+ close`, the open already consumed.
fn parse_nonempty_group_after_open<T>(
    cursor: &mut TokenCursor<'_>,
    close: GraphQLTokenKind,
    mut parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    let mut items = parse_item(cursor)?.wrap_vec();
    items.extend(parse_group_after_open(cursor, close, parse_item)?);
    items.wrap_ok()
}

/// `item* close`, the open already consumed.
fn parse_group_after_open<T>(
    cursor: &mut TokenCursor<'_>,
    close: GraphQLTokenKind,
    mut parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    std::iter::from_fn(|| match cursor.consume_token_if(close) {
        Some(_) => None,
        None if cursor.at_end() => cursor
            .expected(Expectation::Token(close))
            .wrap_err()
            .wrap_some(),
        None => parse_item(cursor).wrap_some(),
    })
    .collect()
}
```

`collect` into `Result<Vec<_>, _>` stops at the first error. A group open at the end of the file is `Expected(Token(close), EndOfFile)`. `{}` where the spec says `{ X+ }` fails as the first item parse at `}`.

Origin: upstream isograph `parse_optional_enclosed_items` (`../isograph/crates/graphql_schema_parser/src/parse_schema.rs`). Delta: the first item is required by a separate function; a group open at the end of the file reports the missing close instead of the missing item.

A list separated by `&` or `|` with an optional leading separator (spec 3.6 `ImplementsInterfaces`, 3.8 `UnionMemberTypes`, 3.13 `DirectiveLocations`):

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_separated_nonempty<T>(
    cursor: &mut TokenCursor<'_>,
    separator: GraphQLTokenKind,
    mut parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    cursor.consume_token_if(separator);
    let mut items = parse_item(cursor)?.wrap_vec();
    items.extend(
        std::iter::from_fn(|| cursor.consume_token_if(separator).map(|_| parse_item(cursor)))
            .collect::<Result<Vec<_>, _>>()?,
    );
    items.wrap_ok()
}
```

## Errors

```rust
// from crates/graphql_schema_parser/src/parse_error.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
    #[error("This float does not parse as a 64-bit float.")]
    InvalidFloat,
    #[error("Enum values cannot be true, false, or null.")]
    EnumValueCannotBeTrueFalseOrNull,
    #[error("This escape sequence does not encode a Unicode scalar value.")]
    InvalidUnicodeEscape,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, derive_more::Display)]
#[display("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum Expectation {
    #[strum(to_string = "{0}")]
    Token(GraphQLTokenKind),
    #[strum(to_string = "the keyword `{0}`")]
    Keyword(&'static str),
    #[strum(to_string = "a definition (schema, scalar, type, interface, union, enum, input, or directive)")]
    DefinitionKeyword,
    #[strum(to_string = "a definition (schema, scalar, type, interface, union, enum, input, or directive) or extend")]
    DefinitionOrExtensionKeyword,
    #[strum(to_string = "schema, scalar, type, interface, union, enum, or input")]
    ExtensionKeyword,
    #[strum(to_string = "a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[strum(to_string = "a constant value, like 42, 1.5, \"bar\", true, null, an enum value, a list, or an object")]
    ConstantValue,
    #[strum(to_string = "query, mutation, or subscription")]
    OperationType,
    #[strum(to_string = "a directive location, like FIELD_DEFINITION")]
    DirectiveLocation,
    #[strum(to_string = "a directive, like '@deprecated'")]
    Directive,
    #[strum(to_string = "`implements`, a directive, or '{{'")]
    ImplementsDirectiveOrOpenBrace,
    #[strum(to_string = "a directive or '='")]
    DirectiveOrEquals,
    #[strum(to_string = "a directive or '{{'")]
    DirectiveOrOpenBrace,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum Found {
    #[strum(to_string = "{0}")]
    Token(GraphQLTokenKind),
    #[strum(to_string = "end of file")]
    EndOfFile,
}

impl ParseError {
    pub fn expected(expected: Expectation, found: Found) -> Self {
        ParseError::Expected(ExpectedFound { expected, found })
    }
}
```

Origin: iso `crates/isograph_parser/src/parse_error.rs` (`AstError`, `ExpectedFound`, `Expectation`, `Found`, `AstError::expected`). Delta:

- `ParseError` is the one syntax error enum. Iso's `ParseError` wraps `AstError`, `BracketError`, and `CommaWithoutItem`; the latter two do not exist here. Iso's `EmptyLiteral` has no counterpart (an empty file has no entries, Pipeline above).
- `Expectation` derives `strum::Display`. Iso hand-writes `Display` (for `OneOf`) and implements `std::error::Error`; `Expectation` is not an error, has no `OneOf`, and has no `Error` impl.
- Iso's `Expectation` variants for chunks, selections, variables, and arguments are not here; the variants above are the schema grammar's.
- `Found::EndOfFile` replaces `Found::EndOfChunk`; there is no `Found::Group`.
- `InvalidFloat`, `EnumValueCannotBeTrueFalseOrNull` (spec 2.10.6 `EnumValue : Name but not true, false or null`, enforced in spec 3.9 `EnumValueDefinition`), and `InvalidUnicodeEscape` (spec 2.10.4) are new.

`InvalidFloat` is the `Err` of `str::parse::<f64>()` on a `FloatLiteral` token. The float regex produces no token known to fail that parse (an overflow parses as infinity). The variant exists because `f64::from_str` returns `Result` and the parser does not panic.

An error is `WithSpan<ParseError>`. The span is the offending token, or zero-width at `previous_end` when the file ended where a token was expected. `thiserror` formats `ParseError`.

Do not `match` a `Result` to bind success and convert the error. `let value = cursor.require_token(kind).map_err(|()| cursor.expected(expectation))?;`.

## Recovery

Recovery is at the top level only. A definition or extension whose parse fails becomes an `Err(WithSpan<DefinitionError>)` entry in `ParsedSchema.definitions` (ast.md), the cursor skips to the start of the next definition, and parsing continues. Inside a definition there is no recovery; its first error ends that definition.

The entry carries the name the failed definition declared when the parse got that far, so a lookup of that name sees the failure (integration.md). Every named definition parse reads its name first and attaches it to any later error:

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_after_entity_name<T>(
    cursor: &mut TokenCursor<'_>,
    parse_rest: impl FnOnce(
        &mut TokenCursor<'_>,
        WithSpan<EntityNameWrapper>,
    ) -> Result<T, WithSpan<ParseError>>,
) -> Result<T, DefinitionError> {
    let name = parse_entity_name(cursor).map_err(DefinitionError::unnamed)?;
    parse_rest(cursor, name).map_err(|error| DefinitionError {
        name: DefinitionName::Entity(name.item.0).wrap_some(),
        error,
    })
}

fn parse_after_directive_name<T>(
    cursor: &mut TokenCursor<'_>,
    parse_rest: impl FnOnce(
        &mut TokenCursor<'_>,
        WithSpan<DirectiveNameWrapper>,
    ) -> Result<T, WithSpan<ParseError>>,
) -> Result<T, DefinitionError> {
    let name = parse_directive_definition_name(cursor).map_err(DefinitionError::unnamed)?;
    parse_rest(cursor, name).map_err(|error| DefinitionError {
        name: DefinitionName::Directive(name.item.0).wrap_some(),
        error,
    })
}

impl DefinitionError {
    fn unnamed(error: WithSpan<ParseError>) -> Self {
        DefinitionError { name: None, error }
    }
}
```

`parse_entity_name` is `require_token(Identifier)` interned as `EntityNameWrapper`, mapped to `Expectation::Token(Identifier)`. `parse_directive_definition_name` requires `@` then the identifier, interned as `DirectiveNameWrapper`. A schema definition or extension has no name; its parse maps every error with `DefinitionError::unnamed`. So does a failed keyword.

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_definition_or_extension_or_skip(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<TypeSystemDefinitionOrExtension>, WithSpan<DefinitionError>> {
    let start = cursor.position();
    cursor
        .spanning(parse_type_system_definition_or_extension)
        .map_err(|error| {
            skip_failed_definition(cursor, start);
            error.with_span(cursor.span_since(start))
        })
}
```

The `Err` entry's span runs from the failed definition's first token to the last token the skip moved past: the text that definition occupied.

```rust
// from crates/graphql_schema_parser/src/recovery.rs
/// How many brackets of any kind are open. Never below zero: a close with no open leaves
/// the depth at the top level.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct BracketDepth(u32);

impl BracketDepth {
    const TOP_LEVEL: BracketDepth = BracketDepth(0);

    fn after(self, kind: GraphQLTokenKind) -> BracketDepth {
        match kind {
            GraphQLTokenKind::OpenBrace
            | GraphQLTokenKind::OpenParenthesis
            | GraphQLTokenKind::OpenBracket => BracketDepth(self.0 + 1),
            GraphQLTokenKind::CloseBrace
            | GraphQLTokenKind::CloseParenthesis
            | GraphQLTokenKind::CloseBracket => BracketDepth(self.0.saturating_sub(1)),
            _ => self,
        }
    }
}

/// Moves the cursor from a failed definition, which began at `start`, to the next token
/// that starts a definition at the top level, or to the end of the file.
pub(crate) fn skip_failed_definition(cursor: &mut TokenCursor<'_>, start: TokenIndex) {
    if cursor.position() == start {
        cursor.advance();
    }
    let mut depth = cursor
        .consumed_since(start)
        .iter()
        .fold(BracketDepth::TOP_LEVEL, |depth, token| depth.after(token.item));
    while let Some(token) = cursor.peek() {
        if depth == BracketDepth::TOP_LEVEL && starts_definition(cursor) {
            break;
        }
        depth = depth.after(token.item);
        cursor.advance();
    }
}

/// Whether the next tokens are a definition keyword or `extend`, or a description followed
/// by one.
fn starts_definition(cursor: &TokenCursor<'_>) -> bool {
    match cursor.peek().map(|token| token.item) {
        Some(GraphQLTokenKind::StringLiteral | GraphQLTokenKind::BlockStringLiteral) => cursor
            .peek_nth(1)
            .is_some_and(|token| is_top_level_keyword(cursor, token)),
        Some(GraphQLTokenKind::Identifier) => cursor
            .peek()
            .is_some_and(|token| is_top_level_keyword(cursor, token)),
        _ => false,
    }
}

fn is_top_level_keyword(cursor: &TokenCursor<'_>, token: WithSpan<GraphQLTokenKind>) -> bool {
    let text = &cursor.text()[token.location.as_usize_range()];
    token.item == GraphQLTokenKind::Identifier
        && (text == "extend" || DefinitionKeyword::from_str(text).is_ok())
}
```

The depth starts from the tokens the failed definition already consumed, so a failure inside `{ ... }` skips to that group's close before a keyword counts. A field named `type` inside the failed definition's braces is at depth 1 and does not restart parsing. When the failure is at the definition's first token (an unknown word at the top level), that token is skipped first so the loop always moves. When the failing token is itself a top-level keyword (`directive @a on scalar Date` fails at `scalar`, which is not a directive location), the skip stops there at once and `scalar Date` parses.

The depth counts all three bracket kinds as one counter and does not match kinds. There is no bracket tree; the counter lives in `skip_failed_definition` only.

A valid file never skips: each definition's parse ends at its last token, and the next definition's parse starts at the following token. A name that is also a keyword (a union member named `input`, a directive named `@type`) can restart parsing too early only while skipping a definition that already failed.

## Grammar function shapes

- `parse_*`: a form made of several tokens. Parameter is `&mut TokenCursor`. Returns `Result<T, WithSpan<ParseError>>`; the first `Err` is returned. A value made of several tokens gets its span from one `spanning` call around the function that parses it. The top-level definition parses return `Result<T, DefinitionError>` through `parse_after_entity_name` / `parse_after_directive_name`.
- `consume_*`: optional; `None` (or an empty vec) when the form is absent, without consuming.
- Dispatch on the next token peeks without consuming (`cursor.peek().map(|token| token.item)`), then calls a parse function that requires its first token. The last arm is `expected`.
- Dispatch on the next identifier's text is `consume_identifier_of::<K>`.

## Tests

`#[cfg(test)]` in this crate. Facts about the result; no snapshot files; no expected-output files. Degenerate cases for every form: empty file, empty group where the spec requires one item, end of file inside each group, the optional leading `&` / `|`, an unknown word at the top level, a definition that fails at its first token.

Test helper, `#[cfg(test)]`:

```rust
// from crates/graphql_schema_parser/src/test_cursor.rs
pub(crate) fn with_cursor<R>(text: &str, parse: impl FnOnce(&mut TokenCursor<'_>) -> R) -> R {
    let items = tokenize(text);
    parse(&mut TokenCursor::new(items.as_slice(), text))
}
```

`with_cursor` runs one grammar function on a fixture, for the changes in order.md that land before `parse_schema`. Origin: iso `parsed_items` (`crates/isograph_parser/src/parsed_items.rs`). Delta: no bracket matching, chunking, dummy parent stream, or semantic-token assertion; the caller's function receives the cursor and its result is returned. `span_of` is copied from iso `parsed_items.rs` unchanged.
