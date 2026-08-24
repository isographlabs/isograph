# `#` comments

Requires the `SingleLineComment` token in `token_kind.rs` (Shared lexer). Independent of lsp-diagnostics.md. GraphQL schema parsing skips comments at the lexer (`logos::skip`) and is a different language (newlines are not separators); do not copy that skip into iso.

iso-format.md is pending. Until it lands, comments are recorded into the current `Vec<WithSpan<IsographSemanticToken>>`. After it lands, they are `WithOptionalSpan<IsographSemanticToken>`. A literal is formatted only when `errors.is_empty()`. Leftover is an error (`parse_stream` reports `Expected` on remaining contents). A comment that is leftover therefore refuses format. Preservation means comments are recorded at consume or attachment, not leftover.

This is a strategy discussion. It does not pick one. A later pass, with permission, turns one strategy into the implementation plan.

## What we are building

`#` to end of line is a comment. It is not an `Error` plus identifiers. Brackets inside a comment are comment text, not groups. The formatter keeps comments. Highlighting has a `Comment` role. Hover does not treat a comment as a resolve leaf: a position on a comment answers the enclosing node whose span covers it (the group from `consume_group_if`, or `IsoLiteralParse` at root, whose span is the whole literal). After grammar, resolve walks `IsoLiteralParse`. The chunk tree is a local of `parse_iso_literal` and is dropped. Comments are not fields on the tree.

The source

```
foo(
# asdf
a: b # yo
)
```

parses as one argument `a: b` with two comments, `errors` empty, and formats with both comment texts present. The close paren is on its own line. `foo(a: b # yo)` on one line is an unclosed `(`, because the comment eats `)`.

`a: # what\ntrue` is not a requirement. Trivia absorb on `Colon` / `Equals` is the only way it works without breaking one-chunk-one-item.

## The token is to end of line

```rust
// from crates/isograph_parser/src/token_kind.rs
    #[regex("#[^\n\r]*")]
    #[strum(to_string = "comment")]
    SingleLineComment,
```

`#[^\n\r]*` consumes the rest of the line. Spaces, commas, identifiers, brackets, and a closing `)` / `}` on that line are comment text, not later tokens. GraphQL is the same. There are no block comments.

A comment token is always the last token of its line, then `LineBreak` or EOF. A mixed chunk that contains a comment contains grammar tokens, then one comment. A mixed chunk cannot start with a comment: that would require a non-comment token after `#` on the same line.

`# a # b` is one token. `# c,` is one token; the comma is not `SeparatorToken::Comma`. `#` in `"..."` and `"""..."""` is string text. `foo#bar` is identifier `foo` then comment `#bar`. `#` at EOF with no line break is a comment. `# foo\r\n` is comment `# foo` then `LineBreak` `\r\n`.

These are unclosed groups, not comments-inside-groups:

```
{ # c }
foo(# { bar })
foo(a: b # yo)
```

Write them as `{ # c\n}`, `foo(# { bar }\n)`, `foo(\na: b # yo\n)`.

`id # c @loadable` parses as `id` with no directive: the rest of the line is commented out. `id @ # c loadable` is `@` without a name. `foo # c { bar }` on one line is scalar `foo`. `foo # c\n{ bar }` is two chunks, already pinned: a selection plus a failed selection on the orphaned group.

`a: # what true` on one line is `a`, `:`, comment `# what true`. There is no `true` token. Missing value is the correct parse. Same for `foo # c (x: 1)` and `Query # c .x`.

## Current behavior

```rust
// from crates/isograph_parser/src/token_kind.rs
    #[regex(r"[ \t\f\ufeff]+", logos::skip)]
    #[error]
    Error,
```

`# asdf` is `Error` (`#`) then `Identifier` (`asdf`). `foo(# { bar })` is an `Error`, then a brace group: the matcher sees the braces.

