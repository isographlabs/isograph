# Extract iso literals

Iso literals exist in files: an isograph source string occupies a span in a file. Finding that string, and checking that the host-language embedding is valid, is `THostLanguage: HostLanguage`. `extract_iso_literals` returns `Vec<Slot<IsoLiteralExtraction<Self>, Vec<WithSpan<IsoLiteralError<Self>>>>>`. `Slot.item` is `Some` when the host produced the string. `Slot.extra` is `Some` when there were errors, never `Some` of an empty vec. `IsoLiteralError` is `Host`, `Parse`, `Bracket`, `Comma`. How the host finds the string is not common. Host facts live on `THostLanguage::LiteralContext`, not on `SelectableDeclaration`.

The first implementor is `TypeScriptHostLanguage`: the same regex isograph uses on JavaScript and TypeScript source. It finds `iso(\`...\`)` and `iso\`...\`` and puts the interior of the backticks in `item`.

## Types

Most important first. Parser crate: the trait. TypeScript crate: the implementor.

```rust
// from crates/isograph_parser/src/host_language.rs
use std::fmt;

use span::WithSpan;

use crate::{BracketError, CommaWithoutItem, ParseError, Slot};

pub trait HostLanguage: Sized {
    type Error: fmt::Display + std::error::Error;
    type LiteralContext;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<Slot<IsoLiteralExtraction<'a, Self>, Vec<WithSpan<IsoLiteralError<Self>>>>>;
}

pub struct IsoLiteralExtraction<'a, THostLanguage: HostLanguage> {
    pub iso_literal_text: &'a str,
    pub context: THostLanguage::LiteralContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IsoLiteralError<THostLanguage: HostLanguage> {
    Host(THostLanguage::Error),
    Parse(ParseError),
    Bracket(BracketError),
    Comma(CommaWithoutItem),
}

impl<THostLanguage: HostLanguage> fmt::Display for IsoLiteralError<THostLanguage> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IsoLiteralError::Host(error) => write!(f, "{error}"),
            IsoLiteralError::Parse(error) => write!(f, "{error}"),
            IsoLiteralError::Bracket(error) => write!(f, "{error}"),
            IsoLiteralError::Comma(error) => write!(f, "{error}"),
        }
    }
}

impl<THostLanguage: HostLanguage> std::error::Error for IsoLiteralError<THostLanguage> {}
```

`IsoLiteralError` derives `Debug, PartialEq, Eq` (`Clone` when `THostLanguage::Error` is `Clone`). Spans on those errors are the inner `WithSpan` in `Slot.extra`'s `Vec<WithSpan<IsoLiteralError<Self>>>`. The outer `WithSpan` on `extra` is the extraction span.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use isograph_parser::{
    BracketError, CommaWithoutItem, HostLanguage, IsoLiteralError, IsoLiteralExtraction,
    IsoLiteralItem, IsoLiteralParse, ParseError, SelectableNameWrapper, Slot, chunk,
    match_brackets, parse_iso_literal, tokenize,
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

Derives: `TypeScriptHostLanguage` is `Copy, Clone, Debug, Default, PartialEq, Eq`. `IsoLiteralExtraction` is `Copy, Clone, Debug, PartialEq, Eq` when `LiteralContext` is. `TypeScriptLiteralContext`, `IsoCall`, `AssociatedJsFunction`, `TypeScriptHostError` are `Copy, Clone, Debug, PartialEq, Eq`. `isograph_parser` does not depend on `regex` or on the TypeScript crate.

The string's location in the file is `Slot.item`'s `WithSpan`, not a field on `IsoLiteralExtraction`. For `TypeScriptHostLanguage`, that span is the interior of the backticks, and `&source[item.location.as_usize_range()] == item.item.iso_literal_text`. Host errors share that span: `extra` is `WithSpan<Vec<Error>>` at the same location.

