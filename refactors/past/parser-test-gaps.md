# Parser grammar-stage tests

Fixtures and assertions for the grammar stage. Helpers are the ones in each file's `tests` module (`parsed`, `assert_no_declaration`, `span_of`, `as_selectable`, `as_declared`, `variables_of`, `parsed_pairs`, `parsed_selections`).

## parse_iso_literal.rs

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn an_empty_variable_list_is_some_and_empty() {
        let text = "field Query.Foo() { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let variables = as_selectable(parse.reference())
            .variable_definitions
            .as_ref()
            .expect("() is a present list");
        assert_eq!(variables.item.0.len(), 0);
        assert!(as_selectable(parse.reference()).selection_set.is_some());
    }

    #[test]
    fn a_to_without_a_selection_set_parses() {
        let text = "field Query.Foo to Pet";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert_eq!(
            declaration
                .target_type
                .as_ref()
                .expect("the fixture writes to Pet")
                .location,
            span_of(text, "Pet")
        );
        assert_eq!(declaration.selection_set, None);
    }

    #[test]
    fn a_directive_without_vars_or_to_parses() {
        let text = "field Query.Foo @component { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let field = as_selectable(parse.reference());
        assert!(field.variable_definitions.is_none());
        assert_eq!(field.target_type, None);
        assert_eq!(
            field
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .location,
            span_of(text, "@component")
        );
    }

    #[test]
    fn a_full_header_includes_directives() {
        let text = "field Pet.Owner($limit: Int) to Person! @updatable \"the owner\" { name }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.variable_definitions.is_some());
        assert!(declaration.target_type.is_some());
        assert!(declaration.directive_set.is_some());
        assert!(declaration.description.is_some());
        assert!(declaration.selection_set.is_some());
    }

    #[test]
    fn two_directives_on_a_field_stay_in_one_list() {
        let text = "field Query.Foo @a @b { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let directives = as_selectable(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries directives");
        assert_eq!(directives.item.0.len(), 2);
        assert_eq!(directives.location, span_of(text, "@a @b"));
    }

    #[test]
    fn two_directives_on_an_entrypoint_stay_in_one_list() {
        let text = "entrypoint Query.foo @a @b";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let directives = as_entrypoint(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries directives");
        assert_eq!(directives.item.0.len(), 2);
    }

    #[test]
    fn a_directive_after_the_description_is_leftover() {
        let text = "field Query.Foo \"x\" @component { bar }";
        let (parse, errors) = parsed(text);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.description.is_some());
        assert_eq!(declaration.directive_set, None);
        assert_eq!(
            errors[0],
            expected(EndOfDeclaration, Found::Token(At)).with_span(span_of(text, "@")),
        );
    }

    #[test]
    fn a_to_after_a_directive_is_leftover() {
        let text = "field Query.Foo @component to Pet { id }";
        let (parse, errors) = parsed(text);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.directive_set.is_some());
        assert_eq!(declaration.target_type, None);
        assert_eq!(
            errors[0],
            expected(EndOfDeclaration, Found::Token(Identifier)).with_span(span_of(text, "to")),
        );
    }

    #[test]
    fn a_dollar_without_a_name_fails_that_variable() {
        let text = "field Query.Foo($) { bar }";
        let (parse, errors) = parsed(text);
        as_selectable(parse.reference());
        let variables = variables_of(parse.reference());
        assert!(variables.item.0[0].item.item.is_none());
        let dollar_end = span_of(text, "$").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(dollar_end, dollar_end)
        }));
    }

    #[test]
    fn an_equals_without_a_default_fails_that_variable() {
        let text = "field Query.Foo($id: ID =) { bar }";
        let (parse, errors) = parsed(text);
        as_selectable(parse.reference());
        let variables = variables_of(parse.reference());
        assert!(variables.item.0[0].item.item.is_none());
        let eq_end = span_of(text, "=").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(Expectation::Value, Found::EndOfChunk)
                && error.location == Span::new(eq_end, eq_end)
        }));
    }

    #[test]
    fn an_alias_without_a_name_fails_that_selection() {
        let text = "field Query.Foo { b: }";
        let (parse, errors) = parsed(text);
        let items = selections(selection_set_of(as_selectable(parse.reference())));
        assert!(items[0].item.item.is_none());
        let colon_end = span_of(text, ":").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(colon_end, colon_end)
        }));
    }

    #[test]
    fn to_as_a_selectable_name_is_the_name() {
        let text = "field Query.to { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_selectable(parse.reference()).name.location,
            span_of(text, "to")
        );
        assert_eq!(as_selectable(parse.reference()).target_type, None);
    }

    #[test]
    fn to_as_a_target_type_name_parses() {
        let text = "field Query.Foo to to { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_selectable(parse.reference()).name.location,
            span_of(text, "Foo")
        );
        match as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes a target")
            .item
            .reference()
        {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.item, EntityNameWrapper("to".intern().to()));
            }
            annotation => panic!("expected a named target, got {annotation:?}"),
        }
    }

    #[test]
    fn uppercase_to_is_not_the_keyword() {
        let text = "field Query.Foo TO Pet { bar }";
        let (parse, errors) = parsed(text);
        as_selectable(parse.reference());
        assert_eq!(as_selectable(parse.reference()).target_type, None);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "TO"))
                .wrap_vec(),
        );
    }

    #[test]
    fn true_false_and_null_as_selection_names_are_selections() {
        for name in ["true", "false", "null"] {
            let text = format!("field Query.Foo {{ {name} }}");
            let (parse, errors) = parsed(text.reference());
            assert_eq!(errors, vec![], "for literal {text:?}");
            assert_eq!(
                as_selection(
                    selections(selection_set_of(as_selectable(parse.reference())))[0]
                        .item
                        .reference()
                )
                .name
                .item,
                SelectionNameWrapper(name.intern().to()),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn a_directive_with_empty_arguments_parses() {
        let text = "field Query.Foo { bar @loadable() }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let arguments = as_selection(
            selections(selection_set_of(as_selectable(parse.reference())))[0]
                .item
                .reference(),
        )
        .directive_set
        .as_ref()
        .expect("the fixture selects with a directive")
        .item
        .0[0]
        .item
        .arguments
        .as_ref()
        .expect("() is a present list");
        assert_eq!(arguments.item.0.len(), 0);
    }

    #[test]
    fn a_selection_with_alias_arguments_directives_and_a_nested_set_parses() {
        let text = "field Query.Foo { a: bar(id: $id) @loadable { baz } }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let selection = as_selection(
            selections(selection_set_of(as_selectable(parse.reference())))[0]
                .item
                .reference(),
        );
        assert!(selection.reader_alias.is_some());
        assert!(selection.arguments.is_some());
        assert!(selection.directive_set.is_some());
        assert!(selection.selection_set.is_some());
    }

    #[test]
    fn field_directive_names_resolve_through_the_declaration() {
        let text = "field Query.Foo @component { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "component")) {
            IsographResolutionNode::IsographDirectiveNameWrapper(name) => {
                match name.parent.parent.parent {
                    IsographFieldDirectiveListParent::SelectableDeclaration(_) => {}
                    parent => panic!("expected a field directive list, got {parent:?}"),
                }
            }
            node => panic!("expected the directive name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_field_with_only_variables_parses() {
        let text = "field Query.Foo($id: ID)";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert_eq!(variables_of(parse.reference()).item.0.len(), 1);
        assert_eq!(declaration.selection_set, None);
    }

    #[test]
    fn to_as_a_parent_type_name_is_the_parent() {
        let text = "field to.Foo { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_selectable(parse.reference()).parent_type.item,
            EntityNameWrapper("to".intern().to())
        );
        assert_eq!(
            as_selectable(parse.reference()).parent_type.location,
            span_of(text, "to")
        );
        assert_eq!(as_selectable(parse.reference()).target_type, None);
    }

    #[test]
    fn uppercase_field_is_not_the_keyword() {
        let text = "FIELD Query.Foo { bar }";
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Token(Identifier)),
            span_of(text, "FIELD"),
        );
    }

    #[test]
    fn at_without_a_name_fails_that_selection() {
        let text = "field Query.Foo { bar @ }";
        let (parse, errors) = parsed(text);
        let items = selections(selection_set_of(as_selectable(parse.reference())));
        assert_eq!(items.len(), 1);
        assert!(items[0].item.item.is_none());
        let at_end = span_of(text, "@").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(at_end, at_end)
        }));
    }

    #[test]
    fn an_entrypoint_directive_with_arguments_parses() {
        let text = "entrypoint Query.foo @lazyLoad(x: 1)";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let arguments = as_entrypoint(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries a directive")
            .item
            .0[0]
            .item
            .arguments
            .as_ref()
            .expect("the fixture passes arguments");
        assert_eq!(arguments.item.0.len(), 1);
        assert_eq!(arguments.location, span_of(text, "(x: 1)"));
    }

    #[test]
    fn a_field_directive_with_arguments_parses() {
        let text = "field Query.Foo @component(x: 1) { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let arguments = as_selectable(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries a directive")
            .item
            .0[0]
            .item
            .arguments
            .as_ref()
            .expect("the fixture passes arguments");
        assert_eq!(arguments.item.0.len(), 1);
        assert_eq!(arguments.location, span_of(text, "(x: 1)"));
    }

    #[test]
    fn a_field_records_keyword_type_to_and_selections() {
        let text = "field Query.Foo to Pet { id }";
        let (parse, errors, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        let parse = parse.expect("the fixture is not an empty literal");
        as_selectable(parse.reference());
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "field")),
                SemanticToken::Type.with_span(span_of(text, "Query")),
                SemanticToken::Period.with_span(span_of(text, ".")),
                SemanticToken::FieldName.with_span(span_of(text, "Foo")),
                SemanticToken::Keyword.with_span(span_of(text, "to")),
                SemanticToken::GraphQLTypeName.with_span(span_of(text, "Pet")),
                SemanticToken::Brace.with_span(span_of(text, "{")),
                SemanticToken::FieldName.with_span(span_of(text, "id")),
                SemanticToken::Brace.with_span(span_of(text, "}")),
            ],
        );
    }
