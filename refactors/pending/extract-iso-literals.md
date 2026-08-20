# Extract iso literals

A source file is JavaScript or TypeScript containing `iso(\`...\`)` (and the tagged-template form `iso\`...\``). Extraction is a regex over that file text. Each match is an `IsoLiteralExtraction`: the host-language facts (exported name, call form, associated JS function, the literal's span in the file) plus the backtick contents as a `&str`.

## Types

Most important first.

```rust
// from crates/isograph_parser/src/extract.rs
use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use prelude::Postfix;
use regex::Regex;
use span::Span;
use std::sync::LazyLock;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IsoLiteralExtraction<'a> {
    pub const_export_name: Option<ConstExportName>,
    pub iso_literal_text: &'a str,
    pub span: Span,
    pub call: IsoCall,
    pub associated_js_function: AssociatedJsFunction,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IsoCall {
    FunctionCall,
    TaggedTemplate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AssociatedJsFunction {
    Present,
    Absent,
}
```

Origin: `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs` in isograph (the `IsoLiteralExtraction` there). Delta:

```
const_export_name: Option<String>  -> Option<ConstExportName>
iso_literal_text: String           -> &'a str  (borrowed from the source)
iso_literal_start_index: usize     -> span: Span  (file-absolute, start and end of iso_literal_text)
has_associated_js_function: bool   -> associated_js_function: AssociatedJsFunction
iso_function_called_with_paren: bool -> call: IsoCall
```

`IsoCall::FunctionCall` is `iso(\`...\`)`. `IsoCall::TaggedTemplate` is `iso\`...\``. `AssociatedJsFunction::Present` is the `(` the regex reads after the iso call, the start of the resolver argument.

## Change 1: `extract_iso_literals`

`crates/isograph_parser/Cargo.toml` gains `regex`:

```toml
# from crates/isograph_parser/Cargo.toml
[dependencies]
common_lang_types = { path = "../common_lang_types" }
intern = { path = "../../relay-crates/intern" }
logos = { workspace = true }
nonempty = { workspace = true }
prelude = { path = "../prelude" }
regex = { workspace = true }
resolve_position = { path = "../resolve_position" }
resolve_position_macros = { path = "../resolve_position_macros" }
safe_peekable = { path = "../safe_peekable" }
scoped_stack = { path = "../scoped_stack" }
span = { path = "../span" }
strum = { workspace = true }
thiserror = { workspace = true }
```

Before: no `regex` line.

New module `crates/isograph_parser/src/extract.rs`, registered in `lib.rs`:

```rust
// from crates/isograph_parser/src/lib.rs
mod arguments;
mod chunk;
mod chunk_stream;
mod directives;
mod extract;
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

pub use arguments::*;
pub use chunk::*;
pub use directives::*;
pub use extract::*;
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

Before: no `mod extract` / `pub use extract::*`.

The regex is isograph's, with named groups. Origin pattern:

```
(// )?(export const ([^ ]+) =\s+)?iso(\()?\s*`([^`]+)`,?\s*(\))?(\()?
```

```rust
// from crates/isograph_parser/src/extract.rs
const EXTRACT_ISO_LITERAL_PATTERN: &str =
    r"(?<comment>// )?(export const (?<export_name>[^ ]+) =\s+)?iso(?<open_paren>\()?\s*`(?<literal>[^`]+)`,?\s*(?<close_paren>\))?(?<associated>\()?";

static EXTRACT_ISO_LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(EXTRACT_ISO_LITERAL_PATTERN)
        .expect("EXTRACT_ISO_LITERAL_PATTERN is a valid regex")
});
```

`expect`: the pattern is a string literal in this file. `Regex::new` returns `Err` only for invalid syntax; the type system cannot check regex syntax. The invariant is that this pattern compiles.

```rust
// from crates/isograph_parser/src/extract.rs
pub fn extract_iso_literals(source: &str) -> Vec<IsoLiteralExtraction<'_>> {
    EXTRACT_ISO_LITERAL
        .captures_iter(source)
        .filter_map(|captures| {
            if captures.name("comment").is_some() {
                return None;
            }
            let literal = captures.name("literal")?;
            IsoLiteralExtraction {
                const_export_name: captures
                    .name("export_name")
                    .map(|m| m.as_str().intern().to()),
                iso_literal_text: literal.as_str(),
                span: Span::from_usize(literal.start(), literal.end()),
                call: match captures.name("open_paren") {
                    Some(_) => IsoCall::FunctionCall,
                    None => IsoCall::TaggedTemplate,
                },
                associated_js_function: match captures.name("associated") {
                    Some(_) => AssociatedJsFunction::Present,
                    None => AssociatedJsFunction::Absent,
                },
            }
            .wrap_some()
        })
        .collect()
}
```

Origin: `extract_iso_literals_from_file_content` in isograph's `isograph_literals.rs`. Delta: plain function over `&str` (no pico, no path, no panic if a path is missing); named groups; `filter_map`; interned `ConstExportName`; `Span`; the two enums. A missing `literal` group skips the match (`?` on `Option`). `close_paren` is in the pattern so the associated `(` can match; it is not a field.

`iso_literal_text` is group `literal`, the text inside the backticks, not including them. `span` is that slice's range in `source`. Invariant: `&source[extraction.span.as_usize_range()] == extraction.iso_literal_text`. Empty backticks (`iso(\`\`)`) do not match: `[^`]+` needs at least one character.

### Tests

In `extract.rs` under `#[cfg(test)]`. Each test owns its `source` and asserts facts about the `Vec`.

