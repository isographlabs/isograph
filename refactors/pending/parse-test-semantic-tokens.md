# Parse tests assert semantic token sequences

Every parse test asserts the semantic token sequence `parse_iso_literal` (or the sub-parser the test calls) recorded, in addition to the tree and error facts it already asserts. Tests write the sequence as roles and source slices in consume order. The helper turns each slice into a span (next occurrence at or after the previous token's end) and `assert_eq`s the `WithSpan<SemanticToken>` values, so a wrong span fails even when the slice text matches.

Parse tests are tests whose subject is a grammar parse: `parse_iso_literal`, `parse_selection`, `parse_argument` / `parse_non_constant_value` / `consume_argument_list`, `parse_each_chunk` with a parse function, and `consume_description`. tokenize, match_brackets, chunk-structure, parse_error Display, and cursor-mechanics tests in `chunk_stream.rs` that do not parse a form are not parse tests.

One shippable change.

## Helper

```rust
// from crates/isograph_parser/src/assert_semantic_tokens.rs
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::SemanticToken;

pub(crate) fn assert_semantic_tokens(
    text: &str,
    actual: &[WithSpan<SemanticToken>],
    expected: &[(SemanticToken, &str)],
) {
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let offset = text[search_from..].find(pattern).expect(
            "the expected lexeme occurs in the fixture after the previous token",
        );
        let start = search_from + offset;
        let end = start + pattern.len();
        expected_tokens.push(role.with_span(Span::from_usize(start, end)));
        search_from = end;
    }
    assert_eq!(
        actual,
        expected_tokens.as_slice(),
        "for literal {text:?}, actual {:?}, expected {:?}",
        displayed(text, actual),
        displayed(text, &expected_tokens),
    );
}

fn displayed(
    text: &str,
    tokens: &[WithSpan<SemanticToken>],
) -> Vec<(SemanticToken, String)> {
    tokens
        .iter()
        .map(|token| {
            (
                token.item,
                text[token.location.as_usize_range()].to_owned(),
            )
        })
        .collect()
}
```

Sequential search: each lexeme is found at or after the previous token's end. Duplicate lexemes (`to to`, two `id`s, two `{`) resolve in source order. `assert_eq` of the `WithSpan` values checks role and span. `displayed` is only the panic message, so a failure reads as `(Keyword, "entrypoint")` rather than span offsets. A recorded token whose slice is right and whose span points at a later occurrence of that slice fails. The helper does not skip a recorded token and does not invent a span for a lexeme the grammar did not commit.

```rust
// from crates/isograph_parser/src/lib.rs
#[cfg(test)]
mod assert_semantic_tokens;
```

## Helpers take the expected sequence

A parse helper asserts tokens before it returns. Callers cannot forget.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<AstError>>) {
        let parsed = parse_iso_literal(text);
        let errors = parsed
            .errors
            .into_iter()
            .map(|error| match error.item {
                ParseError::Ast(ast) => ast.with_span(error.location),
                ParseError::Bracket(_) | ParseError::Comma(_) => {
                    panic!("for literal {text:?}")
                }
            })
            .collect();
        (
            parsed.item.expect("the fixture is not an empty literal"),
            errors,
        )
    }

    fn parsed_with_errors(text: &str) -> ParsedIsoLiteral {
        parse_iso_literal(text)
    }

    fn parsed_with_tokens(text: &str) -> ParsedIsoLiteral {
        parse_iso_literal(text)
    }

    fn assert_no_declaration(text: &str, reason: AstError, reason_span: Span) {
        let (parse, errors) = parsed(text);
        // ...
    }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    use crate::assert_semantic_tokens::assert_semantic_tokens;

    fn parsed(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<AstError>>) {
        let parsed = parse_iso_literal(text);
        assert_semantic_tokens(text, &parsed.tokens, expected_tokens);
        let errors = parsed
            .errors
            .into_iter()
            .map(|error| match error.item {
                ParseError::Ast(ast) => ast.with_span(error.location),
                ParseError::Bracket(_) | ParseError::Comma(_) => {
                    panic!("for literal {text:?}")
                }
            })
            .collect();
        (
            parsed.item.expect("the fixture is not an empty literal"),
            errors,
        )
    }

    fn parsed_with_errors(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> ParsedIsoLiteral {
        let parsed = parse_iso_literal(text);
        assert_semantic_tokens(text, &parsed.tokens, expected_tokens);
        parsed
    }

    fn assert_no_declaration(
        text: &str,
        reason: AstError,
        reason_span: Span,
        expected_tokens: &[(SemanticToken, &str)],
    ) {
        let (parse, errors) = parsed(text, expected_tokens);
        assert!(
            parsed_item(parse.reference()).is_none(),
            "for literal {text:?}",
        );
        assert!(
            errors
                .iter()
                .any(|error| error.item == reason && error.location == reason_span),
            "for literal {text:?}, errors were {errors:?}",
        );
    }
```

`parsed_with_tokens` is deleted. Its call sites are the AST tests for those fixtures.

```rust
// from crates/isograph_parser/src/selections.rs
    fn parsed_selections(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> ParsedSelections {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
            expected_tokens,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors)
    }

    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> ParsedItems<P> {
        // same setup as today through parse_each_chunk
        assert_semantic_tokens(text, &tokens, expected_tokens);
        (items, errors, comma_errors)
    }
```

`ParsedSelections` and `ParsedItems` drop the tokens field. The helper already compared it.

```rust
// from crates/isograph_parser/src/arguments.rs
    fn parsed_pairs(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> ParsedPairs { /* parsed_items(..., parse_argument, expected_tokens); drop tokens from the tuple */ }

    fn parsed_argument_list(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> ParsedArgumentList { /* consume_argument_list; assert_semantic_tokens; drop tokens from the tuple */ }

    fn parsed_items<P>(/* same extra expected_tokens parameter as selections.rs */)
```

```rust
// from crates/isograph_parser/src/chunk.rs
    fn parsed_each(
        text: &str,
        expected_tokens: &[(SemanticToken, &str)],
    ) -> ParsedEach {
        // same setup as today
        assert_semantic_tokens(text, &tokens, expected_tokens);
        (items, errors, comma_errors)
    }
```

`ParsedEach` drops the tokens field.

Tests that call `stream_of` and then a consume function call `assert_semantic_tokens` on the local `tokens` vec after the consume.

## One test, before and after

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn an_entrypoint_declaration_parses_with_tight_spans() {
        let text = "entrypoint Query.foo";
        let (parse, errors) = parsed(text);
        let declaration = as_entrypoint(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.name.item,
            SelectableNameWrapper("foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.name.location, span_of(text, "foo"));
        assert_eq!(errors, vec![]);
        assert_eq!(parse.location, Span::from_usize(0, text.len()));
    }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn an_entrypoint_declaration_parses_with_tight_spans() {
        let text = "entrypoint Query.foo";
        let (parse, errors) = parsed(
            text,
            &[
                (SemanticToken::Keyword, "entrypoint"),
                (SemanticToken::Type, "Query"),
                (SemanticToken::Period, "."),
                (SemanticToken::FieldName, "foo"),
            ],
        );
        let declaration = as_entrypoint(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.name.item,
            SelectableNameWrapper("foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.name.location, span_of(text, "foo"));
        assert_eq!(errors, vec![]);
        assert_eq!(parse.location, Span::from_usize(0, text.len()));
    }
```

Tree and error asserts stay. The second argument is the sequence.

## Deleted tests

These fixtures are already parse tests. Their token-only copies go away:

- `an_entrypoint_records_keyword_type_period_field_name` — `an_entrypoint_declaration_parses_with_tight_spans`
- `a_failed_prefix_keeps_the_tokens_it_committed` — `a_failed_form_keeps_the_whole_chunk_as_remaining`
- `an_unknown_keyword_records_keyword_at_that_identifier` — `an_unknown_keyword_is_an_error_at_the_keyword`
- `leftover_after_an_entrypoint_is_not_recorded` — `tokens_after_a_complete_entrypoint_are_leftover`
- `a_field_records_keyword_type_to_and_selections` — `a_named_target_with_bang_is_not_null_wrapped`
- `a_pair_records_argument_colon_and_value_tokens` — `pairs_parse_as_name_colon_value`

## Sequences

Each entry is `test_name`, then one line per fixture: the source, then the `expected_tokens` slice. Shared loops that share a sequence list it once. `!` is never a token. Separator commas and unread leftover are never a token. Type-list `[` `]` are `GraphQLTypeName`. Value-list `[` `]` are `Bracket`. Open and close of one group are two entries with the same role.

### `parse_iso_literal.rs`

`an_entrypoint_declaration_parses_with_tight_spans`

```
"entrypoint Query.foo"
(Keyword, "entrypoint"), (Type, "Query"), (Period, "."), (FieldName, "foo")
```

`surrounding_line_breaks_and_interior_spaces_are_insignificant` — same sequence for `"\n  entrypoint Query.foo\n"`, `"\n\nentrypoint Query.foo"`, `"entrypoint Query . foo"`.

`empty_literal_is_none_with_empty_literal_error` / `empty_and_whitespace_only_literals_are_empty_literal_errors` — `""`, `"   "`, `"\n\n"`: `&[]`.

`comma_mistakes_are_chunkings_errors_and_the_declaration_still_parses` — `",entrypoint Query.foo"` and `",,entrypoint Query.foo"`: entrypoint sequence.

`a_lone_comma_is_chunkings_error_and_an_empty_literal` — `","`: `&[]`.

`the_cut_removes_an_unmatched_bracket_and_the_declaration_parses` / `a_stray_close_is_a_parse_error_and_the_declaration_parses` — `"entrypoint Query.foo)"` and `"entrypoint Query.foo ("`: entrypoint sequence.

`a_final_comma_after_the_declaration_is_an_error` — `"entrypoint Query.foo,"` and `"\nentrypoint Query.foo,\n"`: entrypoint sequence.

`a_comma_before_a_second_declaration_is_the_boundary_comma` — `"entrypoint Query.foo, field User.name"`: entrypoint sequence.

`a_second_contentful_chunk_is_multiple_declarations` — `"entrypoint Query.foo\nfield User.name"`: entrypoint sequence.

`a_failed_first_chunk_is_reported_even_when_a_second_exists`

```
"entrypoint\nQuery.foo"
(Keyword, "entrypoint")
```

`an_unknown_keyword_is_an_error_at_the_keyword`

```
"fieldd Query.foo { bar }"
(Keyword, "fieldd")
```

`a_literal_opening_with_a_group_expects_a_keyword` — `"{ bar }"`: `&[]`.

`a_pointer_keyword_is_not_a_declaration`

```
"pointer Pet.BestFriend to Owner { id }"
(Keyword, "pointer")
```

`each_missing_entrypoint_part_reports_at_its_position`

```
"entrypoint"
(Keyword, "entrypoint")

"entrypoint 42.foo"
(Keyword, "entrypoint")

"entrypoint Query foo"
(Keyword, "entrypoint"), (Type, "Query")

"entrypoint Query."
(Keyword, "entrypoint"), (Type, "Query"), (Period, ".")
```

`a_failed_form_keeps_the_whole_chunk_as_remaining`

```
"entrypoint Foo.$ asdf"
(Keyword, "entrypoint"), (Type, "Foo"), (Period, ".")
```

`tokens_after_a_complete_entrypoint_are_leftover` / `a_selection_set_on_an_entrypoint_is_leftover` / leftover-resolve tests on `"entrypoint Query.foo bar"` / `"entrypoint Query.foo { bar }"`: entrypoint sequence.

`an_entrypoint_directive_parses`

```
"entrypoint Query.foo @lazyLoad"
(Keyword, "entrypoint"), (Type, "Query"), (Period, "."), (FieldName, "foo"), (DirectiveName, "@"), (DirectiveName, "lazyLoad")
```

`a_field_directive_sits_between_variables_and_the_description`

```
"field Query.Foo($id: ID) @component \"the route\" { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "id"), (Colon, ":"), (GraphQLTypeName, "ID"), (Parenthesis, ")"), (DirectiveName, "@"), (DirectiveName, "component"), (String, "\"the route\""), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_field_directive_sits_between_the_target_and_the_description`

```
"field Pet.BestFriend to Owner @updatable \"x\" { id }"
(Keyword, "field"), (Type, "Pet"), (Period, "."), (FieldName, "BestFriend"), (Keyword, "to"), (GraphQLTypeName, "Owner"), (DirectiveName, "@"), (DirectiveName, "updatable"), (String, "\"x\""), (Brace, "{"), (FieldName, "id"), (Brace, "}")
```

`a_selection_directive_with_arguments_parses`

```
"field Query.Foo { bar @loadable(lazyLoadArtifact: true) }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (DirectiveName, "@"), (DirectiveName, "loadable"), (Parenthesis, "("), (Argument, "lazyLoadArtifact"), (Colon, ":"), (BooleanOrNull, "true"), (Parenthesis, ")"), (Brace, "}")
```

`two_directives_on_one_selection_stay_in_one_list`

```
"field Query.Foo { bar @loadable @updatable }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (DirectiveName, "@"), (DirectiveName, "loadable"), (DirectiveName, "@"), (DirectiveName, "updatable"), (Brace, "}")
```

`a_directive_on_the_next_line_is_its_own_failed_selection`

```
"field Query.Foo { bar\n@loadable }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`an_unknown_directive_name_parses`

```
"entrypoint Query.foo @notARealDirective"
(Keyword, "entrypoint"), (Type, "Query"), (Period, "."), (FieldName, "foo"), (DirectiveName, "@"), (DirectiveName, "notARealDirective")
```

`at_without_a_name_fails_the_host`

```
"entrypoint Query.foo @"
(Keyword, "entrypoint"), (Type, "Query"), (Period, "."), (FieldName, "foo"), (DirectiveName, "@")
```

`directive_names_resolve_through_the_host`

```
"field Query.Foo { bar @loadable }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (DirectiveName, "@"), (DirectiveName, "loadable"), (Brace, "}")
```

`names_resolve_to_their_leaves_and_the_rest_to_the_declaration` — entrypoint sequence.

`positions_inside_a_failed_first_chunk_resolve_through_the_cloned_chunk` / `the_unrecognized_keyword_resolves_as_a_token_in_the_failed_chunk` — unknown-keyword sequence.

`a_selectable_declaration_parses_with_selections`

```
"field Query.Foo {\n  bar,\n  baz\n}"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (FieldName, "baz"), (Brace, "}")
```

`empty_selection_sets_hold_zero_selections` — `"field Query.Foo {}"`, `"field Query.Foo { }"`, `"field Query.Foo {\n}"`:

```
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (Brace, "}")
```

`a_field_declaration_without_a_selection_set_parses`

```
"field Query.Foo"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo")
```

`a_selection_set_on_its_own_line_is_a_second_declaration` — `"field Query.Foo\n{ bar }"`: field-without-set sequence.

`a_final_comma_after_the_selectable_declaration_is_an_error` / `tokens_after_the_selection_set_are_leftover` / `a_field_without_a_description_has_none` / `a_field_without_to_has_no_target_type`

```
"field Query.Foo { bar }" and "field Query.Foo { bar }," and "field Query.Foo { bar } junk"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_single_line_description_parses_with_its_quotes` / `a_description_resolves_to_its_leaf`

```
"field Query.Foo \"the home route\" { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (String, "\"the home route\""), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_block_string_description_spans_lines_without_splitting_the_chunk`

```
"field Query.Foo \"\"\"\n  the home\n  route\n\"\"\" { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (String, "\"\"\"\n  the home\n  route\n\"\"\""), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_description_after_the_selection_set_is_leftover` — field-with-`{ bar }` sequence.

`an_entrypoint_carries_no_description` — `"entrypoint Query.foo \"nope\""`: entrypoint sequence.

`a_field_declaration_with_only_a_description_parses`

```
"field Query.Foo \"the home route\""
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (String, "\"the home route\"")
```

`a_field_with_to_parses_the_target_type` / `to_and_the_target_resolve_with_their_ancestry`

```
"field Pet.BestFriend to Owner { id }"
(Keyword, "field"), (Type, "Pet"), (Period, "."), (FieldName, "BestFriend"), (Keyword, "to"), (GraphQLTypeName, "Owner"), (Brace, "{"), (FieldName, "id"), (Brace, "}")
```

`a_full_field_parses_in_order`

```
"field Pet.Owner($limit: Int) to Person! \"the owner\" { name }"
(Keyword, "field"), (Type, "Pet"), (Period, "."), (FieldName, "Owner"), (Parenthesis, "("), (Variable, "$"), (Variable, "limit"), (Colon, ":"), (GraphQLTypeName, "Int"), (Parenthesis, ")"), (Keyword, "to"), (GraphQLTypeName, "Person"), (String, "\"the owner\""), (Brace, "{"), (FieldName, "name"), (Brace, "}")
```

`a_to_target_accepts_every_type_annotation_form`

```
"field Query.Foo to Pet { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "Pet"), (Brace, "{"), (FieldName, "id"), (Brace, "}")