```

Need `At` in the `NonBracketTokenKind` import for leftover `@`.

## arguments.rs

```rust
// from crates/isograph_parser/src/arguments.rs
    #[test]
    fn integer_underflow_is_a_typed_error_on_that_pair() {
        let text = "a: -99999999999999999999, b: 1";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        as_argument(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "-99999999999999999999")
        }));
    }

    #[test]
    fn zero_parses_as_an_integer() {
        let text = "a: 0";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(0))
        ));
    }

    #[test]
    fn i64_min_parses() {
        let text = "a: -9223372036854775808";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(i64::MIN))
        ));
    }

    #[test]
    fn a_leading_zero_integer_is_not_a_value() {
        let text = "a: 01";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::ErrorNumberLiteralLeadingZero),
                )
                && error.location == span_of(text, "01")
        }));
    }

    #[test]
    fn a_float_does_not_fit_i64() {
        let text = "a: 1.5";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item == ParseError::IntegerDoesNotFitI64
                && error.location == span_of(text, "1.5")
        }));
    }

    #[test]
    fn an_empty_object_is_a_value() {
        let text = "input: {}";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference()).value.item.reference() {
            NonConstantValue::Object(object) => assert_eq!(object.0.len(), 0),
            value => panic!("expected an object, got {value:?}"),
        }
    }

    #[test]
    fn a_block_string_is_not_a_value() {
        let text = "a: \"\"\"hi\"\"\"";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::BlockStringLiteral),
                )
                && error.location == span_of(text, "\"\"\"hi\"\"\"")
        }));
    }

    #[test]
    fn i64_max_parses() {
        let text = "a: 9223372036854775807";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(i64::MAX))
        ));
    }

    #[test]
    fn negative_zero_parses_as_zero() {
        let text = "a: -0";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(0))
        ));
    }

    #[test]
    fn an_empty_string_is_a_value() {
        let text = "a: \"\"";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::String(_)
        ));
    }

    #[test]
    fn a_colon_without_a_value_fails_that_pair() {
        let text = "a:";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        let colon_end = span_of(text, ":").end;
        assert!(errors.iter().any(|error| {
            error.item == ParseError::expected(Expectation::Value, Found::EndOfChunk)
                && error.location == Span::new(colon_end, colon_end)
        }));
    }

    #[test]
    fn a_dollar_without_a_name_is_not_a_value() {
        let text = "a: $";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        let dollar_end = span_of(text, "$").end;
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Token(NonBracketTokenKind::Identifier),
                    Found::EndOfChunk,
                )
                && error.location == Span::new(dollar_end, dollar_end)
        }));
    }

    #[test]
    fn a_paren_group_is_not_a_value() {
        let text = "a: (x)";
        let (items, errors, _) = parsed_pairs(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Value,
                    Found::Group(BracketKind::Parenthesis),
                )
                && error.location == span_of(text, "(x)")
        }));
    }

    #[test]
    fn a_non_integer_number_is_not_a_value() {
        for (text, found, pattern) in [
            (
                "a: .5",
                Found::Token(NonBracketTokenKind::ErrorFloatLiteralMissingZero),
                ".5",
            ),
            (
                "a: 1.",
                Found::Token(NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid),
                "1.",
            ),
            ("a: 1e2", Found::Token(NonBracketTokenKind::Error), "1e2"),
        ] {
            let (items, errors, _) = parsed_pairs(text);
            assert!(items[0].item.item.is_none(), "for literal {text:?}");
            assert!(
                errors.iter().any(|error| {
                    error.item == ParseError::expected(Expectation::Value, found)
                        && error.location == span_of(text, pattern)
                }),
                "for literal {text:?}, errors were {errors:?}",
            );
        }
    }

    #[test]
    fn an_object_entry_that_does_not_start_with_a_name_fails_that_entry() {
        let text = "input: { 1: 2 }";
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors.len(), 1);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::Object(object) => {
                assert!(object.0[0].item.item.is_none());
            }
            value => panic!("expected an object, got {value:?}"),
        }
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::ObjectEntry,
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
                && error.location == span_of(text, "1")
        }));
    }
