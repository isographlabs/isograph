use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use isograph_compiler::{ExtractedIsoLiterals, HostLanguage, IsoLiteralError, WithErrors};
use isograph_parser::{IsoLiteralItem, IsoLiteralParse, SelectableNameWrapper, parse_iso_literal};
use prelude::Postfix;
use regex::Regex;
use span::{Span, WithSpan, WithSpanPostfix};
use std::sync::LazyLock;
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

    fn extract_iso_literals<'a>(&self, source: &'a str) -> ExtractedIsoLiterals<'a, Self> {
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
                    if let AssociatedJsFunction::Absent = context.associated_js_function {
                        errors.push(
                            IsoLiteralError::Host(TypeScriptHostError::MissingAssociatedFunction)
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
    parse.item.item.as_ref().map(|item| item.item.reference())
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use isograph_compiler::{ExtractedIsoLiterals, HostLanguage, IsoLiteralError};
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        AssociatedJsFunction, IsoCall, TypeScriptHostError, TypeScriptHostLanguage,
        TypeScriptLiteralContext,
    };

    fn extract(source: &str) -> Vec<WithSpan<(&str, TypeScriptLiteralContext)>> {
        TypeScriptHostLanguage
            .extract_iso_literals(source)
            .into_iter()
            .map(|extracted| extracted.item)
            .collect()
    }

    fn extract_all(source: &str) -> ExtractedIsoLiterals<'_, TypeScriptHostLanguage> {
        TypeScriptHostLanguage.extract_iso_literals(source)
    }

    fn interned_export(name: &str) -> common_lang_types::ConstExportName {
        name.intern().to()
    }

    fn errors_of(source: &str) -> Vec<IsoLiteralError<TypeScriptHostLanguage>> {
        let extracted = extract_all(source);
        assert_eq!(extracted.len(), 1);
        extracted[0]
            .errors
            .iter()
            .map(|error| error.item.clone())
            .collect()
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
        assert_eq!(extracted[0].errors[0].location, extracted[0].item.location);
    }

    #[test]
    fn valid_extraction_has_no_errors() {
        let extracted = extract_all("iso(`entrypoint Query.HomeRoute`)");
        assert_eq!(extracted.len(), 1);
        assert!(extracted[0].errors.is_empty());
    }

    #[test]
    fn missing_parentheses_displays() {
        assert_eq!(
            TypeScriptHostError::MissingParentheses.to_string(),
            "You must call the iso function with parentheses. \"iso`...`\" is not supported"
        );
    }
}