"field Query.Foo to Pet! { id }"
same, GraphQLTypeName "Pet" (no token for !)

"field Query.Foo to [Pet] { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "["), (GraphQLTypeName, "Pet"), (GraphQLTypeName, "]"), (Brace, "{"), (FieldName, "id"), (Brace, "}")

"field Query.Foo to [Pet!]! { id }"
same as [Pet] (no tokens for !)

"field Query.Foo to [[Pet]] { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "["), (GraphQLTypeName, "["), (GraphQLTypeName, "Pet"), (GraphQLTypeName, "]"), (GraphQLTypeName, "]"), (Brace, "{"), (FieldName, "id"), (Brace, "}")
```

`a_bracketed_target_is_a_list_annotation` / `a_non_null_list_of_non_null_named_is_list_of_named` — `[Pet!]!` sequence.

`a_named_target_without_bang_is_null_wrapped` — `to Pet { id }` sequence.

`a_named_target_with_bang_is_not_null_wrapped` — `to Pet! { id }` sequence.

`a_list_target_maps_graphql_nullability` / `a_nullable_list_of_non_null_named` / `a_non_null_list_of_nullable_named` — `[Pet]`, `[Pet!]`, `[Pet]!` use the `[Pet]` sequence.

`a_nested_list_target_wraps_null_at_every_layer` — `[[Pet]]` sequence.

`a_second_bang_is_leftover`

```
"field Query.Foo to Pet!! { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "Pet")
```

`a_variable_type_without_bang_is_null_wrapped`

```
"field Query.Foo($x: ID) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "x"), (Colon, ":"), (GraphQLTypeName, "ID"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_variable_type_with_bang_is_not_null_wrapped` / `a_bang_resolves_to_the_variable_declaration` — `"field Query.Foo($x: ID!) { bar }"` and `"field Query.Foo($id: ID!) { bar }"`: same roles, lexeme `"x"` or `"id"`.