```

`tokenize("1.5")` is one `IntegerLiteral` covering the whole slice. `a: 1.5` then fails as `IntegerDoesNotFitI64`. Block strings are not values: `parse_non_constant_value` matches `StringLiteral` only.

## selections.rs

Covered by the alias-without-name and `true`-as-name tests in parse_iso_literal.rs (they go through `parse_selection`). No extra module test required unless we want the same facts without a wrapping field.

## parse_error.rs

```rust
// from crates/isograph_parser/src/parse_error.rs
        assert_eq!(
            Expectation::OneOf(&[
                Expectation::Keyword("to"),
                Expectation::Description,
                Expectation::SelectionSet,
            ])
            .to_string(),
            "the keyword `to`, a description, or a selection set, like '{ id, name }'",
        );
        assert_eq!(Expectation::OneOf(&[]).to_string(), "one of");
        assert_eq!(
            Expectation::OneOf(&[Expectation::Description]).to_string(),
            "a description",
        );
        assert_eq!(Expectation::Description.to_string(), "a description");
        assert_eq!(
            Expectation::SelectionSet.to_string(),
            "a selection set, like '{ id, name }'",
        );
        assert_eq!(Expectation::Selection.to_string(), "a selection");
        assert_eq!(
            Expectation::Argument.to_string(),
            "an argument, like 'id: $id'",
        );
        assert_eq!(
            Expectation::Value.to_string(),
            "a value, like $foo, 42, \"bar\", true, false, null, or an object literal",
        );
        assert_eq!(
            Expectation::ObjectEntry.to_string(),
            "an object entry, like 'id: 4'",
        );
        assert_eq!(
            Expectation::VariableDeclarationOrUsage.to_string(),
            "a variable declaration, like '$id: ID!'",
        );
        assert_eq!(
            Expectation::TypeAnnotation.to_string(),
            "a type, like 'String', 'String!', or '[String]'",
        );
        assert_eq!(Expectation::EndOfType.to_string(), "the end of the type");
        assert_eq!(
            Expectation::Separator(BracketKind::Brace).to_string(),
            "a comma, a line break, or '}'",
        );
        assert_eq!(
            Expectation::Separator(BracketKind::Bracket).to_string(),
            "a comma, a line break, or ']'",
        );
    }

    #[test]
    fn parse_error_unit_variants_use_their_messages() {
        assert_eq!(
            ParseError::EmptyLiteral.to_string(),
            "Expected a declaration. An isograph literal cannot be empty.",
        );
        assert_eq!(
            ParseError::MultipleDeclarations.to_string(),
            "Expected nothing after the declaration. Each literal holds exactly one declaration.",
        );
        assert_eq!(
            ParseError::IntegerDoesNotFitI64.to_string(),
            "This integer does not fit in a 64-bit signed integer.",
        );
    }