Spaces are skipped. Line breaks are tokens. Chunking splits on comma and line break. A group's leading line breaks are dropped by the matcher:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
fn strip_captured_line_breaks(items: &mut Vec<WithSpan<BracketItem>>)
```

and again by `absorb_chunk` if any leak. Root is unchunked (`ChunkedRoot`): line breaks there are content. `consume_line_breaks` runs in `parse_chunked_iso_literal` before `require_end`, and in `parse_iso_literal_item` / `parse_entrypoint` / `parse_selectable_declaration` at the sites in parsing-standards.md. Interiors are chunked. One chunk is one grammar item.

`parse_iso_literal` parses one declaration. A second is leftover (`Expectation::EndOfDeclaration`).

`ParsedIsoLiteral.tokens` is `Vec<WithSpan<IsographSemanticToken>>`. There is no `Layout` and no `format_recorded` until iso-format.md. `leftover_token` maps `Error` to `Error`, `LineBreak | EndOfFile` to `None`, other non-brackets to `Content`. LSP encoding (`crates/isograph_lsp/src/semantic_tokens.rs` `lsp_type_index`) is exhaustive: `Error => COMMENT`. A new `Comment` variant needs `Comment => COMMENT` there or the crate does not compile.

`a: b # yo` today: the argument parses, `#` and `yo` are leftover, `parse_stream` reports an error. `errors` is not empty.

`foo(\n# asdf\na: b)` today: after the `(` line break is stripped, `# asdf` is a chunk of `Error` plus `Identifier` with a trailing line break, then `a: b` is a second chunk. First chunk fails as an argument.

## The constraint

A line break ends a chunk. A comment does not include the line break. Two shapes that exist:

```
a: b # yo
```

One chunk: `a`, `:`, `b`, comment. Trailing line break is the separator.

```
foo(
# asdf
a: b
)
```

After the `(` line break is stripped: comment, line break, `a`, `:`, `b`. Two chunks. The first is comment-only.

```
a: # what
true
```

Two chunks: `a`, `:`, comment, then `true`. The value is not in the name-colon chunk. `parse_argument` on the first chunk has no value.

`foo # c\nbar` inside a set is two selections. That is pinned. Any rule that treats "line break after a comment" as not a separator also glues `foo # c\nbar` into one chunk. That is a language change.

`a: # what\ntrue` working without that language change means looking at the next chunk during parse (breaking one-chunk-one-item) or a trivia rule that absorbs the line break only after a token that cannot end an item (`:` and `=`). That optional rule is under Trivia.

## Shared lexer change

Uncomment `SingleLineComment` with `#[strum(to_string = "comment")]`. Skip regex stays `[ \t\f\ufeff]+`. Do not `logos::skip` comments. Do not delete `PeriodPeriod` / `Pipe` / `Spread` here; that is future-improvements.md.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub enum IsographSemanticToken {
    Comment,
    // existing variants
}
```

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
        IsographSemanticToken::Comment | IsographSemanticToken::Error => COMMENT,
```

`leftover_token` maps `SingleLineComment` to `Some(Comment)`, not `None`. Fill-in already skip-duplicates by span. Mapping to `None` makes a missed skip invisible. Mapping to `Comment` highlights the miss.

```rust
// from crates/isograph_parser/src/semantic_token.rs
        SplitToken::NonBracket(NonBracketTokenKind::SingleLineComment) => {
            IsographSemanticToken::Comment.wrap_some()
        }
```

Every strategy adds `NonBracketTokenKind::SingleLineComment` and both `From` arms. `From<IsographLangTokenKind> for SplitToken` is exhaustive; without the arm it does not compile. Trivia still drops comment tokens before `match_brackets`, so the variant does not appear in the chunk tree. leftover_token still needs the arm.

`NoSemanticTokens::record` ignores comments. The compile path does not highlight.

