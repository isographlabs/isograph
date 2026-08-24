# `#` comments

Requires the comment token in `token_kind.rs` (this file, Shared lexer change). Independent of lsp-diagnostics.md. iso-format.md's printer is the reader that makes preservation matter: a parse with errors is not formatted, and leftover `#` is an error, so a commented literal is never formatted. GraphQL schema parsing skips comments at the lexer (`logos::skip`) and is a different language (newlines are not separators); do not copy that skip into iso.

This is a strategy discussion. It does not pick one. A later pass, with permission, turns one strategy into the implementation plan.

## What we are building

`#` to end of line is a comment. It is not an `Error` plus identifiers. Brackets inside a comment are comment text, not groups. The formatter keeps comments. Hover does not treat a comment as a resolve leaf: a position on a comment answers the parent whose span covers it (the chunk, the slot, or the list). Highlighting has a `Comment` role.

The source

```
foo(
# asdf
a: b # yo
)
```

parses as one argument `a: b` with two comments, and formats with both comments present. `a: # what\ntrue` is not a requirement. Same-line `a: # what true` is the same production with a comment between `:` and the value; some strategies get it for free.

## Current behavior

```rust
// from crates/isograph_parser/src/token_kind.rs
    #[regex(r"[ \t\f\ufeff]+", logos::skip)]
    #[error]
    Error,
```

`# asdf` is `Error` (`#`) then `Identifier` (`asdf`). `foo(# { bar })` is an `Error`, then a brace group: the matcher sees the braces. The commented-out `SingleLineComment` regex is the token we actually want:

```rust
// from crates/isograph_parser/src/token_kind.rs
    // #[regex("#[^\n\r]*")]
    // SingleLineComment,
```

Spaces are skipped. Line breaks are tokens. Chunking splits on comma and line break. A group's leading line breaks are dropped by the matcher:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
fn strip_captured_line_breaks(items: &mut Vec<WithSpan<BracketItem>>)
```

and again by `absorb_chunk` if any leak. Root is unchunked (`ChunkedRoot`): line breaks there are content, and `consume_line_breaks` skips them at the declaration sites listed in parsing-standards.md. Interiors are chunked. One chunk is one grammar item (`parse_one_chunk` / `parse_each_chunk`).

iso-format.md records consume tokens plus a synthetic comma after every successful list item, and prints that stream. Leftover is `Recorded::Leftover` and is not printed. A literal with any parse error is not passed to `format_recorded`.

`a: b # yo` today: the argument parses, `#` and `yo` are leftover, `parse_stream` reports `Expected(separator, ...)`. The literal has errors. Format will refuse it.

`foo(\n# asdf\na: b)` today: after the `(` line break is stripped, `# asdf` is a chunk of `Error` plus `Identifier` with a trailing line break, then `a: b` is a second chunk. First chunk fails as an argument. Same refusal.

## The constraint

A line break ends a chunk. A comment token is `#[^\n\r]*` and does not include the line break. Three shapes:

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

`foo # c\nbar` inside a set is two selections. That is pinned: a line break after a complete item starts the next item. Any rule that treats "line break after a comment" as not a separator also glues `foo # c\nbar` into one chunk (`foo` plus leftover `bar`, or a failed extra). That is a language change.

`a: # what\ntrue` working without that language change means looking at the next chunk during parse (breaking one-chunk-one-item) or a trivia rule that absorbs the line break only after a token that cannot end an item (`:` and `=`). That optional rule is under Trivia.

## Shared lexer change

Every strategy starts here. This is the language-shape change future-improvements.md named. It is not leftover fill-in.

```rust
// from crates/isograph_parser/src/token_kind.rs
    #[regex("#[^\n\r]*")]
    #[strum(to_string = "comment")]
    SingleLineComment,
```

Delete the commented-out `SingleLineComment` / `PeriodPeriod` / `Pipe` / `Spread` variants, the TODO on the skip regex, and the matching commented-out Display arms. Skip regex stays `[ \t\f\ufeff]+`. Do not `logos::skip` comments.

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
pub enum NonBracketTokenKind {
    SingleLineComment,
    // existing variants
}
```

`SplitToken::from` / `From<NonBracketTokenKind>` gain the arm. `leftover_token` maps `SingleLineComment` the way it maps `LineBreak`: `None`, once comments are recorded at consume or attachment. Until a strategy records them, leftover would highlight them as `Content`; that is a hole the strategy must close.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub enum IsographSemanticToken {
    Comment,
    // existing variants
}
```