```rust
// from crates/isograph_parser/src/extract.rs
#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::Span;

    use super::{
        AssociatedJsFunction, IsoCall, IsoLiteralExtraction, extract_iso_literals,
    };

    fn interned_export(name: &str) -> common_lang_types::ConstExportName {
        name.intern().to()
    }

    #[test]
    fn empty_source_extracts_nothing() {
        assert_eq!(extract_iso_literals(""), vec![]);
    }

    #[test]
    fn source_with_no_iso_extracts_nothing() {
        assert_eq!(extract_iso_literals("export const Foo = 1;"), vec![]);
    }

    #[test]
    fn exported_field_with_associated_function() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let text = "field Pet.fullName { id }";
        let start = source
            .find(text)
            .expect("the fixture contains the literal text");
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0],
            IsoLiteralExtraction {
                const_export_name: interned_export("fullName").wrap_some(),
                iso_literal_text: text,
                span: Span::from_usize(start, start + text.len()),
                call: IsoCall::FunctionCall,
                associated_js_function: AssociatedJsFunction::Present,
            }
        );
        assert_eq!(&source[extracted[0].span.as_usize_range()], text);
    }

    #[test]
    fn unexported_entrypoint() {
        let source = "iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].const_export_name, None);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(
            &source[extracted[0].span.as_usize_range()],
            extracted[0].iso_literal_text
        );
    }

    #[test]
    fn tagged_template() {
        let source = "iso`entrypoint Query.HomeRoute`";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].call, IsoCall::TaggedTemplate);
        assert_eq!(
            extracted[0].associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn commented_literal_is_skipped() {
        let source = "// export const Foo = iso(`field Pet.fullName { id }`)(";
        assert_eq!(extract_iso_literals(source), vec![]);
    }

    #[test]
    fn commented_literal_does_not_hide_a_later_one() {
        let source = "\
// export const Foo = iso(`field Pet.fullName { id }`)(
export const Bar = iso(`field Pet.bar { id }`)(";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].const_export_name, interned_export("Bar").wrap_some());
        assert_eq!(extracted[0].iso_literal_text, "field Pet.bar { id }");
    }

    #[test]
    fn two_literals_in_one_file() {
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].const_export_name,
            interned_export("fullName").wrap_some()
        );
        assert_eq!(extracted[1].const_export_name, None);
        assert_eq!(extracted[1].iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn nested_iso_inside_resolver() {
        let source = "\
export const HomeRoute = iso(`
  field Query.HomeRoute {
    pets {
      id
    }
  }
`)(function HomeRouteComponent({ data }) {
  iso(`entrypoint Query.PetFavoritePhrase`);
});";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].const_export_name,
            interned_export("HomeRoute").wrap_some()
        );
        assert_eq!(
            extracted[0].associated_js_function,
            AssociatedJsFunction::Present
        );
        assert_eq!(extracted[1].const_export_name, None);
        assert_eq!(
            extracted[1].iso_literal_text,
            "entrypoint Query.PetFavoritePhrase"
        );
        assert_eq!(
            extracted[1].associated_js_function,
            AssociatedJsFunction::Absent
        );
    }

    #[test]
    fn empty_backticks_are_not_extracted() {
        assert_eq!(extract_iso_literals("iso(``)"), vec![]);
    }

    #[test]
    fn const_without_export_has_no_name() {
        let source = "const Foo = iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].const_export_name, None);
    }

    #[test]
    fn extra_space_before_equals_drops_the_export_name() {
        let source = "export const Foo  = iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].const_export_name, None);
    }

    #[test]
    fn no_space_after_equals_drops_the_export_name() {
        let source = "export const Foo =iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].const_export_name, None);
    }

    #[test]
    fn trailing_comma_after_the_template() {
        let source = "iso(`entrypoint Query.HomeRoute`,)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].call, IsoCall::FunctionCall);
    }

    #[test]
    fn function_call_without_close_paren() {
        let source = "iso(`entrypoint Query.HomeRoute`";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].associated_js_function,
            AssociatedJsFunction::Absent
        );
    }

    #[test]
    fn whitespace_between_iso_and_backtick() {
        let source = "iso( `entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn multiline_literal() {
        let source = "iso(`\nentrypoint Query.HomeRoute\n`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "\nentrypoint Query.HomeRoute\n");
        assert_eq!(
            &source[extracted[0].span.as_usize_range()],
            extracted[0].iso_literal_text
        );
    }

    #[test]
    fn double_quoted_string_is_not_extracted() {
        assert_eq!(
            extract_iso_literals("iso(\"entrypoint Query.HomeRoute\")"),
            vec![]
        );
    }

    #[test]
    fn double_slash_without_space_does_not_count_as_comment() {
        let source = "//iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract_iso_literals(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }
}
```

`exported_field_with_associated_function` computes the expected span from `source.find` on the fixture text, not from the extraction. `&source[span] == iso_literal_text` is asserted there and on `unexported_entrypoint` and `multiline_literal`.
