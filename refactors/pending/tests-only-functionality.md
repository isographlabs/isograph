# Tests-only functionality

A production item exists only because something other than a test reads it. An API only tests call should not exist: a helper tests need lives under `#[cfg(test)]`. This file is the deletions from a sweep of the i2 crates. Each heading is independently shippable. No user-facing change.

Helpers already under `#[cfg(test)]` stay: `assert_semantic_tokens`, `parsed_items` / `span_of`, the `chunk_level` re-export in `lib.rs`.

`EndOfFile`, `Expectation::Description`, `Expectation::SelectionSet`, and `impl std::error::Error for Expectation` are specified in parser-minor-improvements.md. This file does not repeat them.

## Leave: production APIs whose only current callers are tests

These are the hover, diagnostics, and tokens stack. `handle` does not invoke them. `isograph_cli` does not depend on `isograph_lsp`. Tests are the current callers. The production callers are the pending docs named here. Do not delete these, and do not move them under `#[cfg(test)]`.

Hover, in `isograph_compiler`. Callers today: `memo_tests` in `isograph_extract_typescript`. Production caller: hover / goto / completion once those land.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
pub struct LineChar {
    pub line: u32,
    pub character: u32,
}

pub struct LiteralId {
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

pub fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
    line_char: LineChar,
) -> Option<LiteralId>

pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>>
```

Diagnostics, in `isograph_extract_typescript`. Callers today: tests in that crate. Production caller: lsp-diagnostics.md via `file_literals`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
pub fn host_errors_for_extraction(
    extraction: &IsoLiteralExtraction<TypeScriptHostLanguage>,
    parsed: &ParsedIsoLiteral,
) -> Vec<span::WithSpan<TypeScriptHostError>>

pub struct FileLiteral<'a> {
    pub extraction: &'a IsoLiteralExtraction<TypeScriptHostLanguage>,
    pub parsed: &'a ParsedIsoLiteral,
    pub errors: Vec<span::WithSpan<IsoLiteralError<TypeScriptHostLanguage>>>,
}

pub fn file_literals<'a>(
    db: &'a IsographState<TypeScriptHostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<FileLiteral<'a>>>
```

`IsoLiteralError` is the error type on `FileLiteral.errors`. It stays with `file_literals`.

Tokens, the `isograph_lsp` crate. No production crate depends on it. Production caller: lsp-tokens.md / lsp-dispatch.md.

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>>

// from crates/isograph_lsp/src/semantic_tokens.rs
pub fn lsp_semantic_tokens<'a>(
    page_content: &str,
    literals: impl IntoIterator<Item = (IsoLiteralStartIndex, &'a [WithSpan<IsographSemanticToken>])>,
) -> Vec<lsp_types::SemanticToken>

pub fn semantic_token_legend() -> SemanticTokensLegend
```

File memos that only those two call. They stay. `parsed_iso_literals_in_file` and `text_through_last_iso_literal` are called from `lsp_semantic_tokens_for_file`. `parsed_iso_literal` is called from those and from `file_literals`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
pub fn parsed_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    iso_literal_text: String,
) -> ParsedIsoLiteral

pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>>

pub fn text_through_last_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<String>
```

`isograph send` is a hidden CLI verb. Tests and CI are the users. It stays.

`common_lang_types` (`text_with_carats`, diagnostic constructors, `Location::as_embedded_location`, `LocationFreeDiagnostic::from_error`, `EntityNameAndSelectableName::{underscore_separated, relative_path}`) and prelude (`clone_err`, `drop_err`, `note_todo`) are the upstream keep. Not this file.

AST fields, token kinds, and semantic-token variants that parse records and resolve or LSP encoding read stay. Tests asserting them is not the reader that justifies them.

## `Chunk::contents_span`

`boundary_comma` and `first_item` have production callers in `variables.rs`. `contents_span` does not. Failed-parse span uses remaining first/last, not this method.

Delete the method.

```rust
// from crates/isograph_parser/src/chunk.rs
    /// First content item through last content item. `WithSpan<Chunk>` also covers
    /// the trailing separator.
    pub fn contents_span(&self) -> Span {
        Span::join(
            self.contents.first().location,
            self.contents.last().location,
        )
    }
```

The test that names it also pins `boundary_comma` `Some` and `first_item` on `{ foo, }`. Those stay. `a_chunk_without_a_comma_has_no_boundary_comma` is the `None` pair and is unchanged.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn contents_span_stops_at_the_last_content_item() {
        let text = "{ foo, }";
        let tree = chunked(text);
        let interior = as_group(&tree.item.0[0].item).children.item.0.reference();
        let top = interior[0].item.reference();
        assert_eq!(top.contents_span(), span_of(text, "foo"));
        assert_eq!(top.boundary_comma(), span_of(text, ",").wrap_some());
        assert_eq!(top.first_item().location, span_of(text, "foo"));
        match top.first_item().item.reference() {
            ChunkContentItem::NonBracket(token) => {
                assert_eq!(token.0, NonBracketTokenKind::Identifier);
            }
            other => panic!("expected the identifier, got {other:?}"),
        }
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn a_chunk_with_a_comma_has_that_comma_as_boundary_comma() {
        let text = "{ foo, }";
        let tree = chunked(text);
        let interior = as_group(&tree.item.0[0].item).children.item.0.reference();
        let top = interior[0].item.reference();
        assert_eq!(top.boundary_comma(), span_of(text, ",").wrap_some());
        assert_eq!(top.first_item().location, span_of(text, "foo"));
        match top.first_item().item.reference() {
            ChunkContentItem::NonBracket(token) => {
                assert_eq!(token.0, NonBracketTokenKind::Identifier);
            }
            other => panic!("expected the identifier, got {other:?}"),
        }
    }
```

`cargo test -p isograph_parser`.

## `WithErrors`

Extract no longer wraps item plus errors. `file_literals` builds `FileLiteral` instead. Nothing reads `WithErrors`. Delete the struct. `pub use host_language::*` drops it with the definition.

```rust
// from crates/isograph_compiler/src/host_language.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithErrors<T, E> {
    pub item: T,
    pub errors: E,
}
```

`cargo test -p isograph_compiler`.

## Unused `span` methods

Callers use the `item` field, `as_usize_range`, and `WithOptionalSpan`. These three methods have no callers, including tests. Delete them. `Display` for `WithGenericLocation<TItem, ()>` stays: `common_lang_types::WithNoLocation` is that type.

```rust
// from crates/span/src/lib.rs
    pub fn as_usize(self) -> (usize, usize) {
        (self.start as usize, self.end as usize)
    }
```

```rust
// from crates/span/src/lib.rs
    pub fn drop_location(self) -> WithGenericLocation<T, ()> {
        self.map_location(|_| ())
    }

    pub fn item(self) -> T {
        self.item
    }
```

`cargo test -p span` and `cargo test -p isograph_parser`.
