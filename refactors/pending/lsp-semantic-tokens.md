# LSP semantic tokens

Requires `HostLanguage` in `crates/isograph_compiler` (landed) and lsp-semantic-token-encoding.md (landed). send-events.md is the daemon event socket: JSON `IsographEvent` frames, `isograph send`. This crate does not send events and does not speak LSP JSON-RPC.

`file_literals` extracts and parses every `iso(\`...\`)` in a source string. `lsp_tokens_for_file` rebases those tokens onto the file and calls `lsp_semantic_tokens`. The LSP adapter (event-model.md item 7) will call these on `OpenFile` else `DiskFile`. `didOpen` / `didChange` / `didClose` become `EditorChanged` in that adapter. `isograph lsp` is that adapter's stdio proxy. This slice does not start a server.

Two shippable changes: extract and parse, then concatenate and encode.

## What the user does

No user-facing coloring until the adapter. Tests construct a TypeScript source string, extract, encode, and assert the LSP integers.

## Types

Most important first.

```rust
// from crates/isograph_lsp/src/file_literals.rs
use isograph_compiler::{HostLanguage, IsoLiteralError};
use isograph_parser::{IsoLiteralParse, IsographSemanticToken, parse_iso_literal};
use span::WithSpan;

pub struct FileLiteral<'a, THostLanguage: HostLanguage> {
    pub extraction: WithSpan<(&'a str, THostLanguage::LiteralContext)>,
    pub parse: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<IsoLiteralError<THostLanguage>>>,
    pub tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub fn file_literals<THostLanguage: HostLanguage>(
    host: &THostLanguage,
    source: &str,
) -> Vec<FileLiteral<'_, THostLanguage>> {
    host.extract_iso_literals(source)
        .into_iter()
        .map(|extracted| {
            let extraction = extracted.item;
            let text = extraction.item.0;
            let parsed = parse_iso_literal(text);
            FileLiteral {
                extraction,
                parse: parsed.item,
                errors: extracted.errors,
                tokens: parsed.tokens,
            }
        })
        .collect()
}
```

`parse` is `None` when `ParsedIsoLiteral.item` is `None` (empty literal). `tokens` are literal-relative, consume order, whatever the grammar recorded. extra and extra_chunks leftover is leftover-semantic-tokens.md. The matcher's cut is not filled in. `errors` is `WithErrors.errors`: `IsoLiteralError` (`Host`, `Parse`). `Parse` is pipeline `ParseError` (`Ast`, `Bracket`, `Comma`), already file-absolute.

## Change 1: `file_literals`

encoding.md created `crates/isograph_lsp` with `lsp_semantic_tokens` and `semantic_token_legend`. This change adds `file_literals`.

```toml
# from crates/isograph_lsp/Cargo.toml
[package]
name = "isograph_lsp"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
isograph_compiler = { path = "../isograph_compiler" }
isograph_parser = { path = "../isograph_parser" }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
span = { path = "../span" }

[dev-dependencies]
common_lang_types = { path = "../common_lang_types" }
intern = { path = "../../relay-crates/intern" }
isograph_extract_typescript = { path = "../isograph_extract_typescript" }

[lints]
workspace = true
```

Production `isograph_lsp` does not depend on `isograph_extract_typescript`. Tests that open a TypeScript fixture take `TypeScriptHostLanguage` through the dev-dependency.

```rust
// from crates/isograph_lsp/src/lib.rs
mod file_literals;
mod semantic_tokens;

pub use file_literals::{FileLiteral, file_literals};
pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};
```

`file_literals.rs` is the types above. Tests in that module:

