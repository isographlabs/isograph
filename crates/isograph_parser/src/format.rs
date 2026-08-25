use prelude::Postfix;
use span::WithOptionalSpan;

use crate::IsographSemanticToken;

pub fn format_recorded(text: &str, tokens: &[WithOptionalSpan<IsographSemanticToken>]) -> String {
    let mut out = String::new();
    let mut indent = 0_usize;
    let mut last: Option<IsographSemanticToken> = None;
    // Brace is `{` and `}`. Role cannot mean "last was an opener". After an
    // opener, the next gap is Line at current indent.
    let mut last_list = ListIndent::Neither;
    for token in tokens {
        // Always recorded with location None, even when a source comma exists. No lexeme to slice.
        if token.item == IsographSemanticToken::Comma {
            out.push(',');
            last = IsographSemanticToken::Comma.wrap_some();
            last_list = ListIndent::Neither;
            continue;
        }
        let span = token
            .location
            .expect("only Comma is recorded with location None");
        let lexeme = &text[span.as_usize_range()];
        let role = token.item;
        // Type-list `[` is GraphQLTypeName, Neither.
        let list = list_indent(role, lexeme);
        // Before the gap: a closer uses the opener's indent (`foo: {\n}` not `foo: {\n  }`).
        if let ListIndent::Close = list {
            indent -= 1;
        }
        apply_gap(&mut out, gap(last, last_list, role, lexeme), indent);
        out.push_str(lexeme);
        // After the lexeme: indent what follows. Do not break; the next token's gap does.
        if let ListIndent::Open = list {
            indent += 1;
        }
        last = role.wrap_some();
        last_list = list;
    }
    out
}

enum Gap {
    None,
    Space,
    Line,
}

#[derive(Copy, Clone)]
enum ListIndent {
    Open,
    Close,
    Neither,
}

fn list_indent(role: IsographSemanticToken, lexeme: &str) -> ListIndent {
    match (role, lexeme) {
        (IsographSemanticToken::Brace, "{")
        | (IsographSemanticToken::Parenthesis, "(")
        | (IsographSemanticToken::Bracket, "[") => ListIndent::Open,
        (IsographSemanticToken::Brace, "}")
        | (IsographSemanticToken::Parenthesis, ")")
        | (IsographSemanticToken::Bracket, "]") => ListIndent::Close,
        _ => ListIndent::Neither,
    }
}

fn gap(
    last: Option<IsographSemanticToken>,
    last_list: ListIndent,
    role: IsographSemanticToken,
    lexeme: &str,
) -> Gap {
    match last {
        None => Gap::None,
        Some(IsographSemanticToken::Comma) => Gap::Line,
        Some(_) if matches!(last_list, ListIndent::Open) => Gap::Line,
        Some(prev) => {
            if role == IsographSemanticToken::Description
                || prev == IsographSemanticToken::Description
            {
                Gap::Line
            } else if space_after(prev) || space_before(role, lexeme) {
                Gap::Space
            } else {
                Gap::None
            }
        }
    }
}

fn space_after(role: IsographSemanticToken) -> bool {
    matches!(
        role,
        IsographSemanticToken::Keyword
            | IsographSemanticToken::Colon
            | IsographSemanticToken::Equals
    )
}

fn space_before(role: IsographSemanticToken, lexeme: &str) -> bool {
    matches!(
        role,
        IsographSemanticToken::Keyword | IsographSemanticToken::Equals | IsographSemanticToken::At
    ) || lexeme == "{"
}

fn apply_gap(out: &mut String, gap: Gap, indent: usize) {
    match gap {
        Gap::None => {}
        Gap::Space => out.push(' '),
        Gap::Line => {
            out.push('\n');
            for _ in 0..indent {
                out.push_str("  ");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_recorded;
    use crate::parse_iso_literal;

    fn formatted(text: &str) -> String {
        let parsed = parse_iso_literal(text);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        format_recorded(text, parsed.tokens.as_slice())
    }

    #[test]
    fn one_selection_breaks_and_trailing_comma() {
        assert_eq!(formatted("field Query.x{id}"), "field Query.x {\n  id,\n}");
    }

    #[test]
    fn two_selections_one_per_line_with_commas() {
        assert_eq!(
            formatted("field Query.x { id, name }"),
            "field Query.x {\n  id,\n  name,\n}"
        );
        assert_eq!(
            formatted("field Query.x { id, name, }"),
            "field Query.x {\n  id,\n  name,\n}"
        );
    }

    #[test]
    fn empty_braces_break() {
        assert_eq!(formatted("field Query.x {}"), "field Query.x {\n}");
    }

    #[test]
    fn empty_parens_break() {
        assert_eq!(
            formatted("field Query.x { user() }"),
            "field Query.x {\n  user(\n  ),\n}"
        );
    }

    #[test]
    fn empty_object_breaks() {
        assert_eq!(
            formatted("field Query.x { user(arg: {}) }"),
            "field Query.x {\n  user(\n    arg: {\n    },\n  ),\n}"
        );
    }

    #[test]
    fn empty_value_list_breaks() {
        assert_eq!(
            formatted("field Query.x { foo(x: []) }"),
            "field Query.x {\n  foo(\n    x: [\n    ],\n  ),\n}"
        );
    }

    #[test]
    fn value_list_breaks() {
        assert_eq!(
            formatted("field Query.x { foo(x: [1, 2]) }"),
            "field Query.x {\n  foo(\n    x: [\n      1,\n      2,\n    ],\n  ),\n}"
        );
    }

    #[test]
    fn type_list_stays_glued() {
        assert_eq!(
            formatted("field Query.x($y: [Foo!]) { id }"),
            "field Query.x(\n  $y: [Foo!],\n) {\n  id,\n}"
        );
    }

    #[test]
    fn entrypoint_is_unchanged() {
        assert_eq!(
            formatted("entrypoint Query.HomeRoute"),
            "entrypoint Query.HomeRoute"
        );
    }

    #[test]
    fn to_has_spaces() {
        assert_eq!(
            formatted("field Pet.x to Image { id }"),
            "field Pet.x to Image {\n  id,\n}"
        );
    }

    #[test]
    fn nested_object_and_args_break() {
        assert_eq!(
            formatted("field Query.x { user(arg: { b: true }, c: false) }"),
            "field Query.x {\n  user(\n    arg: {\n      b: true,\n    },\n    c: false,\n  ),\n}"
        );
    }

    #[test]
    fn variable_list_breaks() {
        assert_eq!(
            formatted("field Query.user($id:ID,$x:Int){id}"),
            "field Query.user(\n  $id: ID,\n  $x: Int,\n) {\n  id,\n}"
        );
    }

    #[test]
    fn description_is_on_its_own_line() {
        assert_eq!(
            formatted("field Pet.x\n\"desc\"\n{id}"),
            "field Pet.x\n\"desc\"\n{\n  id,\n}"
        );
    }

    #[test]
    fn at_has_a_space_before() {
        let out = formatted("field Query.x { id @loadable }");
        assert!(out.contains("id @loadable"), "{out}");
    }

    #[test]
    fn dot_stays_glued() {
        assert!(formatted("field Query.x { id }").contains("Query.x"));
    }

    #[test]
    fn failed_parse_is_not_formatted() {
        let parsed = parse_iso_literal("not a declaration");
        assert!(!parsed.errors.is_empty());
    }
}
