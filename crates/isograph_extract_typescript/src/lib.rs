use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use isograph_compiler::{
    HostLanguage, IsoLiteralError, IsoLiteralExtraction, IsographState, parsed_iso_literal,
};
use isograph_parser::{IsoLiteralItem, IsoLiteralParse, ParsedIsoLiteral, SelectableNameWrapper};
use pico_macros::memo;
use prelude::Postfix;
use regex::Regex;
use span::WithSpanPostfix;
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

    #[memo]
    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: PathBuf,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let source_id = match db.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(source_id) => source_id,
            None => db.get_disk_file_map().tracked().0.get(&path).copied()?,
        };
        let contents = db.get(source_id).contents.reference();
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| {
                if captures.name("comment").is_some() {
                    return None;
                }
                let literal = captures.name("literal")?;
                IsoLiteralExtraction {
                    iso_literal_text: literal.as_str().to_owned(),
                    iso_literal_start_index: literal.start(),
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
                }
                .wrap_some()
            })
            .collect::<Vec<_>>()
            .wrap_some()
    }
}

pub fn host_errors_for_extraction(
    extraction: &IsoLiteralExtraction<TypeScriptHostLanguage>,
    parsed: &ParsedIsoLiteral,
) -> Vec<span::WithSpan<TypeScriptHostError>> {
    let span = extraction.span();
    let mut errors = Vec::new();
    if let IsoCall::TaggedTemplate = extraction.context.call {
        errors.push(TypeScriptHostError::MissingParentheses.with_span(span));
    }
    let parsed_item = parsed.item.as_ref().and_then(item_of);
    if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
        if extraction.context.const_export_name.is_none() {
            errors.push(
                TypeScriptHostError::MissingExport {
                    suggested_name: selectable.name.item,
                }
                .with_span(span),
            );
        }
        if let AssociatedJsFunction::Absent = extraction.context.associated_js_function {
            errors.push(TypeScriptHostError::MissingAssociatedFunction.with_span(span));
        }
    }
    errors
}

fn item_of(parse: &span::WithSpan<IsoLiteralParse>) -> Option<&IsoLiteralItem> {
    parse.item.item.as_ref().map(|item| item.item.reference())
}

pub struct FileLiteral<'a> {
    pub extraction: &'a IsoLiteralExtraction<TypeScriptHostLanguage>,
    pub parsed: &'a ParsedIsoLiteral,
    pub errors: Vec<span::WithSpan<IsoLiteralError<TypeScriptHostLanguage>>>,
}