Tests: `tokenize("# asdf\nx")` is `SingleLineComment`, `LineBreak`, `Identifier`. `tokenize("#")` is one `SingleLineComment`. `tokenize("\"#\"")` is one `StringLiteral`. `tokenize("\"\"\"#\"\"\"")` is one `BlockStringLiteral`. `tokenize("# a # b")` is one `SingleLineComment`. `tokenize("foo(# { bar })")` is `Identifier`, `OpenParenthesis`, `SingleLineComment` (the rest of the line, including `)`). `tokenize("foo(# { bar }\n)")` has `CloseParenthesis`.

## Recording and format

iso-format.md tokens are `Vec<WithOptionalSpan<IsographSemanticToken>>`. There is no `Recorded` enum. `commit` still takes only the highlight role. `parse_each_chunk` records `Comma` with `location: None`. Source commas are not leftover-filled. `Comma` prints `,` then a line break. Space after `Keyword` / `Colon` / `Equals`. Space before `Keyword` / `Equals` / `Brace` / `At`. Everything else glues. List-group `{` `(` `[` indent and break. Empty open-then-close is `{}` `()` `[]`. Descriptions are `IsographSemanticToken::Description`, own line. The handler does not call `format_recorded` when `errors` is non-empty.

Until iso-format.md, `ItemCursor::record` pushes `IsographSemanticToken::Comment` at the comment's span into the current `Vec<WithSpan<IsographSemanticToken>>`. After iso-format.md, comments are `WithOptionalSpan` with role `Comment` and `Some(span)`.

`#` to end of line makes `a: b # yo,` not idempotent. Reparse of that line, the comment token is `# yo,`. The next format prints that slice plus another comma. The `)` of a packed one-line group on that same line is also comment text, and the `(` is unclosed. Do not print a comma (or a close bracket) on the same line after a comment.

The idempotent same-line form is `a: b, # yo` then a line break: the comma is a token before `#`. iso-format.md's `Comma` always line-breaks, so `Token(Comment)` immediately after `Comma` would print

```
a: b,
  # yo
```

which is also idempotent. Same-line `a: b, # yo` needs a printer extension: when `Comma` is followed by a trailing comment, print `, ` then the comment slice then a line break, and do not emit a second break. Own-line comments are a different placement: after that break, at the current indent, then another break.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub enum CommentPlacement {
    Trailing,
    OwnLine,
}