`an_empty_list_target_fails_as_a_type`

```
"field Query.Foo to [] { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "["), (GraphQLTypeName, "]")
```

`a_non_to_identifier_is_not_consumed_as_to` / `uppercase_to_is_not_the_keyword`

```
"field Query.Foo Owner { id }"
"field Query.Foo TO Pet { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo")
```

`a_missing_target_type_reports_after_to` / `a_to_at_the_end_of_the_chunk_expects_a_type`

```
"field Pet.BestFriend to { id }"
"field Pet.BestFriend to"
(Keyword, "field"), (Type, "Pet"), (Period, "."), (FieldName, "BestFriend"), (Keyword, "to")
```

`a_to_after_the_description_is_not_a_target`

```
"field Query.Foo \"x\" to Owner { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (String, "\"x\"")
```

`a_final_comma_after_a_field_with_to_is_an_error` — `to Owner { id }` sequence.

`selection_names_resolve_with_their_ancestry`

```
"field Query.Foo { pet { name } }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "pet"), (Brace, "{"), (FieldName, "name"), (Brace, "}"), (Brace, "}")
```

`leftover_positions_resolve_to_the_leftover_token`

```
"field Query.Foo { bar baz }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`positions_inside_a_failed_selection_resolve_through_unparsed_items`

```
"field Query.Foo { 42 }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (Brace, "}")
```

`whitespace_and_separators_inside_a_selection_set_resolve_to_the_set` — `{ bar, baz }` sequence.

`argument_names_resolve_through_the_selection` / `a_dollar_in_a_use_resolves_to_declaration_or_usage_with_usage_parent`

```
"field Query.Foo { bar(id: $x) }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (Parenthesis, "("), (Argument, "id"), (Colon, ":"), (Variable, "$"), (Variable, "x"), (Parenthesis, ")"), (Brace, "}")
```

`a_dollar_in_a_declaration_resolves_to_declaration_or_usage_with_declaration_parent` — `$id: ID { bar }` sequence.

`a_single_line_description_drops_the_quotes` (stream_of)

```
"\"the home route\""
(String, "\"the home route\"")
```

`an_empty_string_is_a_description`

```
"\"\""
(String, "\"\"")
```

`a_block_string_description_is_one_token_including_line_breaks`

```
"\"\"\"\n  the home\n  route\n\"\"\""
(String, "\"\"\"\n  the home\n  route\n\"\"\"")
```

`a_block_string_with_line_breaks_does_not_split_the_chunk`

```
"Foo \"\"\"\n  the home\n  route\n\"\"\" Bar"
(FieldName, "Foo"), (String, "\"\"\"\n  the home\n  route\n\"\"\""), (FieldName, "Bar")
```

`a_description_does_not_consume_the_following_item`

```
"\"hi\" Foo"
(String, "\"hi\""), (FieldName, "Foo")
```

`a_non_string_is_not_a_description` — after `consume_description`, `&[]`. After the following `consume_token_if` Identifier: `(FieldName, "Foo")`. Assert twice, once per phase.

`a_brace_group_is_not_a_description` — after `consume_description`, `&[]`. After `consume_group_if` Brace: `(Brace, "{"), (Brace, "}")`.

`an_unterminated_string_is_not_a_description` — after `consume_description`, `&[]`. After `consume_token_if` ErrorUnterminatedString: `(Error, "\"unterminated")`.

`an_empty_block_string_is_a_description`

```
"\"\"\"\"\"\""
(String, "\"\"\"\"\"\"")
```

`a_multi_line_variable_list_parses_in_the_demo_style`

```
"field Query.PetCheckinListRoute(\n  $id: ID !\n) {\n  pets\n}"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "PetCheckinListRoute"), (Parenthesis, "("), (Variable, "$"), (Variable, "id"), (Colon, ":"), (GraphQLTypeName, "ID"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "pets"), (Brace, "}")
```

`list_types_nest_with_non_null_markers`

```
"field Query.Foo($pets: [Pet!]!) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "pets"), (Colon, ":"), (GraphQLTypeName, "["), (GraphQLTypeName, "Pet"), (GraphQLTypeName, "]"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`defaults_parse_including_variables`