```rust
// from crates/isograph_lsp/src/file_literals.rs
#[cfg(test)]
mod tests {
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use isograph_parser::{IsoLiteralItem, IsographSemanticToken};
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpanPostfix};

    use super::file_literals;

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

    #[test]
    fn exported_field_is_one_file_literal() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let literals = file_literals(&TypeScriptHostLanguage, source);
        assert_eq!(literals.len(), 1);
        assert_eq!(
            literals[0].extraction.item.1.const_export_name,
            "fullName".intern().to::<common_lang_types::ConstExportName>().wrap_some()
        );
        assert!(matches!(
            literals[0]
                .parse
                .as_ref()
                .expect("the fixture is not an empty literal")
                .item
                .item
                .item
                .item
                .as_ref()
                .expect("the fixture parsed a declaration")
                .item,
            IsoLiteralItem::Selectable(_)
        ));
        assert_eq!(literals[0].errors, vec![]);
        assert!(
            literals[0]
                .tokens
                .contains(&IsographSemanticToken::Keyword.with_span(span_of(
                    literals[0].extraction.item.0,
                    "field"
                )))
        );
    }

    #[test]
    fn two_literals_in_one_file() {
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let literals = file_literals(&TypeScriptHostLanguage, source);
        assert_eq!(literals.len(), 2);
        assert!(matches!(
            literals[1]
                .parse
                .as_ref()
                .expect("the fixture is not an empty literal")
                .item
                .item
                .item
                .item
                .as_ref()
                .expect("the fixture parsed a declaration")
                .item,
            IsoLiteralItem::Entrypoint(_)
        ));
    }
}
```

`with_span` needs `WithSpanPostfix` in the test module.

## Change 2: concatenate literals, call `lsp_semantic_tokens`

encoding.md landed `semantic_token_legend` and `lsp_semantic_tokens`. This change concatenates each `FileLiteral`'s tokens rebased with `with_offset(extraction.location.start)` and calls `lsp_semantic_tokens`.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use isograph_compiler::HostLanguage;
use span::WithSpanPostfix;

use crate::file_literals::file_literals;

pub fn lsp_tokens_for_file<THostLanguage: HostLanguage>(
    host: &THostLanguage,
    source: &str,
) -> Vec<lsp_types::SemanticToken> {
    let literals = file_literals(host, source);
    let mut tokens = Vec::new();
    for literal in &literals {
        tokens.extend(literal.tokens.iter().map(|token| {
            token
                .item
                .with_span(token.location.with_offset(literal.extraction.location.start))
        }));
    }
    lsp_semantic_tokens(&tokens, source)
}
```

`tokens` after the loop are offsets into `source`, in extraction order. `lsp_semantic_tokens` asserts they are ordered, exclusive, and have text. Extraction order is source order.

```rust
// from crates/isograph_lsp/src/lib.rs
pub use semantic_tokens::{lsp_semantic_tokens, lsp_tokens_for_file, semantic_token_legend};
```

Tests:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
#[cfg(test)]
mod file_tests {
    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{CLASS, KEYWORD, PROPERTY, lsp_tokens_for_file};

    #[test]
    fn field_keyword_is_the_first_lsp_token() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let lsp = lsp_tokens_for_file(&TypeScriptHostLanguage, source);
        let field_at = source.find("field").expect("the fixture contains field") as u32;
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, field_at);
        assert_eq!(lsp[0].length, 5);
        assert_eq!(lsp[0].token_modifiers_bitset, 0);
    }

    #[test]
    fn pet_is_class_and_id_is_property() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let lsp = lsp_tokens_for_file(&TypeScriptHostLanguage, source);
        assert_eq!(lsp[1].token_type, CLASS);
        assert_eq!(lsp[5].token_type, PROPERTY);
    }

    #[test]
    fn two_literals_tokens_are_in_file_order() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let lsp = lsp_tokens_for_file(&TypeScriptHostLanguage, source);
        assert_eq!(lsp.len(), 8);
        assert_eq!(lsp[4].delta_line, 1);
        assert_eq!(lsp[4].delta_start, 5);
        assert_eq!(lsp[4].token_type, KEYWORD);
    }
}
```

`file_tests` is a second `#[cfg(test)]` module in `semantic_tokens.rs` so encoding.md's `tests` module stays as written. `KEYWORD` / `CLASS` / `PROPERTY` are the legend indices encoding.md already defines.

`field_keyword_is_the_first_lsp_token`: `field` is the first parser token in the extracted literal. `delta_start` is its file offset.

`pet_is_class_and_id_is_property`: `Pet` is index 1, `id` is index 5 (`field`, `Pet`, `.`, `fullName`, `{`, `id`).

`two_literals_tokens_are_in_file_order`: eight tokens. Index 4 is the second `entrypoint`. Previous piece is `A` on the previous line; `delta_line` 1, `delta_start` 5 (column of `entrypoint` after `iso(\``).
