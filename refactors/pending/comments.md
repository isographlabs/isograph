# `#` comments

Requires the `SingleLineComment` token in `token_kind.rs` (Shared lexer). Independent of lsp-diagnostics.md. GraphQL schema parsing skips comments at the lexer (`logos::skip`) and is a different language (newlines are not separators); do not copy that skip into iso.

iso-format.md is pending. Until it lands, comments are recorded into the current `Vec<WithSpan<IsographSemanticToken>>`. After it lands, they are `Recorded::Token` with a `Layout`. A literal is formatted only when `errors.is_empty()`. Leftover is an error (`parse_stream` reports `Expected` on remaining contents). A comment that is leftover therefore refuses format. Preservation means comments are recorded at consume or attachment, not leftover.

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

That arm exists only if `NonBracketTokenKind` has the variant. Trivia (comments stripped before match-brackets) does not add the variant to `NonBracketTokenKind`. `IsographLangTokenKind::SingleLineComment` exists in every strategy.

`NoSemanticTokens::record` ignores comments. The compile path does not highlight.

Tests: `tokenize("# asdf\nx")` is `SingleLineComment`, `LineBreak`, `Identifier`. `tokenize("#")` is one `SingleLineComment`. `tokenize("\"#\"")` is one `StringLiteral`. `tokenize("\"\"\"#\"\"\"")` is one `BlockStringLiteral`. `tokenize("# a # b")` is one `SingleLineComment`. `tokenize("foo(# { bar })")` is `Identifier`, `OpenParenthesis`, `SingleLineComment` (the rest of the line, including `)`). `tokenize("foo(# { bar }\n)")` has `CloseParenthesis`.

## Recording and format

Until iso-format.md, `ItemCursor::record` pushes `IsographSemanticToken::Comment` at the comment's span into `tokens`. After iso-format.md:

Own-line comments: `Layout::OWN_LINE`.

Trailing comments of a list item: recorded after the item's last consume token and before `Recorded::Comma`, with `Layout { before: Gap::Space, after: Gap::None, indent: Indent::None }`. `Recorded::Comma` stays `Layout { before: Gap::None, after: Gap::Line, indent: Indent::None }` as iso-format.md has it. Printer `max`:

- value `GLUE` (`after: None`) then comment (`before: Space`) is `Space`: `b # yo`
- comment (`after: None`) then comma (`before: None`) is `None`: `# yo,`
- comma `after: Line` then next item: new line

Print is `a: b # yo,` then the next line. `Recorded::Comma` does not grow a comment list. Comma has no span; the highlighter walks `Recorded::Token`.

Recording the comment after the comma instead yields `max(comma.after: Line, comment.before: Space) = Line`, so `a: b,\n  # yo`. That is not the target. iso-format.md has no `lineSuffix`. Do not add one for this.

iso-format.md already breaks `(` / `{` / value `[` and indents interiors. Own-line `# asdf` is not a successful item, so no synthetic comma after it. The formatted motivating example is:

```
foo(
  # asdf
  a: b # yo,
)
```

Empty-of-slots with recorded comments (`foo(\n# asdf\n)`) is not the In/Out `Gap::None` packed `()`. Output is the broken group with the comments inside, no synthetic `Comma`.

A comment-only literal `# only` is not `EmptyLiteral` (there is a token). `require_token(Identifier)` fails. `errors` is not empty. Format refuses. Same for a comment with no declaration.

`willSaveWaitUntil` and `textDocument/formatting` both call `format_iso_literals`. A poisoned parse means save does not format that literal.

iso-format-later.md packs a group when the packed line is under the width. An `OWN_LINE` comment is a recorded `Gap::Line`, so the group does not pack. Trailing-only comments (`foo(a: b # yo)`) do not by themselves prevent packing; the packed form is `foo(a: b # yo,)`.

Descriptions are string tokens with `OWN_LINE`, not `#`.

Blank lines in source collapse: `Gap` has no blank-line variant. Two own-line comments in a row are one line between them.

## Strategy: trivia

Comments never reach match-brackets, chunking, or grammar. After tokenize, a pass attaches each comment to an adjacent remaining token and drops the comment from the vec. `require_token` does not skip comments. `parse_each_chunk` is unchanged. Grammar functions are unchanged. `NonBracketToken`, `OpenBracket`, and `CloseBracket` stay `Copy` newtypes. Trivia is not a field on those nodes: a field the tree carries and nothing on the tree reads is unused.

```rust
// from crates/isograph_parser/src/trivia.rs
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
    Vec<(Span, TokenTrivia)>,
)
```

Callers: `parse_iso_literal` runs `tokenize`, then `attach_trivia`, then `match_brackets` on the filtered vec. The second return is keyed by the surviving token's span. `commit` looks up that span, emits leading `Comment` with `OWN_LINE`, then the token, then trailing `Comment` with `{ before: Space, after: None, indent: None }`.