```
"field Query.Foo($limit: Int = 10) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "limit"), (Colon, ":"), (GraphQLTypeName, "Int"), (Equals, "="), (Integer, "10"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")

"field Query.Foo($limit: Int = $other) { bar }"
same through Equals, then (Variable, "$"), (Variable, "other"), then Parenthesis / Brace / FieldName / Brace

"field Query.Foo($ids: [ID!] = [1, $x]) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "ids"), (Colon, ":"), (GraphQLTypeName, "["), (GraphQLTypeName, "ID"), (GraphQLTypeName, "]"), (Equals, "="), (Bracket, "["), (Integer, "1"), (Variable, "$"), (Variable, "x"), (Bracket, "]"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")

"field Query.Foo($input: Input = { pet: $pet }) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "input"), (Colon, ":"), (GraphQLTypeName, "Input"), (Equals, "="), (Brace, "{"), (ObjectKey, "pet"), (Colon, ":"), (Variable, "$"), (Variable, "pet"), (Brace, "}"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")

"field Query.Foo($foo: String = \"foo\", $bar: Input = { foo: $foo }) { baz }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "foo"), (Colon, ":"), (GraphQLTypeName, "String"), (Equals, "="), (String, "\"foo\""), (Variable, "$"), (Variable, "bar"), (Colon, ":"), (GraphQLTypeName, "Input"), (Equals, "="), (Brace, "{"), (ObjectKey, "foo"), (Colon, ":"), (Variable, "$"), (Variable, "foo"), (Brace, "}"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "baz"), (Brace, "}")
```

