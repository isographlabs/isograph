use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use isograph_compiler::HostLanguage;
use isograph_parser::SelectableNameWrapper;
use prelude::Postfix;
use regex::Regex;
use span::{Span, WithSpan, WithSpanPostfix};
use std::sync::LazyLock;
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeScriptHostLanguage;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeScriptLiteralContext {
    pub const_export_name: Option<ConstExportName>,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum TypeScriptHostError {
    #[error("You must call the iso function with parentheses. \"iso`...`\" is not supported")]
    MissingParentheses,
    #[error(
        "This isograph field literal must be exported as a named export, for example as `export const {suggested_name}`"
    )]
    MissingExport {
        suggested_name: SelectableNameWrapper,
    },
    #[error("Isograph literals must be immediately called, and passed a function")]
    MissingAssociatedFunction,
}

const EXTRACT_ISO_LITERAL_PATTERN: &str = r"(?<comment>// )?(export const (?<export_name>[^ ]+) =\s+)?iso(?<open_paren>\()?\s*`(?<literal>[^`]+)`,?\s*(?<close_paren>\))?(?<associated>\()?";

static EXTRACT_ISO_LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(EXTRACT_ISO_LITERAL_PATTERN).expect("EXTRACT_ISO_LITERAL_PATTERN is a valid regex")
});

impl HostLanguage for TypeScriptHostLanguage {
    type LiteralContext = TypeScriptLiteralContext;
    type Error = TypeScriptHostError;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(source)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                let span = Span::from_usize(literal.start(), literal.end());
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
                (literal.as_str(), context).with_span(span).wrap_some()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use isograph_compiler::HostLanguage;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        AssociatedJsFunction, IsoCall, TypeScriptHostError, TypeScriptHostLanguage,
        TypeScriptLiteralContext,
    };

    fn extract(source: &str) -> Vec<WithSpan<(&str, TypeScriptLiteralContext)>> {
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
        assert_eq!(extracted[1].item.0, "entrypoint Query.PetFavoritePhrase");
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
        assert_eq!(extracted[0].item.0, "\nentrypoint Query.HomeRoute\n");
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

    #[test]
    fn missing_parentheses_displays() {
        assert_eq!(
            TypeScriptHostError::MissingParentheses.to_string(),
            "You must call the iso function with parentheses. \"iso`...`\" is not supported"
        );
    }
}