pub enum Recorded {
    Token(WithSpan<IsographSemanticToken>),
    Comma,
    Comment(WithSpan<CommentPlacement>),
}
```

Highlighter walks `Token` and `Comment`. `Comment` is `COMMENT`. `Comma` has no span.

Printer, extending iso-format.md, not replacing it with `Gap`:

- `Comma` then `Comment(Trailing)`: `, ` + slice + line break. Skip that `Comment`.
- `Comma` not followed by `Comment(Trailing)`: `,` + line break as today.
- `Comment(OwnLine)`: if not already at the start of a line, line break; then indent + slice + line break.
- `Comment(Trailing)` not after `Comma`: space + slice (root trailing, or trailing of a declaration that is not a list item).

Do not record a trailing comment before `Comma`. Skip inside a mixed chunk buffers comments and does not push them. `parse_each_chunk` after a successful item pushes `Comma`, then flushes same-line trailing comments as `Comment(Trailing)`. Own-line comment-only chunks record `Comment(OwnLine)` and do not push `Comma`.

The formatted motivating example:

```
foo(
  # asdf
  a: b, # yo
)
```

Empty-of-slots with recorded comments (`foo(\n# asdf\n)`) is not packed `()`. `Comment(OwnLine)` between open and close clears the empty-group case. No synthetic `Comma`.

`# only` as the whole literal is a comment token, not whitespace. `parse_chunked_iso_literal` currently treats leftover-empty after `consume_line_breaks` as `EmptyLiteral`. Skip must not sit on that `require_end`. Empty is line-breaks only. `# only` reaches `parse_iso_literal_item`, skip then `require_token(Identifier)` fails, `Expected` declaration keyword at the comment. Format refuses. Hover: `parse_stream` still builds a failed slot whose extra spans the comment; the `IsoLiteralParse` wrapper is the whole literal.

`willSaveWaitUntil` and `textDocument/formatting` both call `format_iso_literals`. A poisoned parse means save does not format that literal.

iso-format-later.md packs a group when the packed line is under the width. `Comment(OwnLine)` is a line break, so the group does not pack. A trailing comment on the last item is `, # yo` then a line break, then `)` / `}` on the next line. Packed `foo(a: b # yo,)` is not a format output: `)` cannot share a line with `#`.

Descriptions are `Description`, not `#`. `field Query.x\n"""desc""" # note\n{ id }`: trivia trailing of the string, or skip at root after the description. The printer already puts a description on its own line; `# note` is `Comment(Trailing)` on that line (`"""desc""", # note` is wrong: descriptions are not list items and have no synthetic comma) or `Comment(OwnLine)` after the description. Trailing of the description token: `"desc" # note` then the brace on the next line. Reparse: `# note` does not eat `{` because `{` is on the next line.

Blank lines in source collapse. Two `Comment(OwnLine)` in a row are one line between them.

iso-format.md is amended: `Recorded` grows `Comment(WithSpan<CommentPlacement>)`. The highlighter walks it. The printer arms above are this file. `Description` stays iso-format.md.

## Strategy: trivia

Comments never reach match-brackets, chunking, or grammar. After tokenize, a pass attaches each comment to an adjacent remaining token and drops the comment from the vec. `require_token` does not skip comments. Grammar functions are unchanged. `parse_each_chunk`'s item loop is unchanged except flushing comma trivia after `Comma`. `NonBracketToken`, `OpenBracket`, and `CloseBracket` stay `Copy` newtypes. Trivia is not a field on those nodes: a field the tree carries and nothing on the tree reads is unused.

```rust
// from crates/isograph_parser/src/trivia.rs
use std::collections::HashMap;

pub struct Comment {
    pub span: Span,
}

pub struct TokenTrivia {
    pub leading: Vec<Comment>,
    pub trailing: Vec<Comment>,
}

pub fn attach_trivia(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> (
    Vec<WithSpan<IsographLangTokenKind>>,
    HashMap<Span, TokenTrivia>,
)
```

Callers: `parse_iso_literal` runs `tokenize`, then `attach_trivia`, then `match_brackets` on the filtered vec. `Span` is `Hash`. `ItemCursor` holds the map. `commit` looks up the surviving token's span, emits leading `Comment(OwnLine)`, then the token, then trailing `Comment(Trailing)`. `parse_each_chunk` looks up `chunk.boundary_comma()`: trailing trivia of a source comma is flushed after the synthetic `Comma` as `Comment(Trailing)`.

`IsographLangTokenKind::SingleLineComment` exists. `NonBracketTokenKind` has it too (exhaustive `SplitToken::from`).

Host rules:

- If there is no `LineBreak` between the previous surviving token and the comment, the comment is trailing of that token. `a: b # yo` puts `# yo` on `b`. `a: b, # yo` puts `# yo` on the comma.
- Else if there is a later surviving non-`LineBreak` token, the comment is leading of the next such token. `LineBreak` tokens are not hosts. `# asdf\na` puts `# asdf` on `a`. The line break after the comment stays a line-break token unless absorb (below) takes it.
- Else the comment is trailing of the last surviving token, including across line breaks at EOF.
- Filtered vec empty (`# only`): no host. The root vec is empty. `EmptyLiteral`, `parse_chunked_iso_literal` returns `None`. Format refuses. There is no `IsoLiteralParse` for hover. Skip strategies instead fail the keyword with a slot whose extra is the comment.

The line break after `(` is still stripped by the matcher. Interior of

```
foo(
# asdf
a: b # yo
)
```

is: `a` (leading `# asdf`), `:`, `b` (trailing `# yo`). One chunk. `parse_argument` sees `a: b`. Both comments sit on tokens `commit` already emits. Trailing trivia of `b` is flushed after the synthetic `Comma` as `Comment(Trailing)` so print is `a: b, # yo`. Trailing trivia of a source comma (`a: b, # yo`) is the same flush, keyed on `chunk.boundary_comma()`. Do not emit a comment before `Comma`.

```
{
  foo # c
  bar
}
```

`foo` trailing `# c`, line break, `bar`. Two chunks. Language unchanged.

`{ # c\n}`: `# c` is leading of `}`. `commit` of `}` emits leading `Comment(OwnLine)` before the close.

Optional absorb: if the host of a trailing comment is `Colon` or `Equals`, and the next token is `LineBreak`, drop that `LineBreak` from the vec. Then `a: # what\ntrue` is one chunk. `foo # c\nbar` does not absorb: host is `Identifier`. `[Pet # c\n!]` does not. `foo @#c\nloadable` does not. `bar\n{ baz }` is unchanged. `$x: # c\nInt` would parse. `a: # c\n{ b: 1 }` would parse. `alias: # c\nname` would parse. `$x: Int = # c\n1` would parse. This is a language-shape change for those two token kinds only. It is not implied by trivia itself. It does not peek the next chunk.

Root: a comment before `field` is leading trivia of `field`. `consume_line_breaks` does not skip comments.

Cost: a new pass and a span-keyed table `commit` reads. Chunking and grammar stay comment-free.

## Strategy: specific locations

Comments remain tokens through tokenize, match-brackets, and chunking. They are content, so they are not separators. A comment on its own line is a comment-only chunk. Grammar functions do not mention comments. `parse_each_chunk` and `ItemCursor` do.

Locations we keep:

- Leading of a list item: own-line comments immediately before that item, including several comment-only chunks in a row.
- Trailing of a list item: the comment that is the last content of a mixed chunk (`a: b # yo`), and a comment-only chunk that is trailing of the previous content chunk under the rule below (`a: b, # yo`).
- Dangling of a list: comment-only chunks with no following item (`{\n# c\n}`, comments after the last item).
- Leading / trailing of the one root declaration: comments before the keyword, comments after the declaration in the unchunked root.

Locations we do not keep as a successful parse:

- `a: # what\ntrue` (value in the next chunk).
- A comment that splits a production across a line break in any other place.

A comment-only chunk parsed as an argument is a failed slot and poisons format (`errors` not empty). Skip those chunks as items.

A comment-only chunk is trailing of the previous content chunk iff the previous chunk has a last non-`LineBreak` token (a content token, or a `Comma` in `trailing_separator`) whose start is on the same source line as the comment's start. Do not use `WithSpan<Chunk>` (that span includes trailing line breaks; exclusive end can sit on the next line). Do not use `contents_span` (misses a comma on the next line).

```
a: b, # yo      last non-LineBreak is the comma, same line, trailing of a: b
a: b,\n# yo     comma on the previous line, own-line, leading of next
a: b\n, # yo    comma and comment on the same line, trailing of a: b
a: b\n# yo      last non-LineBreak is b, own-line, leading of next
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Comment {
    pub span: Span,
}

pub(crate) fn parse_each_chunk<P>(
    &self,
    parent: &mut ItemCursor<'_>,
    leftover: Expectation,
    parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
```

Callers unchanged. Body: walk `self.0` with peekable. If the chunk is comment-only and same-line trailing of the previous content chunk, buffer it. If comment-only and own-line, record `Comment(OwnLine)`, no slot. Else `parse_one_chunk`. Skip inside that parse buffers mixed-chunk trailing comments on the cursor and does not push them. After a successful item, push `Recorded::Comma`, then flush same-line buffered comments (cursor buffer plus following same-line comment-only chunks) as `Comment(Trailing)`. After a failed item, flush comments as `Comment(OwnLine)` without a comma; format already refuses. After the last content chunk, remaining own-line comments record as `Comment(OwnLine)`, no list field.

Same-line test: the previous chunk's last non-`LineBreak` token (content or a `Comma` in `trailing_separator`) through the comment's start, `text` contains no `\n` or `\r`. `ItemCursor::text` is the literal.

Comment-only chunks remain in the chunk tree and do not become slots. The `Vec<Slot>` is shorter than the chunk list. Positions on those comments answer the enclosing group.

`parse_argument` does not mention comments.

Root is not `parse_each_chunk`. Skip at `parse_iso_literal_item` / `parse_entrypoint` / `parse_selectable_declaration` next to the existing `consume_line_breaks` sites covers leading comments. `remaining_contents` that skips comments covers trailing comments after the one declaration. A second declaration is still leftover.

Type lists are not `parse_each_chunk`. `parse_bracket_interior_type` takes chunk 0 with `Expectation::EndOfType`, which folds `trailing_separator` into extra. `[# c\nFoo]` is a comment-only first chunk: failed type under a naive `parse_one_chunk` of chunk 0. `[Foo # c]` folds the comment into extra: leftover, highlighted, not printed. Either skip comment-only chunks in `parse_bracket_interior_type` the same way (and record them), or type-list comments error here and parse under trivia. Write the tests either way.

`NonBracketTokenKind::SingleLineComment` exists. `SplitToken::from` / `From<NonBracketTokenKind>` gain the arm.

## Skip

Do not make `ItemCursor::peek` advance. `require_end`, `expected`, `spanning`, and `remaining_contents` call `self.items.peek()` / `next()` directly, not `ItemCursor::peek`. A skip that lives only in `peek` misses those four.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    buffered_comments: Vec<Comment>,
    // existing fields
}