`a_default_variable_resolves_through_variable_default` — `$limit: Int = $other` sequence.

`each_malformed_variable_declaration_degrades_alone`

```
"field Query.Foo($a Int, $b: , id: ID, $c: Float) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "a"), (Variable, "$"), (Variable, "b"), (Colon, ":"), (Variable, "$"), (Variable, "c"), (Colon, ":"), (GraphQLTypeName, "Float"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_final_comma_inside_a_list_type_is_end_of_type` / `a_line_break_inside_a_list_type_does_not_attach_bang` / `type_names_resolve_through_their_annotation_ancestry` — `$pets: [Pet,]` / `[Pet\n!]` / `[Pet]` use the `[Pet]` variable sequence (`$pets` / `[` / `Pet` / `]` / `{ bar }`).

`an_empty_variable_list_is_some_and_empty`

```
"field Query.Foo() { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_to_without_a_selection_set_parses`

```
"field Query.Foo to Pet"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "Pet")
```

`a_directive_without_vars_or_to_parses` / `field_directive_names_resolve_through_the_declaration`

```
"field Query.Foo @component { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (DirectiveName, "@"), (DirectiveName, "component"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`a_full_header_includes_directives`

```
"field Pet.Owner($limit: Int) to Person! @updatable \"the owner\" { name }"
(Keyword, "field"), (Type, "Pet"), (Period, "."), (FieldName, "Owner"), (Parenthesis, "("), (Variable, "$"), (Variable, "limit"), (Colon, ":"), (GraphQLTypeName, "Int"), (Parenthesis, ")"), (Keyword, "to"), (GraphQLTypeName, "Person"), (DirectiveName, "@"), (DirectiveName, "updatable"), (String, "\"the owner\""), (Brace, "{"), (FieldName, "name"), (Brace, "}")
```

`two_directives_on_a_field_stay_in_one_list`

```
"field Query.Foo @a @b { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (DirectiveName, "@"), (DirectiveName, "a"), (DirectiveName, "@"), (DirectiveName, "b"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`two_directives_on_an_entrypoint_stay_in_one_list`

