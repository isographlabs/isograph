use common_lang_types::QueryText;
use intern::Lookup;
use isograph_compiler::{
    CompletedSelection, CompletedSelectionMap, CompletedSelectionTarget, Condition,
    EntrypointOperation, FieldVariable, MergedArguments, MergedValue, NonNullVariableType,
    SelectionKey, VariableType,
};
use isograph_parser::Boolean;
use prelude::Postfix;

use crate::GraphQLNetworkProtocol;
use crate::protocol_data::TYPENAME;

/// The text of `operation`: one selection per line, two spaces of indentation per depth, one
/// newline between lines and none at the end.
pub(crate) fn print_operation(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> QueryText {
    let variables = if operation.variables.is_empty() {
        String::new()
    } else {
        format!(
            "({})",
            operation
                .variables
                .iter()
                .map(print_variable_definition)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let header = format!(
        "{} {}{variables} {{",
        operation.fetch_plan.0, operation.name
    );
    QueryText(
        std::iter::once(header)
            .chain(print_selection_set(operation.selections, 1))
            .chain(std::iter::once("}".to_owned()))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The lines of `selections` at `depth`, without braces.
fn print_selection_set(selections: &CompletedSelectionMap, depth: usize) -> Vec<String> {
    selections
        .iter()
        .flat_map(|selection| print_selection(selection, depth))
        .collect()
}

/// The lines of one selection at `depth`.
fn print_selection(selection: &CompletedSelection, depth: usize) -> Vec<String> {
    let indent = indent(depth);
    match selection {
        CompletedSelection::Discriminator => format!("{indent}{TYPENAME}").wrap_vec(),
        CompletedSelection::Identity(name) => format!("{indent}{name}").wrap_vec(),
        CompletedSelection::Selectable(selectable) => {
            let head = print_selection_head(selectable.key.reference());
            match selectable.target.reference() {
                CompletedSelectionTarget::Leaf => format!("{indent}{head}").wrap_vec(),
                CompletedSelectionTarget::Composite(inner) => {
                    std::iter::once(format!("{indent}{head} {{"))
                        .chain(print_selection_set(inner, depth + 1))
                        .chain(std::iter::once(format!("{indent}}}")))
                        .collect()
                }
            }
        }
        CompletedSelection::Conditional(conditional) => match conditional.condition {
            Condition::SelectionEquals(entity) => {
                std::iter::once(format!("{indent}... on {entity} {{"))
                    .chain(print_selection_set(
                        conditional.selections.reference(),
                        depth + 1,
                    ))
                    .chain(std::iter::once(format!("{indent}}}")))
                    .collect()
            }
        },
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

/// `<alias>: <name>(<arguments>)`, or `<name>` when the key has no arguments.
fn print_selection_head(key: &SelectionKey) -> String {
    if key.arguments.is_empty() {
        return key.name.to_string();
    }
    format!(
        "{}: {}{}",
        key.normalization_alias(),
        key.name,
        print_arguments(key.arguments.reference())
    )
}

/// `(name: value, ...)`, or the empty string when there are none.
fn print_arguments(arguments: &MergedArguments) -> String {
    if arguments.is_empty() {
        return String::new();
    }
    format!(
        "({})",
        arguments
            .iter()
            .map(|argument| format!(
                "{}: {}",
                argument.name,
                print_value(argument.value.reference())
            ))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn print_value(value: &MergedValue) -> String {
    match value {
        MergedValue::Variable(name) => format!("${name}"),
        MergedValue::String(string) => print_string(string.lookup()),
        MergedValue::Integer(integer) => integer.to_string(),
        MergedValue::Boolean(Boolean::True) => "true".to_owned(),
        MergedValue::Boolean(Boolean::False) => "false".to_owned(),
        MergedValue::Null => "null".to_owned(),
        MergedValue::Object(object) => {
            let entries = object
                .iter()
                .map(|entry| format!("{}: {}", entry.name, print_value(entry.value.reference())))
                .collect::<Vec<_>>();
            if entries.is_empty() {
                "{}".to_owned()
            } else {
                format!("{{ {} }}", entries.join(", "))
            }
        }
    }
}

/// `text` as a GraphQL string value, with `"`, `\`, and U+0000 to U+001F escaped (spec 2.9.4).
fn print_string(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|character| match character {
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            '\u{8}' => "\\b".to_owned(),
            '\u{c}' => "\\f".to_owned(),
            '\u{0}'..='\u{1f}' => format!("\\u{:04X}", u32::from(character)),
            _ => character.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

fn print_variable_definition(variable: &FieldVariable) -> String {
    let default = variable
        .default_value
        .as_ref()
        .map(|value| format!(" = {}", print_value(value)))
        .unwrap_or_default();
    format!(
        "${}: {}{default}",
        variable.name,
        print_variable_type(variable.variable_type.reference())
    )
}

fn print_variable_type(variable_type: &VariableType) -> String {
    match variable_type {
        VariableType::NonNull(inner) => format!("{}!", print_non_null_variable_type(inner)),
        VariableType::Nullable(inner) => print_non_null_variable_type(inner),
    }
}

fn print_non_null_variable_type(variable_type: &NonNullVariableType) -> String {
    match variable_type {
        NonNullVariableType::Named(name) => name.to_string(),
        NonNullVariableType::List(inner) => format!("[{}]", print_variable_type(inner)),
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, QueryText, SelectableName, VariableName};
    use graphql_schema_parser::OperationType;
    use intern::string_key::Intern;
    use isograph_compiler::{
        CompletedSelectionMap, Condition, EntrypointOperation, FieldVariable, IdentitySelection,
        MergedArgument, MergedArguments, MergedCompositeTarget, MergedConditional, MergedObject,
        MergedObjectEntry, MergedSelectableSelection, MergedSelection, MergedSelectionMap,
        MergedSelectionTarget, MergedValue, NonNullVariableType, SelectionKey, VariableType,
    };
    use isograph_parser::Boolean;
    use prelude::Postfix;

    use super::print_operation;
    use crate::{GraphQLFetchPlan, GraphQLNetworkProtocol};

    fn name(s: &str) -> SelectableName {
        s.intern().to()
    }

    fn entity(s: &str) -> EntityName {
        s.intern().to()
    }

    fn id() -> IdentitySelection {
        IdentitySelection::Select(name("id"))
    }

    fn key(s: &str, arguments: Vec<(&str, MergedValue)>) -> SelectionKey {
        SelectionKey {
            name: name(s),
            arguments: MergedArguments::new(
                arguments
                    .into_iter()
                    .map(|(name, value)| MergedArgument {
                        name: name.intern().to(),
                        value,
                    })
                    .collect(),
            )
            .expect("distinct argument names"),
        }
    }

    fn leaf(s: &str) -> MergedSelection {
        leaf_with(s, vec![])
    }

    fn leaf_with(s: &str, arguments: Vec<(&str, MergedValue)>) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: key(s, arguments),
            target: MergedSelectionTarget::Leaf,
        })
    }

    fn composite(
        s: &str,
        arguments: Vec<(&str, MergedValue)>,
        identity: IdentitySelection,
        selections: Vec<MergedSelection>,
    ) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: key(s, arguments),
            target: MergedSelectionTarget::Composite(MergedCompositeTarget {
                identity,
                selections: map(selections),
            }),
        })
    }

    fn conditional(
        on: &str,
        identity: IdentitySelection,
        selections: Vec<MergedSelection>,
    ) -> MergedSelection {
        MergedSelection::Conditional(MergedConditional {
            condition: Condition::SelectionEquals(entity(on)),
            identity,
            selections: map(selections),
        })
    }

    fn map(selections: Vec<MergedSelection>) -> MergedSelectionMap {
        selections
            .into_iter()
            .map(MergedSelectionMap::single)
            .try_fold(MergedSelectionMap::empty(), MergedSelectionMap::merge)
            .expect("the test's selections merge")
    }

    fn top_level(selections: Vec<MergedSelection>) -> CompletedSelectionMap {
        CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(map(selections).reference())
            .expect("the test's top level completes")
    }

    fn print(
        plan: &GraphQLFetchPlan,
        variables: &[FieldVariable],
        selections: &CompletedSelectionMap,
    ) -> QueryText {
        print_operation(&EntrypointOperation {
            name: name("X"),
            fetch_plan: plan,
            variables,
            selections,
        })
    }

    fn lines(text: &QueryText) -> Vec<&str> {
        text.0.split('\n').collect()
    }

    fn root(operation_type: OperationType) -> GraphQLFetchPlan {
        GraphQLFetchPlan(operation_type)
    }

    #[test]
    fn a_root_query_prints_its_selections_with_the_discriminator_and_identity_below_the_top() {
        let text = print_operation(&EntrypointOperation {
            name: name("HomeRoute"),
            fetch_plan: root(OperationType::Query).reference(),
            variables: &[],
            selections: top_level(vec![composite("pets", vec![], id(), vec![leaf("tagline")])])
                .reference(),
        });
        assert_eq!(
            lines(text.reference()),
            vec![
                "query HomeRoute {",
                "  pets {",
                "    __typename",
                "    id",
                "    tagline",
                "  }",
                "}",
            ]
        );
    }

    #[test]
    fn the_header_names_the_root_operation_type_and_the_top_level_has_no_discriminator() {
        let selections = top_level(vec![leaf("count"), composite("pets", vec![], id(), vec![])]);
        for (operation_type, keyword) in [
            (OperationType::Query, "query X {"),
            (OperationType::Mutation, "mutation X {"),
            (OperationType::Subscription, "subscription X {"),
        ] {
            let text = print(
                root(operation_type).reference(),
                &[],
                selections.reference(),
            );
            let lines = lines(text.reference());
            assert_eq!(lines[0], keyword);
            assert!(!lines.contains(&"  __typename"), "{keyword}");
            assert!(!lines.contains(&"  id"), "{keyword}");
        }
    }

    #[test]
    fn variable_definitions_print_their_types_and_defaults() {
        let variable =
            |name: &str, variable_type: VariableType, default_value: Option<MergedValue>| {
                FieldVariable {
                    name: name.intern().to::<VariableName>(),
                    variable_type,
                    default_value,
                }
            };
        let named = |name: &str| NonNullVariableType::Named(entity(name));
        let variables = vec![
            variable(
                "limit",
                VariableType::Nullable(named("Int")),
                MergedValue::Integer(3).wrap_some(),
            ),
            variable(
                "ids",
                VariableType::NonNull(NonNullVariableType::List(
                    VariableType::NonNull(named("ID")).boxed(),
                )),
                None,
            ),
            variable(
                "maybe",
                VariableType::Nullable(NonNullVariableType::List(
                    VariableType::Nullable(named("ID")).boxed(),
                )),
                None,
            ),
            variable(
                "nested",
                VariableType::NonNull(NonNullVariableType::List(
                    VariableType::Nullable(NonNullVariableType::List(
                        VariableType::NonNull(named("Int")).boxed(),
                    ))
                    .boxed(),
                )),
                None,
            ),
            variable(
                "flag",
                VariableType::NonNull(named("Boolean")),
                MergedValue::Boolean(Boolean::False).wrap_some(),
            ),
        ];
        let text = print(
            root(OperationType::Query).reference(),
            variables.as_slice(),
            top_level(vec![leaf("a")]).reference(),
        );
        assert_eq!(
            lines(text.reference())[0],
            "query X($limit: Int = 3, $ids: [ID!]!, $maybe: [ID], $nested: [[Int!]]!, $flag: Boolean! = false) {"
        );
    }

    #[test]
    fn the_identity_follows_the_discriminator_only_where_the_entity_has_one() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![
                composite("with", vec![], id(), vec![leaf("a")]),
                composite(
                    "without",
                    vec![],
                    IdentitySelection::Omit,
                    vec![leaf("a"), leaf("id")],
                ),
            ])
            .reference(),
        );
        assert_eq!(
            lines(text.reference()),
            vec![
                "query X {",
                "  with {",
                "    __typename",
                "    id",
                "    a",
                "  }",
                "  without {",
                "    __typename",
                "    a",
                "    id",
                "  }",
                "}",
            ]
        );
    }

    #[test]
    fn a_user_selection_of_id_prints_once() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![composite(
                "pets",
                vec![],
                id(),
                vec![leaf("id"), leaf("age")],
            )])
            .reference(),
        );
        assert_eq!(
            lines(text.reference())
                .iter()
                .filter(|line| line.trim() == "id")
                .count(),
            1
        );
    }

    #[test]
    fn a_refinement_prints_after_the_selections_with_the_discriminator_first_inside() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![composite(
                "newsfeed",
                vec![("limit", MergedValue::Integer(2))],
                IdentitySelection::Omit,
                vec![
                    conditional("BlogItem", id(), vec![leaf("title")]),
                    leaf("zzz"),
                    conditional("AdItem", id(), vec![leaf("message")]),
                ],
            )])
            .reference(),
        );
        assert_eq!(
            lines(text.reference()),
            vec![
                "query X {",
                "  newsfeed____limit___l_2: newsfeed(limit: 2) {",
                "    __typename",
                "    zzz",
                "    ... on AdItem {",
                "      __typename",
                "      id",
                "      message",
                "    }",
                "    ... on BlogItem {",
                "      __typename",
                "      id",
                "      title",
                "    }",
                "  }",
                "}",
            ]
        );
    }

    #[test]
    fn a_nested_refinement_prints_nested() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![composite(
                "u",
                vec![],
                IdentitySelection::Omit,
                vec![conditional(
                    "A",
                    IdentitySelection::Omit,
                    vec![conditional("B", IdentitySelection::Omit, vec![leaf("x")])],
                )],
            )])
            .reference(),
        );
        assert_eq!(
            lines(text.reference())[3..9].to_vec(),
            vec![
                "    ... on A {",
                "      __typename",
                "      ... on B {",
                "        __typename",
                "        x",
                "      }",
            ]
        );
    }

    #[test]
    fn an_empty_composite_without_identity_prints_only_the_discriminator() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![composite(
                "stats",
                vec![],
                IdentitySelection::Omit,
                vec![],
            )])
            .reference(),
        );
        assert_eq!(
            lines(text.reference()),
            vec!["query X {", "  stats {", "    __typename", "  }", "}"]
        );
    }

    #[test]
    fn a_selection_with_arguments_is_aliased_and_one_without_is_its_bare_name() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![
                leaf("plain"),
                leaf_with(
                    "checkins",
                    vec![
                        ("limit", MergedValue::Integer(5)),
                        ("skip", MergedValue::Integer(0)),
                    ],
                ),
                composite(
                    "pet",
                    vec![("id", MergedValue::Variable("id".intern().to()))],
                    id(),
                    vec![leaf_with(
                        "f",
                        vec![("b", MergedValue::Boolean(Boolean::True))],
                    )],
                ),
            ])
            .reference(),
        );
        let lines = lines(text.reference());
        assert!(lines.contains(&"  plain"));
        assert!(
            lines.contains(&"  checkins____limit___l_5____skip___l_0: checkins(limit: 5, skip: 0)")
        );
        assert!(lines.contains(&"  pet____id___v_id: pet(id: $id) {"));
        assert!(lines.contains(&"    f____b___l_true: f(b: true)"));
        for line in &lines[1..lines.len() - 1] {
            let trimmed = line.trim().trim_end_matches(" {");
            if trimmed == "}" || trimmed.starts_with("... on") {
                continue;
            }
            match trimmed.split_once(": ") {
                Some((alias, rest)) => {
                    let (name, arguments) = rest
                        .split_once('(')
                        .expect("an aliased selection has arguments");
                    assert!(alias.starts_with(format!("{name}____").as_str()), "{line}");
                    assert!(arguments.ends_with(')'), "{line}");
                }
                None => assert!(
                    trimmed
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    "a selection without arguments is its bare name: {line}"
                ),
            }
        }
    }

    #[test]
    fn string_values_are_escaped() {
        let text = print(
            root(OperationType::Query).reference(),
            &[],
            top_level(vec![leaf_with(
                "f",
                vec![(
                    "s",
                    MergedValue::String("a\"b\\c\n\r\t\u{8}\u{c}\u{1}é".intern().to()),
                )],
            )])
            .reference(),
        );
        assert!(
            lines(text.reference())[1].ends_with(r#"f(s: "a\"b\\c\n\r\t\b\f\u0001é")"#),
            "{}",
            text.0
        );
    }

    #[test]
    fn values_print_as_graphql() {
        let entry = |name: &str, value: MergedValue| MergedObjectEntry {
            name: name.intern().to(),
            value,
        };
        let object =
            |entries| MergedValue::Object(MergedObject::new(entries).expect("distinct keys"));
        let text = print(
            root(OperationType::Mutation).reference(),
            &[],
            top_level(vec![leaf_with(
                "f",
                vec![
                    (
                        "input",
                        object(vec![
                            entry("tagline", MergedValue::Variable("tagline".intern().to())),
                            entry("id", MergedValue::Variable("id".intern().to())),
                            entry("inner", object(vec![entry("n", MergedValue::Integer(-7))])),
                        ]),
                    ),
                    ("empty", object(vec![])),
                    ("none", MergedValue::Null),
                    ("no", MergedValue::Boolean(Boolean::False)),
                ],
            )])
            .reference(),
        );
        assert!(
            lines(text.reference())[1].ends_with(
                "f(empty: {}, input: { id: $id, inner: { n: -7 }, tagline: $tagline }, no: false, none: null)"
            ),
            "{}",
            text.0
        );
    }

    #[test]
    fn no_line_ends_in_whitespace_and_the_text_does_not_end_in_a_newline() {
        let text = print(
            root(OperationType::Query).reference(),
            &[FieldVariable {
                name: "id".intern().to(),
                variable_type: VariableType::NonNull(NonNullVariableType::Named(entity("ID"))),
                default_value: None,
            }],
            top_level(vec![composite(
                "pet",
                vec![("id", MergedValue::Variable("id".intern().to()))],
                id(),
                vec![composite(
                    "stats",
                    vec![],
                    IdentitySelection::Omit,
                    vec![leaf("w")],
                )],
            )])
            .reference(),
        );
        assert!(!text.0.ends_with('\n'));
        assert!(
            lines(text.reference())
                .iter()
                .all(|line| line.trim_end() == *line)
        );
    }
}
