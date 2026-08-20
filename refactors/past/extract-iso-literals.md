# Extract iso literals

Iso literals exist in files: an isograph source string occupies a span in a file. Finding that string, and checking that the host-language embedding is valid, is `THostLanguage: HostLanguage`. `extract_iso_literals` returns `Vec<WithErrors<WithSpan<(&'a str, Self::LiteralContext)>, Vec<WithSpan<IsoLiteralError<Self>>>>>`. `item` is the isograph string and host context. `errors` is empty when there were none. `IsoLiteralError` is `Host` or `Parse` (pipeline `ParseError` from parse-iso-literal-entry.md). How the host finds the string is not common. Host facts live on `THostLanguage::LiteralContext`, not on `SelectableDeclaration`.

The first implementor is `TypeScriptHostLanguage`: the same regex isograph uses on JavaScript and TypeScript source. It finds `iso(\`...\`)` and `iso\`...\`` and puts the interior of the backticks as `item.item.0`.

## Crates

```
isograph_parser
    ^
isograph_compiler          HostLanguage, WithErrors, IsoLiteralError
    ^
isograph_extract_typescript    TypeScriptHostLanguage
    ^
ts_graphql_react_isograph_cli    names TypeScriptHostLanguage

isograph_compiler
    ^
isograph_lsp               generic over HostLanguage
    ^
isograph_cli               generic bootstrap: run given the type params
    ^
ts_graphql_react_isograph_cli
```

`isograph_parser` is the literal grammar. It does not export `HostLanguage`. It does not depend on `isograph_compiler`. It does not name TypeScript.

`isograph_compiler` owns the seam trait. It depends on `isograph_parser` (`ParseError`). It contains no implementor.

`isograph_extract_typescript` is the TypeScript implementor. `isograph_parser`, `isograph_compiler`, `isograph_lsp`, and `isograph_cli` do not depend on it.

`isograph_cli` is a library. It bootstraps the daemon, lifecycle verbs, and LSP given the type params (`HostLanguage` now; `NetworkProtocol` and `GenerateArtifacts` when those seams exist). It does not name `TypeScriptHostLanguage`.

`ts_graphql_react_isograph_cli` is the consumer of `isograph_cli` (ts-graphql-react-isograph-cli.md). It is the crate that knows about TypeScript: it constructs `TypeScriptHostLanguage` and passes it to `isograph_cli`. Binary name stays `isograph`. This doc does not change that crate; extract tests in `isograph_extract_typescript` are the consumer here. lsp-semantic-tokens.md is the first time the binary names `TypeScriptHostLanguage`.

## Types

Most important first. Compiler crate: the trait. TypeScript crate: the implementor. `ts_graphql_react_isograph_cli` names the implementor. `isograph_cli` is generic over the trait.

```rust
// from crates/isograph_compiler/src/host_language.rs
use span::WithSpan;
use thiserror::Error;

use isograph_parser::ParseError;

pub trait HostLanguage: Sized {
    type Error: std::fmt::Display + std::error::Error;
    type LiteralContext;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<
        WithErrors<
            WithSpan<(&'a str, Self::LiteralContext)>,
            Vec<WithSpan<IsoLiteralError<Self>>>,
        >,
    >;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithErrors<T, E> {
    pub item: T,
    pub errors: E,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum IsoLiteralError<THostLanguage: HostLanguage> {
    #[error("{0}")]
    Host(THostLanguage::Error),
    #[error("{0}")]
    Parse(ParseError),
}
```

`IsoLiteralError` derives `Error`. Spans on those errors are the `WithSpan` wrapping each `IsoLiteralError`. Host errors use the extraction span (`WithErrors.item.location`). Pipeline `ParseError` uses the inner span rebased with `with_offset`. `WithErrors` does not implement `Error`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use isograph_compiler::{HostLanguage, IsoLiteralError, WithErrors};
use isograph_parser::{
    IsoLiteralItem, IsoLiteralParse, ParseError, SelectableNameWrapper, parse_iso_literal,
};
use prelude::Postfix;
use regex::Regex;
use span::{Span, WithSpan, WithSpanPostfix};
use std::sync::LazyLock;
use thiserror::Error;