`IsographLangTokenKind::SingleLineComment` exists. `NonBracketTokenKind` does not gain it.

Host rules:

- If there is no `LineBreak` between the previous surviving token and the comment, the comment is trailing of that token. `a: b # yo` puts `# yo` on `b`. `a: b, # yo` puts `# yo` on the comma.
- Else if there is a later surviving non-`LineBreak` token, the comment is leading of the next such token. `LineBreak` tokens are not hosts. `# asdf\na` puts `# asdf` on `a`. The line break after the comment stays a line-break token unless absorb (below) takes it.
- Else the comment is trailing of the last surviving token, including across line breaks at EOF.
- Filtered vec empty (`# only`): no host. `parse_chunked_iso_literal` still fails the keyword. Format refuses.

The line break after `(` is still stripped by the matcher. Interior of

```
foo(
# asdf
a: b # yo
)
```

is: `a` (leading `# asdf`), `:`, `b` (trailing `# yo`). One chunk. `parse_argument` sees `a: b`. Both comments sit on tokens `commit` already emits. Trailing trivia of a source comma: iso-format synthesizes `Recorded::Comma` and does not print the source comma. `parse_each_chunk`, when it records that `Comma`, must also emit trailing trivia of the source comma that ended the chunk, still before the synthetic comma if we want `b # yo,`, or the comma token's trailing trivia is copied onto `b` at attach time when the comment follows `b` not the comma. `a: b # yo` attaches to `b`. `a: b, # yo` attaches to the comma. The second needs `parse_each_chunk` to pull trivia off the boundary comma when recording `Recorded::Comma`.

```
{
  foo # c
  bar
}
```

`foo` trailing `# c`, line break, `bar`. Two chunks. Language unchanged.

`{ # c\n}`: `# c` is leading of `}`. Printer: leading comments of `CLOSE_BRACE` emit before `}` at inner indent.

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

Callers unchanged. Body: walk `self.0`. If every content item is `NonBracket(NonBracketToken(SingleLineComment))`, classify against the previous content chunk, record `IsographSemanticToken::Comment` (own-line: `OWN_LINE`; same-line trailing: `{ before: Space, after: None, indent: None }`) into `parent`'s token vec, produce no slot. Else `parse_one_chunk` as today. After the last content chunk, remaining comments record as dangling into the token vec, no list field.

Comment-only chunks remain in the chunk tree and do not become slots. The `Vec<Slot>` is shorter than the chunk list. Positions on those comments answer the enclosing group.

Within a mixed chunk, skip (below) records the trailing comment and `require_end` succeeds. `parse_argument` does not mention comments.

Root is not `parse_each_chunk`. `skip_comments` plus `consume_line_breaks` at the existing call sites, including `parse_chunked_iso_literal` before `require_end`, covers leading comments. `remaining_contents` that skips comments covers trailing comments after the one declaration. A second declaration is still leftover.

`NonBracketTokenKind::SingleLineComment` exists. `SplitToken::from` / `From<NonBracketTokenKind>` gain the arm.

## Skip

Do not make `ItemCursor::peek` advance. `require_end`, `expected`, `spanning`, and `remaining_contents` call `self.items.peek()` / `next()` directly, not `ItemCursor::peek`. A skip that lives only in `peek` misses those four.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl ItemCursor<'_> {
    fn skip_comments(&mut self) {
        while let Some(peek) = self.items.peek() {
            match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::SingleLineComment,
                )) => {
                    let item = peek.commit();
                    self.record(IsographSemanticToken::Comment, item.location);
                    self.previous_end = item.location.end;
                }
                _ => break,
            }
        }
    }
}
```

After iso-format.md, that `record` takes `Layout { before: Gap::Space, after: Gap::None, indent: Indent::None }` (in-chunk comments are trailing). Own-line comments are recorded by `parse_each_chunk`, not skip, with `Layout::OWN_LINE`.

Call `skip_comments` at the start of `peek`, `require_end`, `expected`, `spanning`, `remaining_contents`, and `consume_line_breaks`. One record, on this commit. leftover fill-in span-dedup is the backstop. `spanning` skips first so the item span starts at the first non-comment. Trailing skip in `remaining_contents` runs after `spanning` returns, so the item span does not include the trailing comment. The enclosing group's span still covers it.

`chunk.rs` `absorb_chunk` uses a different iterator. Skip does not touch it.

This skip is required for specific locations (mixed-chunk trailing comments) and for skippable content. Trivia does not use it. Comments-as-separators does not use it inside chunked interiors; it still needs it at root (unchunked).

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

Printing that separator in order yields `a: b, # yo` or a newline then `# yo` without a same-line test.