```

## tokenize.rs

```rust
// from crates/isograph_parser/src/tokenize.rs
    #[test]
    fn punctuation_and_sigils_are_their_kinds() {
        let tokens = tokenize("$@:=!,");
        let kinds: Vec<_> = tokens.iter().map(|token| token.item).collect();
        assert_eq!(
            kinds,
            vec![
                IsographLangTokenKind::Dollar,
                IsographLangTokenKind::At,
                IsographLangTokenKind::Colon,
                IsographLangTokenKind::Equals,
                IsographLangTokenKind::Exclamation,
                IsographLangTokenKind::Comma,
            ]
        );
    }

    #[test]
    fn a_string_and_a_block_string_are_their_kinds() {
        assert_eq!(
            tokenize("\"hi\"")[0].item,
            IsographLangTokenKind::StringLiteral
        );
        assert_eq!(
            tokenize("\"\"")[0].item,
            IsographLangTokenKind::StringLiteral
        );
        assert_eq!(
            tokenize("\"\"\"hi\"\"\"")[0].item,
            IsographLangTokenKind::BlockStringLiteral
        );
    }

    #[test]
    fn integers_are_their_kind() {
        for text in ["0", "-0", "42", "-7", "9223372036854775807"] {
            assert_eq!(
                tokenize(text)[0].item,
                IsographLangTokenKind::IntegerLiteral,
                "for literal {text:?}"
            );
        }
        assert_eq!(
            tokenize("1.5")[0].item,
            IsographLangTokenKind::IntegerLiteral
        );
    }

    #[test]
    fn brackets_are_their_kinds() {
        let kinds: Vec<_> = tokenize("(){}[]").iter().map(|token| token.item).collect();
        assert_eq!(
            kinds,
            vec![
                IsographLangTokenKind::OpenParenthesis,
                IsographLangTokenKind::CloseParenthesis,
                IsographLangTokenKind::OpenBrace,
                IsographLangTokenKind::CloseBrace,
                IsographLangTokenKind::OpenBracket,
                IsographLangTokenKind::CloseBracket,
            ]
        );
    }

    #[test]
    fn number_and_string_errors_are_their_kinds() {
        assert_eq!(
            tokenize("01")[0].item,
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero
        );
        assert_eq!(
            tokenize(".5")[0].item,
            IsographLangTokenKind::ErrorFloatLiteralMissingZero
        );
        assert_eq!(
            tokenize("1.")[0].item,
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
        );
        assert_eq!(tokenize("1e2")[0].item, IsographLangTokenKind::Error);
        assert_eq!(tokenize("-")[0].item, IsographLangTokenKind::Error);
        let unterminated = tokenize("\"unterminated");
        assert_eq!(unterminated[0].item, IsographLangTokenKind::Error);
        assert_eq!(unterminated[0].location, Span::new(0, 1));
        assert_eq!(unterminated[1].item, IsographLangTokenKind::Identifier);
        let block = tokenize("\"\"\"unterminated");
        assert_eq!(block[0].item, IsographLangTokenKind::Error);
        assert_eq!(block[1].item, IsographLangTokenKind::Identifier);
    }
```

`lex_string` / `lex_block_string` returning false emit `Error`, not `ErrorUnterminatedString` or `ErrorUnterminatedBlockString`. The opening quote (or `"""`) is the error token; the rest is an identifier.

## Not missing

`a_to_after_the_description_is_not_a_target` already covers `to` after a description. `each_value_kind_parses` covers `$`, strings, positives, negatives, `true`/`false`/`null`. `defaults_parse_including_variables` covers list and object defaults with `$`. `an_empty_list_target_fails_as_a_type` covers `to []`. Overflow and underflow are both tested. `ErrorUnterminatedString` and `ErrorUnterminatedBlockString` are never emitted.