```
"entrypoint Query.foo @a @b"
(Keyword, "entrypoint"), (Type, "Query"), (Period, "."), (FieldName, "foo"), (DirectiveName, "@"), (DirectiveName, "a"), (DirectiveName, "@"), (DirectiveName, "b")
```

`a_directive_after_the_description_is_leftover`

```
"field Query.Foo \"x\" @component { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (String, "\"x\"")
```

`a_to_after_a_directive_is_leftover`

```
"field Query.Foo @component to Pet { id }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (DirectiveName, "@"), (DirectiveName, "component")
```

`a_dollar_without_a_name_fails_that_variable`

```
"field Query.Foo($) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`an_equals_without_a_default_fails_that_variable`

```
"field Query.Foo($id: ID =) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "id"), (Colon, ":"), (GraphQLTypeName, "ID"), (Equals, "="), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`an_alias_without_a_name_fails_that_selection`

```
"field Query.Foo { b: }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "b"), (Colon, ":"), (Brace, "}")
```

`to_as_a_selectable_name_is_the_name`

```
"field Query.to { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "to"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`to_as_a_target_type_name_parses`

```
"field Query.Foo to to { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Keyword, "to"), (GraphQLTypeName, "to"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`true_false_and_null_as_selection_names_are_selections` — for each `name` in `["true", "false", "null"]`:

```
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, name), (Brace, "}")
```

`a_directive_with_empty_arguments_parses`

```
"field Query.Foo { bar @loadable() }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (DirectiveName, "@"), (DirectiveName, "loadable"), (Parenthesis, "("), (Parenthesis, ")"), (Brace, "}")
```

`a_selection_with_alias_arguments_directives_and_a_nested_set_parses`

```
"field Query.Foo { a: bar(id: $id) @loadable { baz } }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "a"), (Colon, ":"), (FieldName, "bar"), (Parenthesis, "("), (Argument, "id"), (Colon, ":"), (Variable, "$"), (Variable, "id"), (Parenthesis, ")"), (DirectiveName, "@"), (DirectiveName, "loadable"), (Brace, "{"), (FieldName, "baz"), (Brace, "}"), (Brace, "}")
```

`a_field_with_only_variables_parses`

```
"field Query.Foo($id: ID)"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Parenthesis, "("), (Variable, "$"), (Variable, "id"), (Colon, ":"), (GraphQLTypeName, "ID"), (Parenthesis, ")")
```

`to_as_a_parent_type_name_is_the_parent`

