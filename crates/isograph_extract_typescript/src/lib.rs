use std::path::PathBuf;
use std::sync::LazyLock;

use common_lang_types::ConstExportName;
use intern::string_key::Intern;
use isograph_compiler::{HostLanguage, IsoLiteralExtraction, IsographState};
use isograph_parser::SelectableNameWrapper;
use pico_macros::memo;
use prelude::Postfix;
use regex::Regex;
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
        assert_eq!(extracted[1].iso_literal_text, "entrypoint Query.PetFavoritePhrase");
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
        assert_eq!(extracted[0].iso_literal_text, "\nentrypoint Query.HomeRoute\n");
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
    use isograph_compiler::{HostLanguage, IsographState};
    use prelude::Postfix;

    use super::{AssociatedJsFunction, IsoCall, TypeScriptHostLanguage, TypeScriptLiteralContext};

    fn intern_file(db: &mut IsographState<TypeScriptHostLanguage>, path: PathBuf, contents: &str) {
        db.insert_disk_file(path, contents.to_owned());
    }

    fn interned_export(name: &str) -> common_lang_types::ConstExportName {
        name.intern().to()
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
}