pub fn file_literals<'a>(
    db: &'a IsographState<TypeScriptHostLanguage>,
    path: &Path,
) -> Option<Vec<FileLiteral<'a>>> {
    let extractions = TypeScriptHostLanguage::extract_iso_literals(db, path.to_owned()).as_ref()?;
    extractions
        .iter()
        .map(|extraction| {
            let parsed = parsed_iso_literal(db, extraction.iso_literal_text.clone());
            let errors = host_errors_for_extraction(extraction, parsed)
                .into_iter()
                .map(|error| error.map(IsoLiteralError::Host))
                .chain(parsed.errors.iter().map(|error| {
                    IsoLiteralError::Parse(error.item.clone())
                        .with_span(error.location.with_offset(extraction.span().start))
                }))
                .collect();
            FileLiteral {
                extraction,
                parsed,
                errors,
            }
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use intern::string_key::Intern;
    use isograph_compiler::{HostLanguage, IsoLiteralExtraction, IsographState};
    use prelude::Postfix;

    use super::{
        AssociatedJsFunction, IsoCall, TypeScriptHostError, TypeScriptHostLanguage,
        TypeScriptLiteralContext,
    };

    fn intern_file(db: &mut IsographState<TypeScriptHostLanguage>, path: PathBuf, contents: &str) {
        db.insert_disk_file(path, contents.to_owned());
    }

    fn extract(source: &str) -> Vec<IsoLiteralExtraction<TypeScriptHostLanguage>> {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(&mut db, path.clone(), source);
        TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path")
            .clone()
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
        assert_eq!(extracted[0].iso_literal_text, text);
        assert_eq!(extracted[0].iso_literal_start_index, start);
        assert_eq!(
            extracted[0].context,
            TypeScriptLiteralContext {
                const_export_name: interned_export("fullName").wrap_some(),
                call: IsoCall::FunctionCall,
                associated_js_function: AssociatedJsFunction::Present,
            }
        );
        assert_eq!(
            &source[extracted[0].iso_literal_start_index
                ..extracted[0].iso_literal_start_index + extracted[0].iso_literal_text.len()],
            text
        );
    }

    #[test]
    fn unexported_entrypoint() {
        let source = "iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].context.const_export_name, None);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].context.call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].context.associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(
            &source[extracted[0].iso_literal_start_index
                ..extracted[0].iso_literal_start_index + extracted[0].iso_literal_text.len()],
            extracted[0].iso_literal_text
        );
    }

    #[test]
    fn tagged_template() {
        let source = "iso`entrypoint Query.HomeRoute`";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].context.call, IsoCall::TaggedTemplate);
        assert_eq!(
            extracted[0].context.associated_js_function,
            AssociatedJsFunction::Absent
        );
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
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
            extracted[0].context.const_export_name,
            interned_export("Bar").wrap_some()
        );
        assert_eq!(extracted[0].iso_literal_text, "field Pet.bar { id }");
    }

    #[test]
    fn two_literals_in_one_file() {
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].context.const_export_name,
            interned_export("fullName").wrap_some()
        );
        assert_eq!(extracted[1].context.const_export_name, None);
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
        let extracted = extract(source);
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].context.const_export_name,
            interned_export("HomeRoute").wrap_some()
        );
        assert_eq!(
            extracted[0].context.associated_js_function,
            AssociatedJsFunction::Present
        );
        assert_eq!(extracted[1].context.const_export_name, None);
        assert_eq!(
            extracted[1].iso_literal_text,
            "entrypoint Query.PetFavoritePhrase"
        );
        assert_eq!(
            extracted[1].context.associated_js_function,
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
        assert_eq!(extracted[0].context.const_export_name, None);
    }

    #[test]
    fn extra_space_before_equals_drops_the_export_name() {
        let source = "export const Foo  = iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].context.const_export_name, None);
    }

    #[test]
    fn no_space_after_equals_drops_the_export_name() {
        let source = "export const Foo =iso(`entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].context.const_export_name, None);
    }

    #[test]
    fn trailing_comma_after_the_template() {
        let source = "iso(`entrypoint Query.HomeRoute`,)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
        assert_eq!(extracted[0].context.call, IsoCall::FunctionCall);
    }

    #[test]
    fn function_call_without_close_paren() {
        let source = "iso(`entrypoint Query.HomeRoute`";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].context.call, IsoCall::FunctionCall);
        assert_eq!(
            extracted[0].context.associated_js_function,
            AssociatedJsFunction::Absent
        );
    }

    #[test]
    fn whitespace_between_iso_and_backtick() {
        let source = "iso( `entrypoint Query.HomeRoute`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn multiline_literal() {
        let source = "iso(`\nentrypoint Query.HomeRoute\n`)";
        let extracted = extract(source);
        assert_eq!(extracted.len(), 1);
        assert_eq!(
            extracted[0].iso_literal_text,
            "\nentrypoint Query.HomeRoute\n"
        );
        assert_eq!(
            &source[extracted[0].iso_literal_start_index
                ..extracted[0].iso_literal_start_index + extracted[0].iso_literal_text.len()],
            extracted[0].iso_literal_text
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
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn missing_parentheses_displays() {
        assert_eq!(
            TypeScriptHostError::MissingParentheses.to_string(),
            "You must call the iso function with parentheses. \"iso`...`\" is not supported"
        );
    }
}

#[cfg(test)]
mod memo_tests {
    use std::path::PathBuf;

    use intern::string_key::Intern;
    use isograph_compiler::{
        HostLanguage, IsoLiteralError, IsoLiteralExtraction, IsographState, LineChar, LiteralId,
        iso_literal_extraction, literal_id_at_location, parsed_iso_literal,
    };
    use isograph_parser::{
        AstError, IsoLiteralItem, ParseError, ParsedIsoLiteral, SelectableNameWrapper,
    };
    use prelude::Postfix;
    use span::WithSpanPostfix;

    use super::{
        AssociatedJsFunction, IsoCall, TypeScriptHostError, TypeScriptHostLanguage,
        TypeScriptLiteralContext, file_literals, host_errors_for_extraction,
    };

    fn iso_literal_item(parsed: &ParsedIsoLiteral) -> Option<&IsoLiteralItem> {
        parsed
            .item
            .as_ref()?
            .item
            .item
            .as_ref()
            .map(|item| item.item.reference())
    }

    fn intern_file(db: &mut IsographState<TypeScriptHostLanguage>, path: PathBuf, contents: &str) {
        db.insert_disk_file(path, contents.to_owned());
    }

    fn interned_export(name: &str) -> common_lang_types::ConstExportName {
        name.intern().to()
    }

    fn one_literal(
        contents: &str,
    ) -> (
        IsographState<TypeScriptHostLanguage>,
        PathBuf,
        IsoLiteralExtraction<TypeScriptHostLanguage>,
        ParsedIsoLiteral,
    ) {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(&mut db, path.clone(), contents);
        let extraction_text = TypeScriptHostLanguage::extract_iso_literals(&db, path.clone())
            .as_ref()
            .expect("the test interned this path")
            .first()
            .expect("the fixture has one literal")
            .iso_literal_text
            .clone();
        let character = contents
            .find(extraction_text.as_str())
            .expect("the fixture contains the iso text") as u32;
        let literal_id = literal_id_at_location(&db, path.clone(), LineChar { line: 0, character })
            .as_ref()
            .expect("the interior is inside the literal")
            .clone();
        let extraction = iso_literal_extraction(&db, literal_id)
            .as_ref()
            .expect("the LiteralId indexes this file")
            .clone();
        let parsed = parsed_iso_literal(&db, extraction.iso_literal_text.clone()).clone();
        (db, path, extraction, parsed)
    }

    fn text_at_line_char(
        db: &IsographState<TypeScriptHostLanguage>,
        path: PathBuf,
        line_char: LineChar,
    ) -> Option<String> {
        let literal_id = literal_id_at_location(db, path, line_char)
            .as_ref()?
            .clone();
        iso_literal_extraction(db, literal_id)
            .as_ref()?
            .iso_literal_text
            .clone()
            .wrap_some()
    }

    fn parsed_at_line_char(
        db: &IsographState<TypeScriptHostLanguage>,
        path: PathBuf,
        line_char: LineChar,
    ) -> Option<ParsedIsoLiteral> {
        let text = text_at_line_char(db, path, line_char)?;
        parsed_iso_literal(db, text).clone().wrap_some()
    }

    #[test]
    fn missing_disk_file_is_none() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        assert!(TypeScriptHostLanguage::extract_iso_literals(&db, path).is_none());
    }

    #[test]
    fn missing_then_present_extracts() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let other = PathBuf::from("/tmp/proj/src/b.ts");
        intern_file(&mut db, other, "export const x = 1;");
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        assert!(TypeScriptHostLanguage::extract_iso_literals(&db, path.clone()).is_none());
        intern_file(&mut db, path.clone(), "iso(`entrypoint Query.HomeRoute`)");
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
    }

    #[test]
    fn present_file_with_no_iso_is_empty() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(&mut db, path.clone(), "export const Foo = 1;");
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert!(extracted.is_empty());
    }

    #[test]
    fn one_exported_field() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let source = "export const fullName = iso(`field Pet.fullName { id }`)(";
        let text = "field Pet.fullName { id }";
        let start = source
            .find(text)
            .expect("the fixture contains the literal text");
        intern_file(&mut db, path.clone(), source);
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, text);
        assert_eq!(extracted[0].iso_literal_start_index, start);
        assert_eq!(
            extracted[0].context,
            TypeScriptLiteralContext {
                const_export_name: interned_export("fullName").wrap_some(),
                call: IsoCall::FunctionCall,
                associated_js_function: AssociatedJsFunction::Present,
            }
        );
    }

    #[test]
    fn two_literals() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let source = "\
export const fullName = iso(`field Pet.fullName { id }`)(
iso(`entrypoint Query.HomeRoute`)";
        let first = "field Pet.fullName { id }";
        let second = "entrypoint Query.HomeRoute";
        intern_file(&mut db, path.clone(), source);
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(extracted.len(), 2);
        assert_eq!(
            extracted[0].iso_literal_start_index,
            source
                .find(first)
                .expect("the fixture contains the first literal")
        );
        assert_eq!(
            extracted[1].iso_literal_start_index,
            source
                .find(second)
                .expect("the fixture contains the second literal")
        );
        assert_eq!(extracted[1].iso_literal_text, second);
    }

    #[test]
    fn a_second_present_replaces_literals() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(
            &mut db,
            path.clone(),
            "export const fullName = iso(`field Pet.fullName { id }`)(",
        );
        intern_file(&mut db, path.clone(), "iso(`entrypoint Query.HomeRoute`)");
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path)
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].iso_literal_text, "entrypoint Query.HomeRoute");
        assert!(extracted[0].context.const_export_name.is_none());
    }

    #[test]
    fn absent_then_extract_is_none() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(&mut db, path.clone(), "iso(`entrypoint Query.HomeRoute`)");
        db.remove_disk_file(&path);
        assert!(TypeScriptHostLanguage::extract_iso_literals(&db, path).is_none());
    }

    #[test]
    fn parsed_entrypoint_has_no_errors() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let parsed = parsed_iso_literal(&db, "entrypoint Query.HomeRoute".to_owned());
        assert!(parsed.errors.is_empty());
        assert!(matches!(
            iso_literal_item(parsed),
            Some(IsoLiteralItem::Entrypoint(_))
        ));
    }

    #[test]
    fn incomplete_entrypoint_has_a_parse_error() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let parsed = parsed_iso_literal(&db, "entrypoint".to_owned());
        assert!(parsed.item.is_some());
        assert!(iso_literal_item(parsed).is_none());
        assert!(!parsed.errors.is_empty());
    }

    #[test]
    fn empty_literal_is_empty_literal_error() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let parsed = parsed_iso_literal(&db, String::new());
        assert!(parsed.item.is_none());
        assert!(
            parsed
                .errors
                .iter()
                .any(|error| error.item == ParseError::Ast(AstError::EmptyLiteral))
        );
    }

    #[test]
    fn parsed_iso_literal_of_the_same_text_matches_after_unrelated_file() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let text = "entrypoint Query.HomeRoute".to_owned();
        let first = parsed_iso_literal(&db, text.clone()).clone();
        let second = parsed_iso_literal(&db, text.clone()).clone();
        assert_eq!(first, second);
        intern_file(
            &mut db,
            PathBuf::from("/tmp/proj/src/other.ts"),
            "export const x = 1;",
        );
        let third = parsed_iso_literal(&db, text).clone();
        assert_eq!(first, third);
    }

    #[test]
    fn missing_disk_file_has_no_text_or_tree() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let line_char = LineChar {
            line: 0,
            character: 0,
        };
        assert!(text_at_line_char(&db, path.clone(), line_char).is_none());
        assert!(parsed_at_line_char(&db, path, line_char).is_none());
    }

    #[test]
    fn one_line_entrypoint_parses_at_the_interior() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path.clone(), contents);
        let interior = "entrypoint Query.HomeRoute";
        let start = contents
            .find(interior)
            .expect("the fixture contains the literal text") as u32;
        let interior_line_char = LineChar {
            line: 0,
            character: start,
        };
        let text = text_at_line_char(&db, path.clone(), interior_line_char)
            .expect("the interior is inside the literal");
        assert_eq!(text, interior);
        let parsed = parsed_at_line_char(&db, path.clone(), interior_line_char)
            .expect("the interior is inside the literal");
        assert!(parsed.errors.is_empty());
        assert!(matches!(
            iso_literal_item(&parsed),
            Some(IsoLiteralItem::Entrypoint(_))
        ));
        let outside = LineChar {
            line: 0,
            character: 0,
        };
        assert!(text_at_line_char(&db, path.clone(), outside).is_none());
        assert!(parsed_at_line_char(&db, path.clone(), outside).is_none());
        let last_byte = LineChar {
            line: 0,
            character: start + interior.len() as u32 - 1,
        };
        assert!(text_at_line_char(&db, path.clone(), last_byte).is_some());
        assert!(parsed_at_line_char(&db, path.clone(), last_byte).is_some());
        let one_past = LineChar {
            line: 0,
            character: start + interior.len() as u32,
        };
        assert!(text_at_line_char(&db, path.clone(), one_past).is_none());
        assert!(parsed_at_line_char(&db, path, one_past).is_none());
    }

    #[test]
    fn incomplete_entrypoint_at_location_has_parse_errors() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`entrypoint`)";
        intern_file(&mut db, path.clone(), contents);
        let character = contents
            .find("entrypoint")
            .expect("the fixture contains the literal text") as u32;
        let parsed = parsed_at_line_char(&db, path, LineChar { line: 0, character })
            .expect("the interior is inside the literal");
        assert!(!parsed.errors.is_empty());
    }

    #[test]
    fn multiline_literal_parses_on_the_interior_line() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(
            &mut db,
            path.clone(),
            "iso(`\nentrypoint Query.HomeRoute\n`)",
        );
        let interior = LineChar {
            line: 1,
            character: 0,
        };
        assert!(text_at_line_char(&db, path.clone(), interior).is_some());
        assert!(parsed_at_line_char(&db, path.clone(), interior).is_some());
        let iso = LineChar {
            line: 0,
            character: 0,
        };
        assert!(text_at_line_char(&db, path.clone(), iso).is_none());
        assert!(parsed_at_line_char(&db, path.clone(), iso).is_none());
        let backtick = LineChar {
            line: 2,
            character: 0,
        };
        assert!(text_at_line_char(&db, path.clone(), backtick).is_none());
        assert!(parsed_at_line_char(&db, path, backtick).is_none());
    }

    #[test]
    fn two_literals_select_by_character() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`field Pet.fullName { id }`) iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path.clone(), contents);
        let second = "entrypoint Query.HomeRoute";
        let parsed = parsed_at_line_char(
            &db,
            path.clone(),
            LineChar {
                line: 0,
                character: contents
                    .find(second)
                    .expect("the fixture contains the second literal")
                    as u32,
            },
        )
        .expect("the second interior is inside the second literal");
        assert!(matches!(
            iso_literal_item(&parsed),
            Some(IsoLiteralItem::Entrypoint(_))
        ));
        let between = LineChar {
            line: 0,
            character: contents
                .find(") iso")
                .expect("the fixture has JS between the literals") as u32,
        };
        assert!(text_at_line_char(&db, path.clone(), between).is_none());
        assert!(parsed_at_line_char(&db, path, between).is_none());
    }

    #[test]
    fn same_text_in_two_files_matches_parsed_iso_literal() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let a = PathBuf::from("/tmp/proj/src/a.ts");
        let b = PathBuf::from("/tmp/proj/src/b.ts");
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, a.clone(), contents);
        intern_file(&mut db, b.clone(), contents);
        let interior = "entrypoint Query.HomeRoute";
        let character = contents
            .find(interior)
            .expect("the fixture contains the literal text") as u32;
        let line_char = LineChar { line: 0, character };
        let expected = parsed_iso_literal(&db, interior.to_owned());
        assert_eq!(
            parsed_at_line_char(&db, a, line_char).expect("file a interior"),
            *expected
        );
        assert_eq!(
            parsed_at_line_char(&db, b, line_char).expect("file b interior"),
            *expected
        );
    }

    #[test]
    fn prefix_with_newline_is_a_new_line_char() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path.clone(), contents);
        let interior = "entrypoint Query.HomeRoute";
        let character = contents
            .find(interior)
            .expect("the fixture contains the literal text") as u32;
        let old = LineChar { line: 0, character };
        let before =
            text_at_line_char(&db, path.clone(), old).expect("the interior is inside the literal");
        let prefix = "const x = 1;\n";
        intern_file(&mut db, path.clone(), &(prefix.to_owned() + contents));
        let new = LineChar { line: 1, character };
        let after = text_at_line_char(&db, path.clone(), new)
            .expect("the new interior is inside the literal");
        assert_eq!(before, after);
        assert_eq!(
            parsed_iso_literal(&db, after.clone()),
            parsed_iso_literal(&db, before)
        );
        assert!(text_at_line_char(&db, path, old).is_none());
    }

    #[test]
    fn lengthening_an_earlier_line_keeps_the_line_char() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(
            &mut db,
            path.clone(),
            "const x = 1;\niso(`entrypoint Query.HomeRoute`)",
        );
        let line_char = LineChar {
            line: 1,
            character: 5,
        };
        let before = text_at_line_char(&db, path.clone(), line_char)
            .expect("line 1 character 5 is e of entrypoint");
        intern_file(
            &mut db,
            path.clone(),
            "const x = 1; const y = 2;\niso(`entrypoint Query.HomeRoute`)",
        );
        let after = text_at_line_char(&db, path, line_char)
            .expect("the LineChar is still inside the literal");
        assert_eq!(before, after);
        assert_eq!(
            parsed_iso_literal(&db, after),
            parsed_iso_literal(&db, before)
        );
    }

    #[test]
    fn missing_disk_file_has_no_literal_id() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let line_char = LineChar {
            line: 0,
            character: 0,
        };
        assert!(literal_id_at_location(&db, path.clone(), line_char).is_none());
        assert!(iso_literal_extraction(&db, LiteralId { path, index: 0 }).is_none());
    }

    #[test]
    fn one_line_entrypoint_is_literal_id_zero() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path.clone(), contents);
        let interior = "entrypoint Query.HomeRoute";
        let start = contents
            .find(interior)
            .expect("the fixture contains the literal text") as u32;
        let expected = LiteralId {
            path: path.clone(),
            index: 0,
        };
        assert_eq!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 0,
                    character: start,
                }
            )
            .as_ref()
            .expect("the interior is inside the literal"),
            &expected
        );
        let extracted = TypeScriptHostLanguage::extract_iso_literals(&db, path.clone())
            .as_ref()
            .expect("the test interned this path");
        assert_eq!(
            iso_literal_extraction(&db, expected.clone())
                .as_ref()
                .expect("index 0 is the one literal"),
            &extracted[0]
        );
        assert!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 0,
                    character: 0,
                }
            )
            .is_none()
        );
        assert_eq!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 0,
                    character: start + interior.len() as u32 - 1,
                }
            )
            .as_ref()
            .expect("the last interior byte is inside the literal"),
            &expected
        );
        assert!(
            literal_id_at_location(
                &db,
                path,
                LineChar {
                    line: 0,
                    character: start + interior.len() as u32,
                }
            )
            .is_none()
        );
    }

    #[test]
    fn two_literals_are_index_zero_and_one() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`field Pet.fullName { id }`) iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path.clone(), contents);
        let second = "entrypoint Query.HomeRoute";
        assert_eq!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 0,
                    character: contents
                        .find(second)
                        .expect("the fixture contains the second literal")
                        as u32,
                }
            )
            .as_ref()
            .expect("the second interior is inside the second literal"),
            &LiteralId {
                path: path.clone(),
                index: 1,
            }
        );
        assert!(
            literal_id_at_location(
                &db,
                path,
                LineChar {
                    line: 0,
                    character: contents
                        .find(") iso")
                        .expect("the fixture has JS between the literals")
                        as u32,
                }
            )
            .is_none()
        );
    }

    #[test]
    fn multiline_literal_id_is_on_the_interior_line() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(
            &mut db,
            path.clone(),
            "iso(`\nentrypoint Query.HomeRoute\n`)",
        );
        let expected = LiteralId {
            path: path.clone(),
            index: 0,
        };
        assert_eq!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 1,
                    character: 0,
                }
            )
            .as_ref()
            .expect("line 1 is the interior"),
            &expected
        );
        assert!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 0,
                    character: 0,
                }
            )
            .is_none()
        );
        assert!(
            literal_id_at_location(
                &db,
                path,
                LineChar {
                    line: 2,
                    character: 0,
                }
            )
            .is_none()
        );
    }

    #[test]
    fn out_of_range_literal_id_is_none() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(&mut db, path.clone(), "iso(`entrypoint Query.HomeRoute`)");
        assert!(iso_literal_extraction(&db, LiteralId { path, index: 1 }).is_none());
    }

    #[test]
    fn prefix_with_newline_literal_id_is_index_zero_at_the_new_line_char() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        intern_file(&mut db, path.clone(), contents);
        let character = contents
            .find("entrypoint")
            .expect("the fixture contains the literal text") as u32;
        let old = LineChar { line: 0, character };
        let prefix = "const x = 1;\n";
        intern_file(&mut db, path.clone(), &(prefix.to_owned() + contents));
        let new = LineChar { line: 1, character };
        assert_eq!(
            literal_id_at_location(&db, path.clone(), new)
                .as_ref()
                .expect("the new interior is inside the literal"),
            &LiteralId {
                path: path.clone(),
                index: 0,
            }
        );
        assert!(literal_id_at_location(&db, path, old).is_none());
    }

    #[test]
    fn lengthening_an_earlier_line_keeps_literal_id_zero() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(
            &mut db,
            path.clone(),
            "const x = 1;\niso(`entrypoint Query.HomeRoute`)",
        );
        let line_char = LineChar {
            line: 1,
            character: 5,
        };
        intern_file(
            &mut db,
            path.clone(),
            "const x = 1; const y = 2;\niso(`entrypoint Query.HomeRoute`)",
        );
        assert_eq!(
            literal_id_at_location(&db, path.clone(), line_char)
                .as_ref()
                .expect("the LineChar is still inside the literal"),
            &LiteralId { path, index: 0 }
        );
    }

    #[test]
    fn tagged_template_is_missing_parentheses() {
        let (_db, _path, extraction, parsed) = one_literal("iso`entrypoint Query.HomeRoute`");
        assert_eq!(
            host_errors_for_extraction(&extraction, &parsed),
            TypeScriptHostError::MissingParentheses
                .with_span(extraction.span())
                .wrap_vec()
        );
    }

    #[test]
    fn incomplete_entrypoint_is_a_parse_error() {
        let (db, path, extraction, parsed) = one_literal("iso(`entrypoint`)");
        assert!(!parsed.errors.is_empty());
        assert!(host_errors_for_extraction(&extraction, &parsed).is_empty());
        let literals = file_literals(&db, &path).expect("the test interned this path");
        assert!(
            literals
                .iter()
                .flat_map(|literal| literal.errors.iter())
                .any(|error| matches!(error.item, IsoLiteralError::Parse(_)))
        );
    }

    #[test]
    fn entrypoint_without_export_is_valid() {
        let (_db, _path, extraction, parsed) = one_literal("iso(`entrypoint Query.HomeRoute`)");
        assert!(host_errors_for_extraction(&extraction, &parsed).is_empty());
    }

    #[test]
    fn field_without_export_is_missing_export() {
        let (_db, _path, extraction, parsed) = one_literal("iso(`field Pet.fullName { id }`)(");
        let errors = host_errors_for_extraction(&extraction, &parsed);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].item,
            TypeScriptHostError::MissingExport { .. }
        ));
    }

    #[test]
    fn field_without_associated_function_is_missing_associated_function() {
        let (_db, _path, extraction, parsed) =
            one_literal("export const fullName = iso(`field Pet.fullName { id }`)");
        assert_eq!(
            host_errors_for_extraction(&extraction, &parsed),
            TypeScriptHostError::MissingAssociatedFunction
                .with_span(extraction.span())
                .wrap_vec()
        );
    }

    #[test]
    fn exported_field_with_associated_function_is_valid() {
        let (_db, _path, extraction, parsed) =
            one_literal("export const fullName = iso(`field Pet.fullName { id }`)(");
        assert!(host_errors_for_extraction(&extraction, &parsed).is_empty());
    }

    #[test]
    fn tagged_template_field_reports_parentheses_and_export_and_associated() {
        let (_db, _path, extraction, parsed) = one_literal("iso`field Pet.fullName { id }`");
        let span = extraction.span();
        assert_eq!(
            host_errors_for_extraction(&extraction, &parsed),
            vec![
                TypeScriptHostError::MissingParentheses.with_span(span),
                TypeScriptHostError::MissingExport {
                    suggested_name: SelectableNameWrapper("fullName".intern().to()),
                }
                .with_span(span),
                TypeScriptHostError::MissingAssociatedFunction.with_span(span),
            ]
        );
    }

    #[test]
    fn host_error_span_is_the_extraction_span() {
        let (_db, _path, extraction, parsed) = one_literal("iso`entrypoint Query.HomeRoute`");
        let errors = host_errors_for_extraction(&extraction, &parsed);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].location, extraction.span());
    }

    #[test]
    fn valid_extraction_has_no_errors() {
        let (_db, _path, extraction, parsed) = one_literal("iso(`entrypoint Query.HomeRoute`)");
        assert!(parsed.errors.is_empty());
        assert!(host_errors_for_extraction(&extraction, &parsed).is_empty());
    }

    #[test]
    fn missing_disk_file_has_no_file_literals() {
        let db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        assert!(
            literal_id_at_location(
                &db,
                path.clone(),
                LineChar {
                    line: 0,
                    character: 0,
                }
            )
            .is_none()
        );
        assert!(file_literals(&db, &path).is_none());
    }

    #[test]
    fn present_file_with_no_iso_has_empty_file_literals() {
        let mut db = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        intern_file(&mut db, path.clone(), "export const Foo = 1;");
        let literals = file_literals(&db, &path).expect("the test interned this path");
        assert!(literals.is_empty());
    }

    #[test]
    fn file_literals_parse_error_is_file_absolute() {
        let (db, path, extraction, _parsed) = one_literal("iso(`entrypoint`)");
        let literals = file_literals(&db, &path).expect("the test interned this path");
        assert_eq!(literals.len(), 1);
        assert!(literals[0].errors.iter().any(|error| matches!(
            error.item,
            IsoLiteralError::Parse(_)
        ) && error.location.start
            >= extraction.span().start));
        assert!(
            literals[0]
                .errors
                .iter()
                .all(|error| !matches!(error.item, IsoLiteralError::Host(_)))
        );
    }

    #[test]
    fn failed_field_parse_is_not_missing_export() {
        let (_db, _path, extraction, parsed) = one_literal("iso(`field`)(");
        assert!(!parsed.errors.is_empty());
        assert!(host_errors_for_extraction(&extraction, &parsed).is_empty());
    }

    #[test]
    fn tagged_template_incomplete_entrypoint_is_parentheses_only() {
        let (db, path, extraction, parsed) = one_literal("iso`entrypoint`");
        assert_eq!(
            host_errors_for_extraction(&extraction, &parsed),
            TypeScriptHostError::MissingParentheses
                .with_span(extraction.span())
                .wrap_vec()
        );
        let literals = file_literals(&db, &path).expect("the test interned this path");
        assert!(
            literals
                .iter()
                .flat_map(|literal| literal.errors.iter())
                .any(|error| matches!(error.item, IsoLiteralError::Host(_)))
        );
        assert!(
            literals
                .iter()
                .flat_map(|literal| literal.errors.iter())
                .any(|error| matches!(error.item, IsoLiteralError::Parse(_)))
        );
    }
}