impl ItemCursor<'_> {
    fn skip_comments(&mut self) {
        while let Some(peek) = self.items.peek() {
            match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::SingleLineComment,
                )) => {
                    let item = peek.commit();
                    self.buffered_comments.push(Comment {
                        span: item.location,
                    });
                    self.previous_end = item.location.end;
                }
                _ => break,
            }
        }
    }
}
```

Do not push to the token vec here. `parse_each_chunk` flushes `buffered_comments` after `Comma`. leftover fill-in span-dedup is the backstop if a path forgets to flush.

Call `skip_comments` at the start of `peek`, `expected`, `spanning`, and `remaining_contents`. Do not call it from `require_end`. Do not call it from `consume_line_breaks`. Empty-literal detection in `parse_chunked_iso_literal` is: every root item is a `LineBreak`. Then `parse_stream`. `parse_iso_literal_item` calls `skip_comments` then `consume_line_breaks` then `require_token(Identifier)`. `# only` is `Expected` the declaration keyword at the comment, not `EmptyLiteral`.

If `peek` skips comments, `consume_line_breaks` that uses `peek` will skip comments too. The empty check must not use that `peek`. Line-breaks-only is decided on the root vec before the stream.

`spanning` skips first so the item span starts at the first non-comment. Trailing skip in `remaining_contents` runs after `spanning` returns, so the item span does not include the trailing comment. The enclosing group's span still covers it.