pub struct TypeScriptHostLanguage;

pub struct TypeScriptLiteralContext {
    pub const_export_name: Option<ConstExportName>,
    pub call: IsoCall,
    pub associated_js_function: AssociatedJsFunction,
}

pub enum IsoCall {
    FunctionCall,
    TaggedTemplate,
}

pub enum AssociatedJsFunction {
    Present,
    Absent,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum TypeScriptHostError {
    #[error(
        "You must call the iso function with parentheses. \"iso`...`\" is not supported"
    )]
    MissingParentheses,
    #[error(
        "This isograph field literal must be exported as a named export, for example as `export const {suggested_name}`"
    )]
    MissingExport { suggested_name: SelectableNameWrapper },
    #[error("Isograph literals must be immediately called, and passed a function")]
    MissingAssociatedFunction,
}
```

Derives: `TypeScriptHostLanguage` is `Copy, Clone, Debug, Default, PartialEq, Eq`. `WithErrors` is `Clone, Debug, PartialEq, Eq`. `TypeScriptLiteralContext`, `IsoCall`, `AssociatedJsFunction`, `TypeScriptHostError` are `Copy, Clone, Debug, PartialEq, Eq`. `isograph_parser` and `isograph_compiler` do not depend on `regex` or on the TypeScript crate.

The string's location in the file is `WithErrors.item`'s `WithSpan`. For `TypeScriptHostLanguage`, that span is the interior of the backticks, and `&source[item.location.as_usize_range()] == item.item.0`. Host errors share that span.

`IsoCall::FunctionCall` is `iso(\`...\`)`. `IsoCall::TaggedTemplate` is `iso\`...\``. `AssociatedJsFunction::Present` is the `(` the regex reads after the iso call, the start of the resolver argument.

Origin of the item: isograph `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs` `IsoLiteralExtraction`. Delta: unnamed tuple `(&'a str, THostLanguage::LiteralContext)`; host facts on `TypeScriptLiteralContext`; location is `WithErrors.item.location`; return is `WithErrors` (`item` / `errors`); `IsoLiteralError` is `Host` / `Parse`.

```rust
// from crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs (upstream)
pub struct IsoLiteralExtraction {
    pub const_export_name: Option<String>,
    pub iso_literal_text: String,
    pub iso_literal_start_index: usize,
    pub has_associated_js_function: bool,
    pub iso_function_called_with_paren: bool,
}
```

```rust
// from crates/isograph_compiler/src/host_language.rs
WithSpan<(&'a str, Self::LiteralContext)>
```

## Change 1: `HostLanguage` in the compiler

New crate `crates/isograph_compiler`. Workspace member via `./crates/*`. Parser `Cargo.toml` does not gain `regex` and does not depend on this crate.

```toml
# from crates/isograph_compiler/Cargo.toml
[package]
name = "isograph_compiler"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
isograph_parser = { path = "../isograph_parser" }
span = { path = "../span" }
thiserror = { workspace = true }

[lints]
workspace = true
```

```rust
// from crates/isograph_compiler/src/lib.rs
mod host_language;

pub use host_language::*;
```

`HostLanguage`, `WithErrors`, and `IsoLiteralError` as in Types. No TypeScript types, no regex. No `NetworkProtocol`, no `GenerateArtifacts`, no `Profile`.

`ParseError` already derives `thiserror::Error` (parse-iso-literal-entry.md). `SelectableNameWrapper` gains `Display` in the parser so `TypeScriptHostError::MissingExport` can format the name.

## Change 2: `isograph_extract_typescript`

New crate. The regex and `TypeScriptHostLanguage` live here. `isograph_parser` and `isograph_compiler` do not depend on this crate (that would cycle). `isograph_lsp` and `isograph_cli` do not depend on this crate. `ts_graphql_react_isograph_cli` does, starting in lsp-semantic-tokens.md.

```toml
# from crates/isograph_extract_typescript/Cargo.toml
[package]
name = "isograph_extract_typescript"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
common_lang_types = { path = "../common_lang_types" }
intern = { path = "../../relay-crates/intern" }
isograph_compiler = { path = "../isograph_compiler" }
isograph_parser = { path = "../isograph_parser" }
prelude = { path = "../prelude" }
regex = { workspace = true }
span = { path = "../span" }
thiserror = { workspace = true }

[lints]
workspace = true
```

Workspace member via `./crates/*`.

The regex is isograph's, with named groups. Origin pattern:

```
(// )?(export const ([^ ]+) =\s+)?iso(\()?\s*`([^`]+)`,?\s*(\))?(\()?
```

```rust
// from crates/isograph_extract_typescript/src/lib.rs
const EXTRACT_ISO_LITERAL_PATTERN: &str =
    r"(?<comment>// )?(export const (?<export_name>[^ ]+) =\s+)?iso(?<open_paren>\()?\s*`(?<literal>[^`]+)`,?\s*(?<close_paren>\))?(?<associated>\()?";

static EXTRACT_ISO_LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(EXTRACT_ISO_LITERAL_PATTERN)
        .expect("EXTRACT_ISO_LITERAL_PATTERN is a valid regex")
});
```

`expect`: the pattern is a string literal in this file. `Regex::new` returns `Err` only for invalid syntax; the type system cannot check regex syntax. The invariant is that this pattern compiles.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
impl HostLanguage for TypeScriptHostLanguage {
    type LiteralContext = TypeScriptLiteralContext;
    type Error = TypeScriptHostError;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<
        WithErrors<
            WithSpan<(&'a str, Self::LiteralContext)>,
            Vec<WithSpan<IsoLiteralError<Self>>>,
        >,
    > {
        EXTRACT_ISO_LITERAL
            .captures_iter(source)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                let span = Span::from_usize(literal.start(), literal.end());
                let iso_literal_text = literal.as_str();
                let context = TypeScriptLiteralContext {
                    const_export_name: captures
                        .name("export_name")
                        .map(|m| m.as_str().intern().to()),
                    call: match captures.name("open_paren") {
                        Some(_) => IsoCall::FunctionCall,
                        None => IsoCall::TaggedTemplate,
                    },
                    associated_js_function: match captures.name("associated") {
                        Some(_) => AssociatedJsFunction::Present,
                        None => AssociatedJsFunction::Absent,
                    },
                };
                let parsed = parse_iso_literal(iso_literal_text);
                let parsed_item = parsed.item.as_ref().and_then(item_of);
                let mut errors = Vec::new();
                if let IsoCall::TaggedTemplate = context.call {
                    errors.push(
                        IsoLiteralError::Host(TypeScriptHostError::MissingParentheses)
                            .with_span(span),
                    );
                }
                if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
                    if context.const_export_name.is_none() {
                        errors.push(
                            IsoLiteralError::Host(TypeScriptHostError::MissingExport {
                                suggested_name: selectable.name.item,
                            })
                            .with_span(span),
                        );
                    }
                    if let AssociatedJsFunction::Absent = context.associated_js_function
                    {
                        errors.push(
                            IsoLiteralError::Host(
                                TypeScriptHostError::MissingAssociatedFunction,
                            )
                            .with_span(span),
                        );
                    }
                }
                for error in parsed.errors {
                    errors.push(
                        IsoLiteralError::Parse(error.item)
                            .with_span(error.location.with_offset(span.start)),
                    );
                }
                WithErrors {
                    item: (iso_literal_text, context).with_span(span),
                    errors,
                }
                .wrap_some()
            })
            .collect()
    }
}

fn item_of(parse: &WithSpan<IsoLiteralParse>) -> Option<&IsoLiteralItem> {
    parse
        .item
        .item
        .item
        .item
        .as_ref()
        .map(|item| item.item.reference())
}
```

`item_of` is a private helper in the TypeScript crate: parentheses are a file fact; export and associated function depend on whether the contents parsed as `Selectable`. The parse tree is not `WithErrors.item`. `item` is the isograph string and host context. Grammar errors come from `parse_iso_literal(text)`.

`TypeScriptHostError` derives `thiserror::Error` in this crate. `SelectableNameWrapper`'s `Display` is Change 1.

`TypeScriptHostLanguage`: empty backticks (`iso(\`\`)`) do not match (`[^`]+` needs at least one character). A missing `literal` group skips the match. `close_paren` is in the pattern so the associated `(` can match; it is not a field.

Origin of `extract_iso_literals`: `extract_iso_literals_from_file_content` plus the host checks in `process_iso_literal_extraction` in isograph's `isograph_literals.rs`. Delta: method on `TypeScriptHostLanguage`; named groups; interned `ConstExportName`; host facts on the tuple's second element; return is `WithErrors` (`item` / `errors`).

### Tests

In `crates/isograph_extract_typescript/src/lib.rs` under `#[cfg(test)]`. Helper `extract` takes `.item` of each `WithErrors`. Host-error tests read `.errors`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use isograph_compiler::{HostLanguage, IsoLiteralError, WithErrors};
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        AssociatedJsFunction, IsoCall, TypeScriptHostError, TypeScriptHostLanguage,
        TypeScriptLiteralContext,
    };

    fn extract(
        source: &str,
    ) -> Vec<WithSpan<(&str, TypeScriptLiteralContext)>> {
        TypeScriptHostLanguage
            .extract_iso_literals(source)
            .into_iter()
            .map(|extracted| extracted.item)
            .collect()
    }

    fn extract_all(
        source: &str,
    ) -> Vec<
        WithErrors<
            WithSpan<(&str, TypeScriptLiteralContext)>,
            Vec<WithSpan<IsoLiteralError<TypeScriptHostLanguage>>>,
        >,
    > {
        TypeScriptHostLanguage.extract_iso_literals(source)
    }

    fn interned_export(name: &str) -> common_lang_types::ConstExportName {
        name.intern().to()
    }

    #[test]
    fn empty_source_extracts_nothing() {
        assert_eq!(extract(""), vec![]);
    }

    #[test]
    fn source_with_no_iso_extracts_nothing() {
        assert_eq!(extract("export const Foo = 1;"), vec![]);
    }

    #[test]
    fn exported_field_with_associated_function() {
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let text = "field Pet.fullName { id }";
        let start = source
            .find(text)
            .expect("the fixture contains the literal text");
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0],
            (
                text,
                TypeScriptLiteralContext {
                    const_export_name: interned_export("fullName").wrap_some(),
                    call: IsoCall::FunctionCall,
                    associated_js_function: AssociatedJsFunction::Present,
                },
            )
            .with_span(Span::from_usize(start, start + text.len()))
        );
        assert_eq!(&source[extracted[0].location.as_usize_range()], text);
    }

    #[test]
    fn unexported_entrypoint() {
        let source = "iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.1.const_export_name, None);
        assert_eq!(extracted[0].item.0, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].item.1.call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].item.1.associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(
            &source[extracted[0].location.as_usize_range()],
            extracted[0].item.0
        );
    }

    #[test]
    fn tagged_template() {
        let source = "iso`entrypoint Query.HomeRoute`";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.1.call, IsoCall::TaggedTemplate);
        assert_eq!(
            extracted[0].item.1.associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(extracted[0].item.0, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn commented_literal_is_skipped() {
        let source = "// export const Foo = iso(`field Pet.fullName { id }`)(";
        assert_eq!(extract(source), vec![]);
    }

    #[test]
    fn commented_literal_does_not_hide_a_later_one() {
        let source = "\
// export const Foo = iso(`field Pet.fullName { id }`)(
export const Bar = iso(`field Pet.bar { id }`)(";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0].item.1.const_export_name,
            interned_export("Bar").wrap_some()
        );
        assert_eq!(extracted[0].item.0, "field Pet.bar { id }");
    }

    #[test]
    fn two_literals_in_one_file() {
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].item.1.const_export_name,
            interned_export("fullName").wrap_some()
        );
        assert_eq!(extracted[1].item.1.const_export_name, None);
        assert_eq!(extracted[1].item.0, "entrypoint Query.HomeRoute");
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
        let extracted = extract(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].item.1.const_export_name,
            interned_export("HomeRoute").wrap_some()
        );
        assert_eq!(
            extracted[0].item.1.associated_js_function,
            AssociatedJsFunction::Present
        );
        assert_eq!(extracted[1].item.1.const_export_name, None);
        assert_eq!(
            extracted[1].item.0,
            "entrypoint Query.PetFavoritePhrase"
        );
        assert_eq!(
            extracted[1].item.1.associated_js_function,
            AssociatedJsFunction::Absent
        );
    }

    #[test]
    fn empty_backticks_are_not_extracted() {
        assert_eq!(extract("iso(``)"), vec![]);
    }

    #[test]
    fn const_without_export_has_no_name() {
        let source = "const Foo = iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.1.const_export_name, None);
    }

    #[test]
    fn extra_space_before_equals_drops_the_export_name() {
        let source = "export const Foo  = iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.1.const_export_name, None);
    }

    #[test]
    fn no_space_after_equals_drops_the_export_name() {
        let source = "export const Foo =iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.1.const_export_name, None);
    }

    #[test]
    fn trailing_comma_after_the_template() {
        let source = "iso(`entrypoint Query.HomeRoute`,)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.0, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].item.1.call, IsoCall::FunctionCall);
    }

    #[test]
    fn function_call_without_close_paren() {
        let source = "iso(`entrypoint Query.HomeRoute`";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.1.call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].item.1.associated_js_function,
            AssociatedJsFunction::Absent
        );
    }

    #[test]
    fn whitespace_between_iso_and_backtick() {
        let source = "iso( `entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.0, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn multiline_literal() {
        let source = "iso(`\nentrypoint Query.HomeRoute\n`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0].item.0,
            "\nentrypoint Query.HomeRoute\n"
        );
        assert_eq!(
            &source[extracted[0].location.as_usize_range()],
            extracted[0].item.0
        );
    }

    #[test]
    fn double_quoted_string_is_not_extracted() {
        assert_eq!(extract("iso(\"entrypoint Query.HomeRoute\")"), vec![]);
    }

    #[test]
    fn double_slash_without_space_does_not_count_as_comment() {
        let source = "//iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.0, "entrypoint Query.HomeRoute");
    }
}
```

`exported_field_with_associated_function` computes the expected span from `source.find` on the fixture text, not from the extraction. `&source[span] == iso_literal_text` is asserted there and on `unexported_entrypoint` and `multiline_literal`.

`TypeScriptHostError` implements `Error` via `thiserror`. Origin of the three messages: isograph `process_iso_literal_extraction` and `expected_literal_to_be_exported_diagnostic`. Delta: typed errors; span is `WithErrors.item.location` (the contents), not `Span::todo_generated`. Parentheses apply to every literal. Export and associated function apply only to `IsoLiteralItem::Selectable`. Entrypoints and a failed parse (`parsed_item: None`) get the parentheses check only. Pipeline `ParseError`s from `parse_iso_literal` go in `errors` as `IsoLiteralError::Parse`, rebased with `with_offset(span.start)`.

`MissingExport` writes `SelectableNameWrapper`. That `Display` lands in Change 1:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl fmt::Display for SelectableNameWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
```