`IsoCall::FunctionCall` is `iso(\`...\`)`. `IsoCall::TaggedTemplate` is `iso\`...\``. `AssociatedJsFunction::Present` is the `(` the regex reads after the iso call, the start of the resolver argument.

Origin of `IsoLiteralExtraction`: isograph `crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs`. Delta:

```
struct IsoLiteralExtraction { const_export_name, iso_literal_text, iso_literal_start_index, has_associated_js_function, iso_function_called_with_paren }
-> IsoLiteralExtraction<THostLanguage> { iso_literal_text: &'a str, context: THostLanguage::LiteralContext }
location is Slot.item.location
return is Slot<IsoLiteralExtraction, Vec<WithSpan<IsoLiteralError>>> (item / extra)
IsoLiteralError is Host(THostLanguage::Error) | Parse | Bracket | Comma
TypeScriptLiteralContext holds const_export_name, call, associated_js_function
has_associated_js_function: bool -> AssociatedJsFunction
iso_function_called_with_paren: bool -> IsoCall
```

## Change 1: `Slot.extra_tokens` is `extra`

`extra_tokens` names leftover parse tokens. `Slot` is also the return of `extract_iso_literals`, where the second Option is host errors. The field is `extra`, matching `Singleton.extra_chunks`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra: Option<WithSpan<E>>,
}
```

Before: `extra_tokens`. Every `extra_tokens` in `crates/isograph_parser` (struct field, constructions, tests, `#[parent_from]` on that field) becomes `extra`. `UnparsedChunkItemsParent` and resolve pins stay. `parsing-standards.md` Slot listings become `extra`.

## Change 2: `HostLanguage` in the parser

New module `crates/isograph_parser/src/host_language.rs`. Parser `Cargo.toml` does not gain `regex`.

```rust
// from crates/isograph_parser/src/lib.rs
mod host_language;

pub use host_language::*;
```

`HostLanguage`, `IsoLiteralExtraction`, and `IsoLiteralError` as in Types. No TypeScript types, no regex.

`BracketError` and `CommaWithoutItem` gain `Copy`, `Clone`, `Display`, and `std::error::Error` here (messages as in lsp-parse-diagnostics.md Change 1), so `IsoLiteralError` can wrap them. `SelectableNameWrapper` gains `Display` here so `TypeScriptHostError::MissingExport` can format the name.

## Change 3: `isograph_extract_typescript`

New crate. The regex and `TypeScriptHostLanguage` live here. `isograph_parser` does not depend on this crate (that would cycle). Callers that want TypeScript take the feature `typescript`.

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
isograph_parser = { path = "../isograph_parser" }
prelude = { path = "../prelude" }
regex = { workspace = true }
span = { path = "../span" }
thiserror = { workspace = true }