```
"field to.Foo { bar }"
(Keyword, "field"), (Type, "to"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

`uppercase_field_is_not_the_keyword`

```
"FIELD Query.Foo { bar }"
(Keyword, "FIELD")
```

`at_without_a_name_fails_that_selection`

```
"field Query.Foo { bar @ }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (Brace, "{"), (FieldName, "bar"), (DirectiveName, "@"), (Brace, "}")
```

`an_entrypoint_directive_with_arguments_parses`

```
"entrypoint Query.foo @lazyLoad(x: 1)"
(Keyword, "entrypoint"), (Type, "Query"), (Period, "."), (FieldName, "foo"), (DirectiveName, "@"), (DirectiveName, "lazyLoad"), (Parenthesis, "("), (Argument, "x"), (Colon, ":"), (Integer, "1"), (Parenthesis, ")")
```

`a_field_directive_with_arguments_parses`

```
"field Query.Foo @component(x: 1) { bar }"
(Keyword, "field"), (Type, "Query"), (Period, "."), (FieldName, "Foo"), (DirectiveName, "@"), (DirectiveName, "component"), (Parenthesis, "("), (Argument, "x"), (Colon, ":"), (Integer, "1"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "bar"), (Brace, "}")
```

### `selections.rs`

`selections_without_a_nested_set`

```
"bar, baz"
(FieldName, "bar"), (FieldName, "baz")
```

`a_single_selection_parses_without_a_trailing_separator` — `"bar"`: `(FieldName, "bar")`.

`empty_and_whitespace_levels_hold_zero_selections` — `""`, `"   "`, `"\n"`: `&[]`.

`a_comma_before_the_first_selection_is_chunkings_error` — `", bar"`: `(FieldName, "bar")`. Lone `","`: `&[]`.

`an_alias_splits_from_the_name_at_the_colon`

```
"b: bar"
(FieldName, "b"), (Colon, ":"), (FieldName, "bar")
```

`selections_nest`

```
"pet { name, age }"
(FieldName, "pet"), (Brace, "{"), (FieldName, "name"), (FieldName, "age"), (Brace, "}")
```

`arguments_parse_on_selections`

```
"pet(id: $petId) { name(shouted: true) }"
(FieldName, "pet"), (Parenthesis, "("), (Argument, "id"), (Colon, ":"), (Variable, "$"), (Variable, "petId"), (Parenthesis, ")"), (Brace, "{"), (FieldName, "name"), (Parenthesis, "("), (Argument, "shouted"), (Colon, ":"), (BooleanOrNull, "true"), (Parenthesis, ")"), (Brace, "}")
```

`an_orphaned_group_after_a_line_break_is_a_failed_selection`

```
"bar\n{ baz }"
(FieldName, "bar")
```

`a_doubled_comma_between_selections_is_chunkings_error`

```
"a,, b"
(FieldName, "a"), (FieldName, "b")
```

`leftover_after_a_selection_keeps_the_item`

```
"bar baz\nqux"
(FieldName, "bar"), (FieldName, "qux")
```

`a_period_where_a_selection_should_start_is_a_selection_error` — `"...UserAvatar"`: `&[]`.

`errors_collect_in_source_order_across_nesting`

```
"a b\npet { c d }\ne f"
(FieldName, "a"), (FieldName, "pet"), (Brace, "{"), (FieldName, "c"), (Brace, "}"), (FieldName, "e")
```

### `arguments.rs`

`pairs_parse_as_name_colon_value`

```
"id: $petId, shouted: true"
(Argument, "id"), (Colon, ":"), (Variable, "$"), (Variable, "petId"), (Argument, "shouted"), (Colon, ":"), (BooleanOrNull, "true")
```

`each_value_kind_parses`

```
"a: $x, b: \"hi\", c: 42, d: -7, e: true, f: false, g: null"
(Argument, "a"), (Colon, ":"), (Variable, "$"), (Variable, "x"), (Argument, "b"), (Colon, ":"), (String, "\"hi\""), (Argument, "c"), (Colon, ":"), (Integer, "42"), (Argument, "d"), (Colon, ":"), (Integer, "-7"), (Argument, "e"), (Colon, ":"), (BooleanOrNull, "true"), (Argument, "f"), (Colon, ":"), (BooleanOrNull, "false"), (Argument, "g"), (Colon, ":"), (BooleanOrNull, "null")
```

`object_values_use_braces`

```
"input: { id: 4, nested: { on: true } }"
(Argument, "input"), (Colon, ":"), (Brace, "{"), (ObjectKey, "id"), (Colon, ":"), (Integer, "4"), (ObjectKey, "nested"), (Colon, ":"), (Brace, "{"), (ObjectKey, "on"), (Colon, ":"), (BooleanOrNull, "true"), (Brace, "}"), (Brace, "}")
```

`empty_and_whitespace_levels_hold_zero_pairs` — `&[]`.

`a_trailing_comma_after_a_pair_is_not_a_parse_error`

```
"id: 1,"
(Argument, "id"), (Colon, ":"), (Integer, "1")
```

`integer_overflow_is_a_typed_error_on_that_pair` — `require_token` records the integer, then parse fails:

```
"a: 99999999999999999999, b: 1"
(Argument, "a"), (Colon, ":"), (Integer, "99999999999999999999"), (Argument, "b"), (Colon, ":"), (Integer, "1")
```

`a_malformed_pair_degrades_that_pair_alone`

```
"a 1, b: 2"
(Argument, "a"), (Argument, "b"), (Colon, ":"), (Integer, "2")
```

`a_non_value_identifier_is_an_error_at_the_value` — `parse_boolean_or_null` records then rejects:

```
"a: yes"
(Argument, "a"), (Colon, ":"), (BooleanOrNull, "yes")
```

`a_pair_that_does_not_start_with_a_name_is_an_argument_error` — `"42: 1"`: `&[]`.

`leftover_after_a_pair_keeps_the_item`

```
"id: $x junk"
(Argument, "id"), (Colon, ":"), (Variable, "$"), (Variable, "x")
```

`a_doubled_comma_between_pairs_is_chunkings_error`

```
"a: 1,, b: 2"
(Argument, "a"), (Colon, ":"), (Integer, "1"), (Argument, "b"), (Colon, ":"), (Integer, "2")
```

`consume_argument_list_reads_a_paren_group`

```
"(id: $petId)"
(Parenthesis, "("), (Argument, "id"), (Colon, ":"), (Variable, "$"), (Variable, "petId"), (Parenthesis, ")")
```

`an_empty_paren_group_is_zero_pairs`

```
"()"
(Parenthesis, "("), (Parenthesis, ")")
```

`a_list_interior_holds_three_values`

```
"1, $x, true"
(Integer, "1"), (Variable, "$"), (Variable, "x"), (BooleanOrNull, "true")
```

`a_list_value_parses_nested_lists_and_objects`

```
"[[1], { a: 2 }]"
(Bracket, "["), (Bracket, "["), (Integer, "1"), (Bracket, "]"), (Brace, "{"), (ObjectKey, "a"), (Colon, ":"), (Integer, "2"), (Brace, "}"), (Bracket, "]")
```

`empty_and_whitespace_list_interiors_are_empty` — `"[]"`, `"[ ]"`, `"[\n]"`:

```
(Bracket, "["), (Bracket, "]")
```

`a_trailing_comma_in_a_list_is_not_a_parse_error` — `"1,"`: `(Integer, "1")`.

`leftover_after_a_list_value_keeps_the_item` — `"1 junk"`: `(Integer, "1")`.

`a_doubled_comma_in_a_list_is_chunkings_error`

```
"1,, 2"
(Integer, "1"), (Integer, "2")
```

`a_list_parses_as_an_argument_value`

```
"id: [1, 2]"
(Argument, "id"), (Colon, ":"), (Bracket, "["), (Integer, "1"), (Integer, "2"), (Bracket, "]")
```

`integer_underflow_is_a_typed_error_on_that_pair`

```
"a: -99999999999999999999, b: 1"
(Argument, "a"), (Colon, ":"), (Integer, "-99999999999999999999"), (Argument, "b"), (Colon, ":"), (Integer, "1")
```

`zero_parses_as_an_integer` — `"a: 0"`: `(Argument, "a"), (Colon, ":"), (Integer, "0")`.

`i64_min_parses` — `"a: -9223372036854775808"`: `(Argument, "a"), (Colon, ":"), (Integer, "-9223372036854775808")`.

`i64_max_parses` — `"a: 9223372036854775807"`: `(Argument, "a"), (Colon, ":"), (Integer, "9223372036854775807")`.

`a_leading_zero_integer_is_not_a_value` — `"a: 01"`: `(Argument, "a"), (Colon, ":")`.

`a_float_is_not_a_value` — `"a: 1.5"`: `(Argument, "a"), (Colon, ":")`.

`an_empty_object_is_a_value`

```
"input: {}"
(Argument, "input"), (Colon, ":"), (Brace, "{"), (Brace, "}")
```

`a_quoted_string_value_drops_the_quotes` — `"a: \"hi\""`: `(Argument, "a"), (Colon, ":"), (String, "\"hi\"")`.

`a_quoted_string_does_not_process_escapes` — `"a: \"hi\\n\""`: `(Argument, "a"), (Colon, ":"), (String, "\"hi\\n\"")`.

`a_quoted_string_interns_inner_quote_characters` — the fixture `r#"a: "\"hi\"""#`: `(Argument, "a"), (Colon, ":"), (String, "\"\\\"hi\\\"\"")`.