`parse_one_chunk` for `Expectation::Separator(_)` (every list) does not fold `trailing_separator` into extra. Recording must read `chunk.trailing_separator` in `parse_each_chunk` after a successful item, emit each `Comment` from it (and the synthetic `Comma` in the same relative order as the source comma in that separator). Failed items have no synthetic comma; still record comments from that chunk's contents-as-comments (there should be none if comments are separators) and from its trailing separator.

Leading comments at a level: after `strip_captured_line_breaks`, a group can still open with `Comment` then `LineBreak`. `absorb_chunk` currently drops leading line breaks and errors on leading commas. Leading comments must not be dropped and must not be `CommaWithoutItem`. Record them as `OWN_LINE` as they are absorbed, attached to the first content chunk if one exists, otherwise dangling of the empty level (`foo(\n# asdf\n)`).

Root is unchunked. `chunk()` maps `to_content` over the root vec. `absorb_chunk` / `SeparatorToken` do not run at root. `# leading\nfield Query.x { id }` is a comment content item, then line break, then `field`. Comments-as-separators cannot leave `ItemCursor` unchanged: root still needs `skip_comments` at the existing `consume_line_breaks` / `require_end` sites.

`NonBracketTokenKind::SingleLineComment` exists so match-brackets can hand the token to chunking as a separator. It is not a `ChunkContentItem` after `absorb_chunk`. `separator_token` gains the `Comment` arm.

Separator tokens are not resolve leaves today. After grammar the chunk tree is gone. Hover is the enclosing group, same as the other strategies, because we recorded a `Comment` token whose span is the comment.

The claimed downside that `a: # what true` would split a chunk is the same-line fiction. That input has no `true` token under any strategy.

Cost: `absorb_chunk` leading-comment behavior, `SeparatorToken`, recording from separators in `parse_each_chunk`, and root skip.

## What "parser code unchanged" actually covers

`parse_selection`, `parse_argument`, `parse_name_colon`, `parse_non_constant_value`, `parse_iso_literal_item`, and the other grammar functions do not mention comments in any strategy. They call `require_token` / `peek` / `consume_group_if`.

Trivia: those functions, `ItemCursor`, `parse_each_chunk`, chunking, match-brackets unchanged aside from `commit` consulting the trivia table. New: tokenize token, `attach_trivia`, printer/record of trivia.

Specific locations: those grammar functions, chunking, match-brackets unchanged. Changed: `skip_comments` and the four cursor methods that must call it, `parse_each_chunk` for comment-only chunks, `NonBracketTokenKind`.

Comments-as-separators: grammar functions and in-chunk cursor unchanged. Changed: `SeparatorToken`, `absorb_chunk`, `parse_each_chunk` recording from separators, root skip, `NonBracketTokenKind`.

None of the strategies is zero parser-crate changes. The claim that holds is: grammar productions stay comment-free.

## Tests (behavior, once a strategy is chosen)

- `foo(\n# asdf\na: b # yo\n)` parses one argument `a: b`, errors empty, recorded comments at `# asdf` and `# yo`. Format contains both comment texts. Close paren is not inside a comment.
- `{ foo # c\nbar }` is two selections, errors empty, `# c` kept on `foo`.
- `{ foo, # c\nbar }` is two selections, `# c` kept on `foo` (not on `bar`).
- `{ foo,\n# c\nbar }` is two selections, `# c` on its own line between them (trailing separator of `foo` or leading of `bar`; print is a line with `# c` between the two items).
- `{ foo\n, # c\nbar }` is two selections, `# c` kept on `foo`.
- `{ # c\n}` errors empty, format keeps `# c` inside the braces.
- `foo(\n# asdf\n)` errors empty as a list of zero arguments with a dangling comment (not packed `()`).
- `# leading\nfield Query.x { id }` parses the field, `# leading` kept.
- `foo(# { bar }\n)` is not a brace group; `)` closes the paren.
- `foo(# { bar })` is an unclosed `(`.
- `{ # c }` is an unclosed `{`.
- `a: # what true` (same line): missing value. `true` is inside the comment.
- `a: # what\ntrue`: two chunks, not one argument, unless trivia absorb on `Colon` is in. `{ foo # c\nbar }` stays two selections even with that absorb.
- `id # c @loadable`: selection `id`, no directive.
- `id @ # c\nloadable`: `@` without a name, then a failed selection on `loadable`.
- `# only` as the whole literal: not empty of tokens, declaration fails, format refuses.
- `tokenize` / leftover / highlight: `Comment` role on the `#...` span, not `Error` plus `Content`.
- Nested `{ {\n# c\n} }`: inner empty set keeps `# c`.
- Comment after a description: `field Query.x\n"""desc""" # note\n{ id }` keeps `# note`.
- Unclosed `foo(\n# c` and unmatched `foo { bar ) # c\n}`: comments still recorded; format refuses because of the bracket error.