`chunk.rs` `absorb_chunk` uses a different iterator. Skip does not touch it.

This skip is required for specific locations (mixed-chunk trailing comments) and for skippable content. Trivia does not use it. Comments-as-separators does not use it inside chunked interiors; it still needs it at root (unchunked), called from `parse_iso_literal_item` not from `require_end`.

## Strategy: skippable content

Comments are content tokens. Skip (above) plus comment-only-chunk handling (specific locations) makes grammar functions unchanged.

Because a comment eats the rest of the line, skip inside a mixed chunk only ever sees a comment as the last content item. The extra power that would have been "comment between `:` and `true` on the same line" is not a token stream this lexer can produce. Crossing a line break mid-production still fails: the cursor is one chunk.

What remains is specific locations. Do not implement this as a third cursor model. If skippable content is chosen, implement specific locations.

`a: # what\ntrue` still two chunks. Making it work here is merging chunks or a cursor that spans the level. That is a redesign of `parse_each_chunk` / `ItemCursor`, not an opaque skip.

## Strategy: comments as separators

A variant of specific locations. `SeparatorToken` grows a comment:

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum SeparatorToken {
    Comma,
    LineBreak,
    Comment,
}
```

A comment is always last on its line, so in a chunked interior it always abuts a boundary. `a: b # yo\n` is one chunk, trailing separator comment plus line break. `a: b, # yo\n` is one chunk, trailing comma, comment, line break. `parse_each_chunk`'s per-item `parse_item` is unchanged. `ItemCursor` inside a chunked interior is unchanged: comments are not content.