Before: `SelectableNameWrapper` has no `Display`. `parse_iso_literal.rs` gains `use std::fmt;`.

## Change 3: host-error tests

Same `tests` module. `extract_all` is `TypeScriptHostLanguage.extract_iso_literals`. Helper `errors_of` takes `WithErrors.errors`.

```rust
    fn errors_of(
        source: &str,
    ) -> Vec<IsoLiteralError<TypeScriptHostLanguage>> {
        let extracted = extract_all(source);
        assert_eq!(extracted.len(), 1);
        extracted[0]
            .errors
            .iter()
            .map(|error| error.item.clone())
            .collect()
    }

    #[test]
    fn tagged_template_is_missing_parentheses() {
        assert_eq!(
            errors_of("iso`entrypoint Query.HomeRoute`"),
            IsoLiteralError::Host(TypeScriptHostError::MissingParentheses).wrap_vec()
        );
    }

    #[test]
    fn incomplete_entrypoint_is_a_parse_error() {
        let errors = errors_of("iso(`entrypoint`)");
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, IsoLiteralError::Parse(_)))
        );
    }

    #[test]
    fn entrypoint_without_export_is_valid() {
        assert_eq!(errors_of("iso(`entrypoint Query.HomeRoute`)"), vec![]);
    }

    #[test]
    fn field_without_export_is_missing_export() {
        let errors = errors_of("iso(`field Pet.fullName { id }`)(");
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            IsoLiteralError::Host(TypeScriptHostError::MissingExport { .. })
        ));
    }

    #[test]
    fn field_without_associated_function_is_missing_associated_function() {
        assert_eq!(
            errors_of("export const fullName = iso(`field Pet.fullName { id }`)"),
            IsoLiteralError::Host(TypeScriptHostError::MissingAssociatedFunction).wrap_vec()
        );
    }

    #[test]
    fn exported_field_with_associated_function_is_valid() {
        assert_eq!(
            errors_of("export const fullName = iso(`field Pet.fullName { id }`)("),
            vec![]
        );
    }

    #[test]
    fn tagged_template_field_reports_parentheses_and_export_and_associated() {
        let errors = errors_of("iso`field Pet.fullName { id }`");
        assert_eq!(errors.len(), 3);
        assert_eq!(
            errors[0],
            IsoLiteralError::Host(TypeScriptHostError::MissingParentheses)
        );
        assert!(matches!(
            errors[1],
            IsoLiteralError::Host(TypeScriptHostError::MissingExport { .. })
        ));
        assert_eq!(
            errors[2],
            IsoLiteralError::Host(TypeScriptHostError::MissingAssociatedFunction)
        );
    }

    #[test]
    fn host_error_span_is_the_extraction_span() {
        let source = "iso`entrypoint Query.HomeRoute`";
        let extracted = extract_all(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].errors.len(), 1);
        assert_eq!(
            extracted[0].errors[0].location,
            extracted[0].item.location
        );
    }

    #[test]
    fn valid_extraction_has_no_errors() {
        let extracted = extract_all("iso(`entrypoint Query.HomeRoute`)");
        assert_eq!(extracted.len(), 1);
        assert!(extracted[0].errors.is_empty());
    }
```

`errors_of("iso(`field Pet.fullName { id }`)(")`: associated `(` is present, export is not. One error, `MissingExport`. The suggested name is `fullName`.

`Display` tests:

```rust
    #[test]
    fn missing_parentheses_displays() {
        assert_eq!(
            TypeScriptHostError::MissingParentheses.to_string(),
            "You must call the iso function with parentheses. \"iso`...`\" is not supported"
        );
    }
```

## Order

1. Change 1; `isograph_compiler` with `HostLanguage`, `WithErrors`, `IsoLiteralError`. `Display` on `SelectableNameWrapper` in the parser.
2. Change 2; `crates/isograph_extract_typescript`, regex, extract tests. No `typescript` feature. No `isograph_lsp` / `isograph_cli` dependency.
3. Change 3; host-error tests.