`a_block_string_is_a_value`

```
"a: \"\"\"hi\"\"\""
(Argument, "a"), (Colon, ":"), (String, "\"\"\"hi\"\"\"")
```

`a_one_line_block_string_keeps_leading_spaces` — the fixture `r#"a: """   hi""""#`: `(Argument, "a"), (Colon, ":"), (String, "\"\"\"   hi\"\"\"")`.

`negative_zero_parses_as_zero`

```
"a: -0"
(Argument, "a"), (Colon, ":"), (Integer, "-0")
```

`an_empty_string_is_a_value`

```
"a: \"\""
(Argument, "a"), (Colon, ":"), (String, "\"\"")
```

`an_empty_block_string_is_a_value`

```
"a: \"\"\"\"\"\""
(Argument, "a"), (Colon, ":"), (String, "\"\"\"\"\"\"")
```

`a_colon_without_a_value_fails_that_pair`

```
"a:"
(Argument, "a"), (Colon, ":")
```

`a_dollar_without_a_name_is_not_a_value`

```
"a: $"
(Argument, "a"), (Colon, ":"), (Variable, "$")
```

`a_paren_group_is_not_a_value`

```
"a: (x)"
(Argument, "a"), (Colon, ":")
```

`a_non_integer_number_is_not_a_value` — `"a: .5"`, `"a: 1."`, `"a: 1e2"`: `(Argument, "a"), (Colon, ":")`.

`an_object_entry_that_does_not_start_with_a_name_fails_that_entry`

```
"input: { 1: 2 }"
(Argument, "input"), (Colon, ":"), (Brace, "{"), (Brace, "}")
```

### `chunk.rs` `parse_each_chunk` tests

`parse_each_chunk_on_an_empty_level_is_no_slots_and_no_errors` — `&[]`.

`parse_each_chunk_parses_one_identifier_per_chunk`

```
"foo, bar"
(FieldName, "foo"), (FieldName, "bar")
```

`a_list_trailing_comma_is_not_a_parse_each_chunk_diagnostic` — `"foo,"`: `(FieldName, "foo")`.

`leftover_after_a_list_item_keeps_the_item` — `"foo bar"`: `(FieldName, "foo")`.

`a_failed_list_chunk_is_none_and_the_next_chunk_still_parses`

```
".\nfoo"
(FieldName, "foo")
```

`a_line_break_is_a_list_separator` — `"foo\nbar"`: `(FieldName, "foo"), (FieldName, "bar")`.

`a_comma_without_item_is_chunkings_error_and_the_item_parses` — `",foo"`: `(FieldName, "foo")`.