Own-line comments between items land in the previous chunk's trailing separator, in source order with the commas and line breaks:

```
a: b, # yo\nc: d     trailing_separator: Comma, Comment, LineBreak
a: b,\n# yo\nc: d    trailing_separator: Comma, LineBreak, Comment, LineBreak
a: b\n# yo\nc: d     trailing_separator: LineBreak, Comment, LineBreak
```

`parse_one_chunk` for `Expectation::Separator(_)` (every list) does not fold `trailing_separator` into extra. All `parse_each_chunk` callers pass `Expectation::Separator`. Recording reads `chunk.trailing_separator` in `parse_each_chunk` after a successful item: emit `Recorded::Comma` at the source comma (or the synthetic comma if the boundary had no comma), and `Comment(Trailing)` at each `SeparatorToken::Comment` that sits on the same line as the last non-line-break of that boundary, `Comment(OwnLine)` at each comment after a line break in that boundary. Do not record during `absorb_chunk`. Failed items have no synthetic comma; format already refuses.

`separator_as_content` gains `SeparatorToken::Comment => NonBracketTokenKind::SingleLineComment` so extra-fold on non-list `parse_one_chunk` (`Expectation::EndOfType`) can hold comments.

Leading comments at a level: after `strip_captured_line_breaks`, a group can still open with `Comment` then `LineBreak`. `absorb_chunk` currently drops leading line breaks and errors on leading commas. The first-item loop needs a `SeparatorToken::Comment` arm that does not drop and does not `CommaWithoutItem`. Store them on the level; record in `parse_each_chunk`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedLevel {
    pub leading_comments: Vec<Comment>,
    pub chunks: Vec<WithSpan<Chunk>>,
}
```

`ChunkedLevel` is a newtype `ChunkedLevel(pub Vec<WithSpan<Chunk>>)` today. Two fields. `parse_each_chunk` records `leading_comments` as `Comment(OwnLine)` first. If the level has no content chunks, those are dangling of the empty group (`foo(\n# asdf\n)`).

Root is unchunked. `chunk()` maps `to_content` over the root vec. `absorb_chunk` / `SeparatorToken` do not run at root. `# leading\nfield Query.x { id }` is a comment content item, then line break, then `field`. Comments-as-separators cannot leave `ItemCursor` unchanged: root still needs `skip_comments` from `parse_iso_literal_item`.

`NonBracketTokenKind::SingleLineComment` exists so match-brackets can hand the token to chunking as a separator. It is not a `ChunkContentItem` after `absorb_chunk`. `separator_token` gains the `Comment` arm.

Separator tokens are not resolve leaves today. After grammar the chunk tree is gone. Hover is the enclosing group, same as the other strategies, because we recorded a `Comment` token whose span is the comment.

The claimed downside that `a: # what true` would split a chunk is the same-line fiction. That input has no `true` token under any strategy.

Cost: `absorb_chunk` leading-comment behavior, `SeparatorToken`, recording from separators in `parse_each_chunk`, and root skip.

## What "parser code unchanged" actually covers

`parse_selection`, `parse_argument`, `parse_name_colon`, `parse_non_constant_value`, `parse_iso_literal_item`, and the other grammar functions do not mention comments in any strategy. They call `require_token` / `peek` / `consume_group_if`.

Trivia: those functions, `ItemCursor` skip, `parse_each_chunk` item loop, chunking, match-brackets unchanged aside from `commit` consulting the trivia map and `parse_each_chunk` flushing comma trivia after `Comma`. New: tokenize token, `attach_trivia`, printer arms for `Recorded::Comment`.

Specific locations: those grammar functions, chunking, match-brackets unchanged. Changed: `skip_comments` (not on `require_end`), `parse_each_chunk` for comment-only chunks and comma-then-trailing-comment order, `parse_bracket_interior_type` or documented type-list errors, `NonBracketTokenKind`, printer arms.

Comments-as-separators: grammar functions and in-chunk cursor unchanged. Changed: `SeparatorToken`, `absorb_chunk` first-item `Comment` arm, `ChunkedLevel` leading comments, `parse_each_chunk` recording from separators, root skip from `parse_iso_literal_item`, `NonBracketTokenKind`, printer arms.

None of the strategies is zero parser-crate changes. The claim that holds is: grammar productions stay comment-free.

## Tests (behavior, once a strategy is chosen)

- `foo(\n# asdf\na: b # yo\n)` parses one argument `a: b`, errors empty, recorded `Comment(OwnLine)` at `# asdf` and `Comment(Trailing)` at `# yo`. Format is `foo(\n  # asdf\n  a: b, # yo\n)`. Close paren is not inside a comment. Reparse comment slices equal the previous parse's comment slices.
- `{ foo # c\nbar }` is two selections, errors empty, format keeps `# c` on the `foo` line as `foo, # c` then `bar,`.
- `{ foo, # c\nbar }` is two selections, `# c` trailing of `foo`.
- `{ foo,\n# c\nbar }` is two selections, `# c` own-line between them.
- `{ foo\n, # c\nbar }` is two selections, `# c` trailing of `foo`.
- `{ # c\n}` errors empty, format keeps `# c` inside the braces, not packed `{}`.
- `foo(\n# asdf\n)` errors empty as a list of zero arguments with a dangling comment, not packed `()`.
- `# leading\nfield Query.x { id }` parses the field, `# leading` kept own-line before `field`.
- `foo(# { bar }\n)` is not a brace group; `)` closes the paren.
- `foo(# { bar })` is an unclosed `(`.
- `{ # c }` is an unclosed `{`.
- `a: # what true` (same line): missing value. `true` is inside the comment.
- `a: # what\ntrue`: two chunks, not one argument, unless trivia absorb on `Colon` is in. `{ foo # c\nbar }` stays two selections even with that absorb.
- `id # c @loadable`: selection `id`, no directive.
- `id @ # c\nloadable`: first slot fails (`@` without a name), second slot is selection `loadable`, errors not empty, format refuses.
- `# only`: not `EmptyLiteral`; `Expected` declaration keyword at the comment; format refuses.
- `tokenize` / leftover / highlight: `Comment` role on the `#...` span, not `Error` plus `Content`.
- Nested `{ {\n# c\n} }`: inner empty set keeps `# c`.
- Comment after a description: `field Query.x\n"""desc""" # note\n{ id }` keeps `# note`.
- Unclosed `foo(\n# c`: the matcher cuts the unclosed group; the comment is not in a chunk. Recorded only once leftover fill-in walks `tokenize` (semantic-tokens.md). Format refuses on the bracket error.
- `[# c\nFoo]` and `[Foo\n# c]` and `[Foo # c]`: type-list behavior as chosen above (skip and keep, or error).
- A group whose last line is a `#` comment always has `)` / `}` on the next line.