[lints]
workspace = true
```

Workspace member via `./crates/*`.

`crates/isograph_lsp/Cargo.toml` (and any other binary that extracts from TS/JS files):

```toml
isograph_extract_typescript = { path = "../isograph_extract_typescript", optional = true }

[features]
default = ["typescript"]
typescript = ["dep:isograph_extract_typescript"]
```

```rust
// from crates/isograph_lsp/src/lib.rs
#[cfg(feature = "typescript")]
pub use isograph_extract_typescript::*;
```

Without `--features typescript` (or with `--no-default-features`), the TypeScript regex is not in the graph.

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
    ) -> Vec<Slot<IsoLiteralExtraction<'a, Self>, Vec<WithSpan<IsoLiteralError<Self>>>>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(source)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                let span = Span::from_usize(literal.start(), literal.end());
                let extraction = IsoLiteralExtraction {
                    iso_literal_text: literal.as_str(),
                    context: TypeScriptLiteralContext {
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
                    },
                };
                let (parse, parse_errors, bracket_errors, comma_errors) =
                    parse_tree(extraction.iso_literal_text);
                let parsed_item = parse.as_ref().and_then(item_of);
                let mut errors = Vec::new();
                if let IsoCall::TaggedTemplate = extraction.context.call {
                    errors.push(
                        IsoLiteralError::Host(TypeScriptHostError::MissingParentheses)
                            .with_span(span),
                    );
                }
                if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
                    if extraction.context.const_export_name.is_none() {
                        errors.push(
                            IsoLiteralError::Host(TypeScriptHostError::MissingExport {
                                suggested_name: selectable.name.item,
                            })
                            .with_span(span),
                        );
                    }
                    if let AssociatedJsFunction::Absent =
                        extraction.context.associated_js_function
                    {
                        errors.push(
                            IsoLiteralError::Host(
                                TypeScriptHostError::MissingAssociatedFunction,
                            )
                            .with_span(span),
                        );
                    }
                }
                for error in parse_errors {
                    errors.push(
                        IsoLiteralError::Parse(error.item)
                            .with_span(error.location.with_offset(span.start)),
                    );
                }
                for error in bracket_errors {
                    let location = match &error {
                        BracketError::UnmatchedOpen(open) => open.location,
                        BracketError::UnmatchedClose(close) => close.location,
                    };
                    errors.push(
                        IsoLiteralError::Bracket(error)
                            .with_span(location.with_offset(span.start)),
                    );
                }
                for error in comma_errors {
                    errors.push(
                        IsoLiteralError::Comma(error)
                            .with_span(error.0.with_offset(span.start)),
                    );
                }
                Slot {
                    item: extraction.with_span(span).wrap_some(),
                    extra: if errors.is_empty() {
                        None
                    } else {
                        errors.with_span(span).wrap_some()
                    },
                }
                .wrap_some()
            })
            .collect()
    }
}

fn parse_tree(text: &str) -> (
    Option<WithSpan<IsoLiteralParse>>,
    Vec<WithSpan<ParseError>>,
    Vec<BracketError>,
    Vec<CommaWithoutItem>,
) {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let parse = parse_iso_literal(text, tree, &mut errors, &mut tokens);
    (parse, errors, bracket_errors, comma_errors)
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

`parse_tree` / `item_of` are private helpers in the TypeScript crate: parentheses are a file fact; export and associated function depend on whether the contents parsed as `Selectable`. The parse tree is not `Slot.item`. `item` is the isograph string and host context. Grammar errors stay on `parse_iso_literal`'s error vec when a later caller parses.

`Display` for `TypeScriptHostError` lands with Change 3. Change 4 is `Display` for `SelectableNameWrapper` (used by `MissingExport`) and the host-error tests.

`TypeScriptHostLanguage`: empty backticks (`iso(\`\`)`) do not match (`[^`]+` needs at least one character). A missing `literal` group skips the match. `close_paren` is in the pattern so the associated `(` can match; it is not a field.

Origin of `extract_iso_literals`: `extract_iso_literals_from_file_content` plus the host checks in `process_iso_literal_extraction` in isograph's `isograph_literals.rs`. Delta: method on `TypeScriptHostLanguage`; named groups; interned `ConstExportName`; host facts on `context`; return is `Slot` (`item` / `extra`).

### Tests

In `crates/isograph_extract_typescript/src/lib.rs` under `#[cfg(test)]`. Helper `extract` takes `.item` of each `Slot`. Host-error tests read `.extra`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use isograph_parser::{HostLanguage, IsoLiteralError, IsoLiteralExtraction, Slot};
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        AssociatedJsFunction, IsoCall, TypeScriptHostError, TypeScriptHostLanguage,
        TypeScriptLiteralContext,
    };

    fn extract(
        source: &str,
    ) -> Vec<WithSpan<IsoLiteralExtraction<'_, TypeScriptHostLanguage>>> {
        TypeScriptHostLanguage
            .extract_iso_literals(source)
            .into_iter()
            .filter_map(|slot| slot.item)
            .collect()
    }

    fn extract_all(
        source: &str,
    ) -> Vec<
        Slot<
            IsoLiteralExtraction<'_, TypeScriptHostLanguage>,
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
            IsoLiteralExtraction {
                iso_literal_text: text,
                context: TypeScriptLiteralContext {
                    const_export_name: interned_export("fullName").wrap_some(),
                    call: IsoCall::FunctionCall,
                    associated_js_function: AssociatedJsFunction::Present,
                },
            }
            .with_span(Span::from_usize(start, start + text.len()))
        );
        assert_eq!(&source[extracted[0].location.as_usize_range()], text);
    }

    #[test]
    fn unexported_entrypoint() {
        let source = "iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.context.const_export_name, None);
        assert_eq!(extracted[0].item.iso_literal_text, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].item.context.call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].item.context.associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(
            &source[extracted[0].location.as_usize_range()],
            extracted[0].item.iso_literal_text
        );
    }

    #[test]
    fn tagged_template() {
        let source = "iso`entrypoint Query.HomeRoute`";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.context.call, IsoCall::TaggedTemplate);
        assert_eq!(
            extracted[0].item.context.associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(extracted[0].item.iso_literal_text, "entrypoint Query.HomeRoute");
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
            extracted[0].item.context.const_export_name,
            interned_export("Bar").wrap_some()
        );
        assert_eq!(extracted[0].item.iso_literal_text, "field Pet.bar { id }");
    }

    #[test]
    fn two_literals_in_one_file() {
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].item.context.const_export_name,
            interned_export("fullName").wrap_some()
        );
        assert_eq!(extracted[1].item.context.const_export_name, None);
        assert_eq!(extracted[1].item.iso_literal_text, "entrypoint Query.HomeRoute");
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
            extracted[0].item.context.const_export_name,
            interned_export("HomeRoute").wrap_some()
        );
        assert_eq!(
            extracted[0].item.context.associated_js_function,
            AssociatedJsFunction::Present
        );
        assert_eq!(extracted[1].item.context.const_export_name, None);
        assert_eq!(
            extracted[1].item.iso_literal_text,
            "entrypoint Query.PetFavoritePhrase"
        );
        assert_eq!(
            extracted[1].item.context.associated_js_function,
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
        assert_eq!(extracted[0].item.context.const_export_name, None);
    }

    #[test]
    fn extra_space_before_equals_drops_the_export_name() {
        let source = "export const Foo  = iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.context.const_export_name, None);
    }

    #[test]
    fn no_space_after_equals_drops_the_export_name() {
        let source = "export const Foo =iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.context.const_export_name, None);
    }

    #[test]
    fn trailing_comma_after_the_template() {
        let source = "iso(`entrypoint Query.HomeRoute`,)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.iso_literal_text, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].item.context.call, IsoCall::FunctionCall);
    }

    #[test]
    fn function_call_without_close_paren() {
        let source = "iso(`entrypoint Query.HomeRoute`";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.context.call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].item.context.associated_js_function,
            AssociatedJsFunction::Absent
        );
    }

    #[test]
    fn whitespace_between_iso_and_backtick() {
        let source = "iso( `entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].item.iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn multiline_literal() {
        let source = "iso(`\nentrypoint Query.HomeRoute\n`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0].item.iso_literal_text,
            "\nentrypoint Query.HomeRoute\n"
        );
        assert_eq!(
            &source[extracted[0].location.as_usize_range()],
            extracted[0].item.iso_literal_text
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
        assert_eq!(extracted[0].item.iso_literal_text, "entrypoint Query.HomeRoute");
    }
}
```

`exported_field_with_associated_function` computes the expected span from `source.find` on the fixture text, not from the extraction. `&source[span] == iso_literal_text` is asserted there and on `unexported_entrypoint` and `multiline_literal`.

`TypeScriptHostError` implements `Error` via `thiserror`. Origin of the three messages: isograph `process_iso_literal_extraction` and `expected_literal_to_be_exported_diagnostic`. Delta: typed errors; span is `Slot.item.location` (the contents), not `Span::todo_generated`. Parentheses apply to every literal. Export and associated function apply only to `IsoLiteralItem::Selectable`. Entrypoints and a failed parse (`parsed_item: None`) get the parentheses check only. Parse / bracket / comma errors from `parse_tree` go in `extra` as `IsoLiteralError::Parse` / `Bracket` / `Comma`, rebased with `with_offset(span.start)`.

`MissingExport` writes `SelectableNameWrapper`. That `Display` lands in Change 2:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl fmt::Display for SelectableNameWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
```

Before: `SelectableNameWrapper` has no `Display`. `parse_iso_literal.rs` gains `use std::fmt;`.

## Change 4: host-error tests

Same `tests` module. `extract_all` is `TypeScriptHostLanguage.extract_iso_literals`. Helper `extra_items` takes the inner vec of `Slot.extra`.

```rust
    fn extra_items(
        source: &str,
    ) -> Vec<IsoLiteralError<TypeScriptHostLanguage>> {
        let extracted = extract_all(source);
        assert_eq!(extracted.len(), 1);
        extracted[0]
            .extra
            .as_ref()
            .map(|errors| errors.item.iter().map(|error| error.item.clone()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn tagged_template_is_missing_parentheses() {
        assert_eq!(
            extra_items("iso`entrypoint Query.HomeRoute`"),
            IsoLiteralError::Host(TypeScriptHostError::MissingParentheses).wrap_vec()
        );
    }

    #[test]
    #[test]
    fn incomplete_entrypoint_is_a_parse_error() {
        let errors = extra_items("iso(`entrypoint`)");
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, IsoLiteralError::Parse(_)))
        );
    }

    #[test]
    fn entrypoint_without_export_is_valid() {
        assert_eq!(extra_items("iso(`entrypoint Query.HomeRoute`)"), vec![]);
    }

    #[test]
    fn field_without_export_is_missing_export() {
        let errors = extra_items("iso(`field Pet.fullName { id }`)(");
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            IsoLiteralError::Host(TypeScriptHostError::MissingExport { .. })
        ));
    }

    #[test]
    fn field_without_associated_function_is_missing_associated_function() {
        assert_eq!(
            extra_items("export const fullName = iso(`field Pet.fullName { id }`)"),
            IsoLiteralError::Host(TypeScriptHostError::MissingAssociatedFunction).wrap_vec()
        );
    }

    #[test]
    fn exported_field_with_associated_function_is_valid() {
        assert_eq!(
            extra_items("export const fullName = iso(`field Pet.fullName { id }`)("),
            vec![]
        );
    }

    #[test]
    fn tagged_template_field_reports_parentheses_and_export_and_associated() {
        let errors = extra_items("iso`field Pet.fullName { id }`");
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
        let item = extracted[0]
            .item
            .as_ref()
            .expect("the fixture extracts a literal");
        let extra = extracted[0]
            .extra
            .as_ref()
            .expect("the fixture has host errors");
        assert_eq!(extra.item.len(), 1);
        assert_eq!(extra.item[0].location, item.location);
        assert_eq!(extra.location, item.location);
    }

    #[test]
    fn valid_extraction_has_item_and_no_extra() {
        let extracted = extract_all("iso(`entrypoint Query.HomeRoute`)");
        assert_eq!(extracted.len(), 1);
        assert!(extracted[0].item.is_some());
        assert!(extracted[0].extra.is_none());
    }
```

`host_errors("iso(`field Pet.fullName { id }`)(")`: associated `(` is present, export is not. One error, `MissingExport`. The suggested name is `fullName`.

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

1. Change 1; `Slot.extra_tokens` is `extra`.
2. Change 2; `HostLanguage`, `IsoLiteralExtraction`, `IsoLiteralError` in `isograph_parser`. `Display` + `Error` on `BracketError` and `CommaWithoutItem`. `Display` on `SelectableNameWrapper`.
3. Change 3; `crates/isograph_extract_typescript`, `typescript` feature on callers, regex, extract tests.
4. Change 4; host-error tests.