A comment at EOF with no line break is a comment. `#` inside a string is string text. `foo#bar` is identifier `foo` then comment `#bar`. `# { }` is one token; the matcher does not see braces.

Tests: `tokenize("# asdf\nx")` is `SingleLineComment` then `LineBreak` then `Identifier`. `tokenize("foo(# { bar })")` has no brace tokens. `tokenize("#")` is one `SingleLineComment`. `tokenize("\"#\"")` is one `StringLiteral`.

## Strategy: trivia

Comments never reach match-brackets, chunking, or grammar. After tokenize, a pass attaches each comment to an adjacent remaining token and drops the comment from the vec. `require_token` does not skip comments. `parse_each_chunk` is unchanged. Grammar functions are unchanged.

```rust
// from crates/isograph_parser/src/trivia.rs
pub enum Trivia {
    Comment(Span),
}

pub struct TokenTrivia {
    pub leading: Vec<Trivia>,
    pub trailing: Vec<Trivia>,
}
```

`NonBracketToken` is a newtype over `NonBracketTokenKind`. Trivia on that newtype makes it a named-field struct:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub struct NonBracketToken {
    pub kind: NonBracketTokenKind,
    pub trivia: TokenTrivia,
}
```

Every `NonBracketToken(kind)` construction becomes `NonBracketToken { kind, trivia: TokenTrivia { leading: vec![], trailing: vec![] } }` or a helper. Open and close brackets need the same trivia: a comment before `{` is leading trivia of the open. `OpenBracket` / `CloseBracket` are newtypes today; same named-field change, or trivia lives only on a wrapper around `WithSpan<IsographLangTokenKind>` before the matcher, and the matcher copies it onto the node it builds.

Attachment, one pass over the token vec, comments and spaces already the only skippable things:

- A comment with no line break between the previous remaining token and the comment is trailing trivia of that previous token. `a: b # yo` puts `# yo` on `b`. `a: b, # yo` puts `# yo` on the comma.
- Every other comment is leading trivia of the next remaining non-line-break token. Line-break tokens are skipped when choosing the host. `# asdf\na` puts `# asdf` on `a`. The line break after the comment stays a line-break token unless the optional absorb rule below takes it.

The line break after `(` is still stripped by the matcher. Interior of

```
foo(
# asdf
a: b # yo
)
```

is: `a` (leading `# asdf`), `:`, `b` (trailing `# yo`). One chunk. `parse_argument` sees `a: b`. Both comments sit on tokens the printer already emits.

```
{
  foo # c
  bar
}
```

`foo` trailing `# c`, line break, `bar`. Two chunks. Language unchanged.

```
a: # what
true
```

`:` trailing `# what`, line break, `true`. Two chunks. Not one argument.

Optional absorb: if the host of a trailing comment is `Colon` or `Equals`, fold the immediately following line break into that trailing trivia and drop the line-break token. Then `a: # what\ntrue` is one chunk. `foo # c\nbar` does not absorb: the host is `Identifier`. `[Pet # c\n!]` does not absorb: host is `Identifier`; bang still does not attach. `foo\n@loadable` is unchanged (no comment). `bar\n{ baz }` is unchanged. `$x: # c\nInt` would parse. `a: # c\n{ b: 1 }` would parse. This rule is a language-shape change for those two token kinds only. It is not implied by trivia itself.

`TokenTrivia` on every token is a field the grammar does not read. The readers are highlighting and format. Recording: when `commit` records a token, emit each leading `Trivia::Comment` as `IsographSemanticToken::Comment` on its own line before the token, and each trailing comment as `Comment` with a space before and glue after. Comma tokens that are separators are not `commit`ted (iso-format synthesizes `Recorded::Comma`). Trailing trivia of a source comma, or of the last content token when there is no source comma, must still enter the Recorded stream. Two ways:

- `parse_each_chunk` after a successful item, before `Recorded::Comma`, records trailing comments of the chunk's last content token; after the comma, records trailing comments of the boundary comma if the source comma carried them.
- Or the printer, when emitting a token, also walks that token's trivia. Then `RecordedToken` carries `TokenTrivia`, and separator commas need trivia too (`Recorded::Comma` grows a comment list, or source commas become printable).

The first keeps trivia off `Recorded` and puts comments in the stream as `Comment` tokens. The second keeps comments on the tokens. Either way the printer sees them. iso-format.md currently prints `Recorded::Consume` and `Recorded::Comma`. Trivia that never becomes a printable `Recorded` is dropped. That is the "formatter kills comments" failure again.

Leading trivia of the first token of a list item prints as own-line comments at the current indent before the item. Trailing trivia prints on the same line after the item. Print order relative to the synthetic comma:

```
a: b, # yo
```

records the comma, then the trailing comment (`lineSuffix` in prettier: the comment flushes before the next line break). The other order, `a: b # yo,`, puts the comment between the value and the comma. The target for the motivating example, once `(` groups break and indent (iso-format.md), is:

```
foo(
  # asdf
  a: b, # yo
)
```

Dangling comments in `{\n# c\n}`: no content token to attach to. The close brace is the next remaining token; `# c` is leading trivia of `}`. Printer: `CLOSE_BRACE` has `before: Line`, leading comments emit before `}` at the inner indent. Empty `{ # c }` on one line: `# c` is trailing of `{` or leading of `}`; either prints inside.

Root: `ChunkedRoot` is not chunked, but trivia still attaches to tokens. A comment before `field` is leading trivia of `field`. `consume_line_breaks` does not need to skip comments.

Cost: `NonBracketToken` / bracket newtypes become named-field structs, every construction site, `SplitToken` and match arms that currently match `NonBracketToken(kind)`. Chunking and grammar stay comment-free. Attachment is a new pass with its own tests (the four shapes above, comma-then-comment, dangling, EOF comment, comment before `}`, comment after `(`).

## Strategy: specific locations

Comments remain tokens through tokenize, match-brackets, and chunking. They are content, so they are not separators. A comment on its own line is a comment-only chunk. Grammar functions (`parse_argument`, `parse_selection`, `parse_name_colon`, ...) do not mention comments. `parse_each_chunk` does.

Locations we keep:

- Leading of a list item: own-line comments immediately before that item, including several comment-only chunks in a row.
- Trailing of a list item: comments in the same chunk after the grammar item (`a: b # yo`), and comment-only chunks that sit on the same line as the previous item's trailing separator (`a: b, # yo`).
- Dangling of a list: comment-only chunks with no following item (`{\n# c\n}`, `{ # c }`, comments after the last item).
- Leading / trailing of the one root declaration: comments before the keyword, comments after the declaration in the unchunked root.

Locations we do not keep as successful parse (they remain leftover or failed slots, and then the literal is not formatted):

- `a: # what\ntrue` (value in the next chunk).
- A comment that splits a production across a line break in any other place (`Query # c\n.foo` at root still has a line-break token between them; `require_token(Period)` does not skip line breaks).

`parse_each_chunk` today maps every chunk to a slot:

```rust
// from crates/isograph_parser/src/chunk.rs
    pub(crate) fn parse_each_chunk<P>(
        &self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>> {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_chunk(
                    chunk,
                    parent.stream_chunk(&chunk.item),
                    leftover,
                    &parse_item,
                )
            })
            .collect()
    }
```

A comment-only chunk parsed as an argument is a failed slot and poisons format. Skip those chunks as items. Buffer their comments. When the next content chunk parses, those comments are leading of that item. When no next content chunk exists, they are dangling of the list. A comment-only chunk whose first comment starts on the same line as the previous chunk's end (no line break between previous chunk span end and comment start) is trailing of the previous item, not leading of the next. That is the `a: b, # yo` case: the comma ended the first chunk, the comment is the next chunk, and attaching it forward would print `# yo` on `c: d`.

```
a: b, # yo
c: d
```

chunks as `a: b` (trailing comma), comment-only `# yo` (trailing line break), `c: d`. Same-line as previous → trailing of `a: b`.

```
a: b,
# yo
c: d
```

comment-only `# yo` is not on the previous chunk's line → leading of `c: d`.

Within a mixed chunk, `ItemCursor::peek` / `require_token` / `require_end` / `remaining_contents` skip comments and record them as trailing of the item being parsed. That is the same skip as Strategy: skippable content, but only inside the chunk `parse_item` already owns. `parse_argument` still does not mention comments: `require_token(Identifier)` skips `# yo` after `b` and `require_end` succeeds.

```rust
// from crates/isograph_parser/src/chunk.rs
struct Comment {
    span: Span,
}

enum PendingComments {
    None,
    Leading(NonEmpty<Comment>),
    TrailingOfPrevious(NonEmpty<Comment>),
}
```

`parse_each_chunk` holds `pending: Vec<Comment>`. For each chunk:

- If every content item is `NonBracket(SingleLineComment)`, classify against the previous content chunk's span, record the comments, and do not produce a slot.
- Else `parse_one_chunk` as today. Before `parse_item`, the cursor's first items may be comments (mixed chunk that starts with a comment). Peek-skip records those as leading of this item, in addition to `pending` leading from previous comment-only chunks.

Dangling comments are recorded after the last slot, still in the parent's `tokens` vec, in source order. They do not need a field on `ArgumentList` if the printer walks `Recorded`. They do need a host for resolve: the list's unmatched span, or a dangling field on the list. A field only the printer reads belongs on `Recorded`, not on the tree. Resolve on a dangling comment answers the list.

`Slot` does not grow comment fields unless something on the tree reads them. Highlighting and format read `Recorded`.

Root: `parse_stream` on `ChunkedRoot`, not `parse_each_chunk`. `consume_line_breaks` plus peek-skip covers leading comments. `remaining_contents` that skips comments covers trailing comments after the one declaration. A second declaration is still leftover.

Cost: `parse_each_chunk` becomes a small state machine (pending comments, same-line vs own-line). Cursor skip is shared with the third strategy. Grammar parse functions stay unchanged. Chunking stays unchanged. Comment-only chunks remain in the chunk tree and disappear from the parse tree; positions on them answer the list (or the next/previous slot if we put comments on `Slot`).

## Strategy: skippable content (comments anywhere the cursor walks)

Comments are content tokens. `ItemCursor::peek` skips them (advances and records once). `require_token`, `consume_token_if`, `consume_group_if`, `require_end`, `remaining_contents`, `expected`, and `spanning` all go through that skip, so grammar functions are unchanged. A comment between any two tokens of the same chunk is invisible to `parse_argument`.

```
a: # what true
```

One chunk. `require_token` after `:` skips the comment, consumes `true`. Parses. Recorded order is name, colon, comment, value. The printer emits `a: # what true` if it prints in stream order. Reattaching that comment as trailing of the value is a format-only choice and is not required to parse.

`peek` must consume comments on the first call, not re-record on the fifth. Skip is `advance` plus record, then peek the next non-comment. `expected` and `require_end` today call `self.items.peek()` directly and would still see comments unless they use the same skip.

This does not cross chunk boundaries. `a: # what\ntrue` is still two chunks. `require_token` cannot skip into the next chunk: the cursor is one chunk. Making it work here is merging chunks or a cursor that spans the level. That is a redesign of `parse_each_chunk` / `ItemCursor`, not an opaque skip.

Comment-only chunks still fail unless `parse_each_chunk` skips them the way Strategy: specific locations does. Opaque consume without that skip means

```
foo(
# asdf
a: b # yo
)
```

still has a failed first slot, errors, no format. The motivating example does not parse. So skippable content for the motivating example includes the comment-only-chunk handling of specific locations. The extra power is same-chunk mid-production comments (`a: # what true`, `foo # c (x: 1)`, `Query # c .x` on one line).

`foo # c (x: 1)` is one chunk today (no line break). Skip makes it a selection with arguments. Trailing comment on `foo` vs comment between name and arguments is a print choice; parse is the same.

Crossing a line break mid-production still fails. "Comments anywhere" in the GraphQL sense (newlines are whitespace, `a: # what\ntrue` is one argument) is not this strategy. That is either the optional trivia absorb, or a language change that line breaks are no longer separators, which contradicts the pins in future-improvements.md.

Cost: cursor skip plus the `parse_each_chunk` comment-only handling. Grammar files unchanged. Every `items.peek()` that is not the skip itself is a bug. `consume_line_breaks` does not skip comments (line breaks are not comments); at root, peek-skip handles comments, then `consume_line_breaks` handles line breaks, and the order of those two at each call site must be "skip both." Folding comments into `consume_line_breaks` (skip comments and line breaks) is a rename: `skip_trivia` that does not skip commas. Commas are not trivia; they are leftover or separators.

## Comments as separators

A variant of specific locations. `SeparatorToken` grows a comment:

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum SeparatorToken {
    Comma,
    LineBreak,
    Comment,
}
```

`a: b # yo\n` is one chunk, trailing separator comment plus line break. `a: b, # yo\n` is one chunk, trailing comma, comment, line break. The comment never starts a chunk. `parse_each_chunk` is unchanged. Cursor is unchanged. Grammar is unchanged.

Own-line `# asdf` before the first item is a leading separator run. `absorb_chunk` currently drops leading line breaks and errors on leading commas. Leading comments must not be dropped and must not be `CommaWithoutItem`. They attach as leading comments of the first content chunk, or as dangling of the level if the level is only comments.

`a: # what true` becomes two chunks (`a:` then `true`) because the comment is now a separator in the middle of the production. Same-line mid-item comments, the thing skippable content gets for free, break. That is the opposite trade of skippable content.

`a: # what\ntrue` is still two chunks. Same as the other strategies without absorb.

Formatter reads `Chunk::trailing_separator` (and leading comments on the first chunk) in source order relative to the synthetic comma. iso-format.md does not walk separators today except as leftover on failed extra. `parse_each_chunk` would record comments from the separator when it records `Recorded::Comma`.

This variant fits "comments only at item boundaries" tightly. It fights "comment between `:` and the value on the same line."

## Formatting

iso-format.md prints consume tokens and synthetic commas. Comments that are not in that stream are killed. Every strategy has to put comments into that stream (as `IsographSemanticToken::Comment`, or as trivia the printer walks). Leftover-only is not enough: leftover is not printed, and leftover is an error, and errors refuse the literal.

Own-line comments use a line break on both sides. Trailing comments use a space before and glue after; the next line break (list-group close, comma, or the next own-line comment) ends the line. Trailing comments must not force an extra blank line before the next item.

Blank lines: a comment does not eat an extra line break beyond the one that ended its source line. Two comments in a row:

```
# a
# b
id
```

two own-line comments, then `id`. Adjacent own-line comments share one line break, not a blank line. A blank line in source (`# a\n\n# b`) is two line breaks and collapses to one line. Preserving blank lines is not in iso-format.md and is not part of comments.

`format_recorded` of a failed parse still does not run. If a strategy leaves a comment as a failed slot, the user writes a comment and save no longer formats the literal. That is the acceptance bar for the motivating example: both comments, parse errors empty, format runs, both comments in `new_text`.

iso-format.md always breaks `(`. Trailing-only comments (`foo(a: b # yo)`) still sit on the item line: `foo(\n  a: b, # yo\n)`. Pick the comma-then-comment order so a trailing comment does not sit between value and comma. Packing a short list back onto one line is iso-format-later.md.

Descriptions are string tokens the printer already puts on their own line, not comments. `#` is not a description. A description and a comment on a field:

```
field Query.x
# note
"desc"
{
  id,
}
```

is a root-level comment between `x` and the description. Root is unchunked. Peek-skip or trivia on `"desc"` (leading) keeps it. Print: comment own-line, description own-line, brace.

## What "parser code unchanged" actually covers

`parse_selection`, `parse_argument`, `parse_name_colon`, `parse_non_constant_value`, `parse_iso_literal_item`, and the other grammar functions do not mention comments in any of the three strategies. They call `require_token` / `peek` / `consume_group_if`.

Unchanged in trivia: those functions, `ItemCursor`, `parse_each_chunk`, chunking, match-brackets (aside from carrying trivia on the nodes). New: tokenize token, attachment pass, printer/record of trivia.

Unchanged in specific locations and skippable content: those grammar functions, chunking, match-brackets. Changed: `ItemCursor` skip, `parse_each_chunk` for comment-only chunks. Skippable content is specific locations plus skip inside mixed chunks; specific locations of trailing-in-chunk comments already need that skip, so the two collapse at the cursor.

Unchanged in comments-as-separators: grammar functions, `ItemCursor`, `parse_each_chunk`. Changed: `SeparatorToken`, `absorb_chunk` leading-comment behavior, recording from separators. Mid-item same-line comments become chunk boundaries.

None of the strategies is zero parser-crate changes. The claim that holds is: grammar productions stay comment-free; skip or attachment is in the cursor, the chunker, or a trivia pass.

## Tests (behavior, once a strategy is chosen)

- `foo(\n# asdf\na: b # yo\n)` parses one argument `a: b`, errors empty, recorded comments at `# asdf` and `# yo`. Format contains both comment texts.
- `{ foo # c\nbar }` is two selections, errors empty, `# c` kept on `foo`.
- `{ foo, # c\nbar }` is two selections, `# c` kept on `foo` (not on `bar`).
- `{ foo,\n# c\nbar }` is two selections, `# c` kept on `bar` (own-line between items).
- `{ # c }` errors empty, format keeps `# c` inside the braces.
- `# leading\nfield Query.x { id }` parses the field, `# leading` kept.
- `foo(# { bar })` is not a brace group.
- `a: # what true` (same line): parses as `a: true` under trivia and skippable content; two failed chunks under comments-as-separators.
- `a: # what\ntrue`: two chunks, not one argument, unless trivia absorb on `Colon` is in. `{ foo # c\nbar }` stays two selections even with that absorb.
- `tokenize` / leftover / highlight: `Comment` role on the `#...` span, not `Error` plus `Content`.
