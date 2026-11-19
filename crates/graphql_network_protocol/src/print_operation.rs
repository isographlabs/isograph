use common_lang_types::QueryText;
use intern::Lookup;
use isograph_compiler::{
    CompletedSelection, CompletedSelectionMap, CompletedSelectionTarget, Condition,
    EntrypointOperation, FieldVariable, MergedArguments, MergedValue, NonNullVariableType,
    ResponseKey, SelectionKey, VariableType,
};
use isograph_config::QueryTextFormat;
use isograph_parser::Boolean;
use prelude::Postfix;

use crate::GraphQLNetworkProtocol;
use crate::protocol_data::TYPENAME;

/// One piece of an operation's text before `lay_out` writes it in a `QueryTextFormat`.
enum PrintItem {
    /// Text both formats write as is: one or more GraphQL tokens with no whitespace or comma
    /// between them (`pets`, `$id`, `[ID!]!`, `"a b"`, `{`).
    Text(String),
    /// A space `Expanded` writes and `Collapsed` does not.
    Space,
    /// A comma between two arguments, object entries, list items, or variable definitions.
    /// GraphQL ignores it (spec 2.1.8); `Expanded` writes it and `Collapsed` does not.
    Comma,
    /// The start of a line at this depth. `Expanded` writes a newline and two spaces per depth;
    /// `Collapsed` writes nothing.
    LineBreak(usize),
}

fn text(text: &str) -> PrintItem {
    PrintItem::Text(text.to_owned())
}

/// The text of `operation` in `format`.
pub(crate) fn print_operation(
    format: QueryTextFormat,
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> QueryText {
    QueryText(lay_out(format, operation_items(operation).as_slice()))
}

/// The header, `print_selection_set(selections, 1)`, a `LineBreak(0)`, and `}`.
fn operation_items(operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>) -> Vec<PrintItem> {
    let variables = if operation.variables.is_empty() {
        Vec::new()
    } else {
        std::iter::once(text("("))
            .chain(comma_separated(
                operation.variables.iter().map(print_variable_definition),
            ))
            .chain(std::iter::once(text(")")))
            .collect()
    };
    [
        PrintItem::Text(operation.fetch_plan.operation_type.to_string()),
        PrintItem::Space,
        PrintItem::Text(operation.name.to_string()),
    ]
    .into_iter()
    .chain(variables)
    .chain(print_selection_block(operation.selections, 0))
    .collect()
}

/// `items` as text. `Expanded` writes every item as its doc comment says. `Collapsed` writes
/// only the `Text` items, each preceded by `collapsed_separator` of the previous `Text`'s
/// last character and its first.
fn lay_out(format: QueryTextFormat, items: &[PrintItem]) -> String {
    match format {
        QueryTextFormat::Expanded => items
            .iter()
            .map(|item| match item {
                PrintItem::Text(text) => text.clone(),
                PrintItem::Space => " ".to_owned(),
                PrintItem::Comma => ",".to_owned(),
                PrintItem::LineBreak(depth) => format!("\n{}", indent(depth.dereference())),
            })
            .collect(),
        QueryTextFormat::Collapsed => items
            .iter()
            .filter_map(|item| match item {
                PrintItem::Text(text) => text.as_str().wrap_some(),
                PrintItem::Space | PrintItem::Comma | PrintItem::LineBreak(_) => None,
            })
            .fold(String::new(), |mut laid_out, text| {
                if let (Some(previous), Some(next)) =
                    (laid_out.chars().next_back(), text.chars().next())
                {
                    laid_out.push_str(collapsed_separator(previous, next));
                }
                laid_out.push_str(text);
                laid_out
            }),
    }
}

/// `" "` when `previous` and `next` are both name characters (`_`, `0`-`9`, `A`-`Z`, `a`-`z`;
/// spec 2.1.9), because the two tokens would otherwise lex as one; otherwise `""`.
fn collapsed_separator(previous: char, next: char) -> &'static str {
    match (previous, next) {
        ('_' | '0'..='9' | 'A'..='Z' | 'a'..='z', '_' | '0'..='9' | 'A'..='Z' | 'a'..='z') => " ",
        _ => "",
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

/// Every selection of `selections` at `depth`, without braces.
fn print_selection_set(selections: &CompletedSelectionMap, depth: usize) -> Vec<PrintItem> {
    selections
        .iter()
        .flat_map(|selection| print_selection(selection, depth))
        .collect()
}

/// ` {`, `selections` at `depth + 1`, and `}` on its own line at `depth`.
fn print_selection_block(selections: &CompletedSelectionMap, depth: usize) -> Vec<PrintItem> {
    [PrintItem::Space, text("{")]
        .into_iter()
        .chain(print_selection_set(selections, depth + 1))
        .chain([PrintItem::LineBreak(depth), text("}")])
        .collect()
}

/// `LineBreak(depth)`, then the selection.
fn print_selection(selection: &CompletedSelection, depth: usize) -> Vec<PrintItem> {
    let line = PrintItem::LineBreak(depth);
    match selection {
        CompletedSelection::Discriminator => vec![line, text(TYPENAME)],
        CompletedSelection::Identity(name) => vec![line, PrintItem::Text(name.to_string())],
        CompletedSelection::Selectable(selectable) => {
            let head = print_selection_head(selectable.key.reference());
            match selectable.target.reference() {
                CompletedSelectionTarget::Leaf => std::iter::once(line).chain(head).collect(),
                CompletedSelectionTarget::Composite(target) => std::iter::once(line)
                    .chain(head)
                    .chain(print_selection_block(target.selections.reference(), depth))
                    .collect(),
            }
        }
        CompletedSelection::Conditional(conditional) => match conditional.condition {
            Condition::SelectionEquals(entity) => [
                line,
                text("..."),
                PrintItem::Space,
                text("on"),
                PrintItem::Space,
                PrintItem::Text(entity.to_string()),
            ]
            .into_iter()
            .chain(print_selection_block(
                conditional.selections.reference(),
                depth,
            ))
            .collect(),
        },
    }
}

/// `<alias>: <name>(<arguments>)`, or `<name>` when the key has no arguments.
fn print_selection_head(key: &SelectionKey) -> Vec<PrintItem> {
    match key.response_key() {
        ResponseKey::SelectableName => PrintItem::Text(key.name.to_string()).wrap_vec(),
        ResponseKey::Alias(alias) => [
            PrintItem::Text(alias.to_string()),
            text(":"),
            PrintItem::Space,
            PrintItem::Text(key.name.to_string()),
        ]
        .into_iter()
        .chain(print_arguments(key.arguments.reference()))
        .collect(),
    }
}

/// `(name: value, ...)`, or no items when there are none.
fn print_arguments(arguments: &MergedArguments) -> Vec<PrintItem> {
    if arguments.is_empty() {
        return Vec::new();
    }
    std::iter::once(text("("))
        .chain(comma_separated(arguments.iter().map(|argument| {
            print_entry(argument.name.to_string(), argument.value.reference())
        })))
        .chain(std::iter::once(text(")")))
        .collect()
}

/// `name: value`, an argument or an object entry.
fn print_entry(name: String, value: &MergedValue) -> Vec<PrintItem> {
    [PrintItem::Text(name), text(":"), PrintItem::Space]
        .into_iter()
        .chain(print_value(value))
        .collect()
}

fn print_value(value: &MergedValue) -> Vec<PrintItem> {
    match value {
        MergedValue::Variable(name) => PrintItem::Text(format!("${name}")).wrap_vec(),
        MergedValue::String(string) => PrintItem::Text(print_string(string.lookup())).wrap_vec(),
        MergedValue::Integer(integer) => PrintItem::Text(integer.to_string()).wrap_vec(),
        MergedValue::Boolean(Boolean::True) => text("true").wrap_vec(),
        MergedValue::Boolean(Boolean::False) => text("false").wrap_vec(),
        MergedValue::Null => text("null").wrap_vec(),
        MergedValue::Object(object) => {
            if object.iter().next().is_none() {
                return text("{}").wrap_vec();
            }
            [text("{"), PrintItem::Space]
                .into_iter()
                .chain(comma_separated(object.iter().map(|entry| {
                    print_entry(entry.name.to_string(), entry.value.reference())
                })))
                .chain([PrintItem::Space, text("}")])
                .collect()
        }
        MergedValue::List(items) => std::iter::once(text("["))
            .chain(comma_separated(items.iter().map(print_value)))
            .chain(std::iter::once(text("]")))
            .collect(),
    }
}

/// `groups` with `Comma` and `Space` between consecutive groups.
fn comma_separated(groups: impl Iterator<Item = Vec<PrintItem>>) -> Vec<PrintItem> {
    groups
        .enumerate()
        .flat_map(|(index, group)| {
            match index {
                0 => Vec::new(),
                _ => vec![PrintItem::Comma, PrintItem::Space],
            }
            .into_iter()
            .chain(group)
        })
        .collect()
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

fn print_variable_definition(variable: &FieldVariable) -> Vec<PrintItem> {
    let default: Vec<PrintItem> = variable
        .default_value
        .as_ref()
        .map(|value| {
            [PrintItem::Space, text("="), PrintItem::Space]
                .into_iter()
                .chain(print_value(value))
                .collect()
        })
        .unwrap_or_default();
    [
        PrintItem::Text(format!("${}", variable.name)),
        text(":"),
        PrintItem::Space,
        PrintItem::Text(print_variable_type(variable.variable_type.reference())),
    ]
    .into_iter()
    .chain(default)
    .collect()
}

pub(crate) fn print_variable_type(variable_type: &VariableType) -> String {
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
/// One line of a query's selection sets: its depth and what it selects.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SelectionLine {
    /// `alias: name...`
    Selection(SelectionLineData),
    /// `... on entity {`
    Conditional(ConditionalLine),
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SelectionLineData {
    depth: usize,
    /// The alias the line prints before `:`, or the selectable name when it prints none.
    response_key: String,
    selectable_name: String,
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConditionalLine {
    depth: usize,
    entity: String,
}

#[cfg(test)]
/// The selection lines of `text`, skipping the header and `}` lines.
pub(crate) fn text_lines(text: &QueryText) -> Vec<SelectionLine> {
    text.0
        .split('\n')
        .skip(1)
        .filter(|line| line.trim() != "}")
        .map(|line| {
            let depth = (line.len() - line.trim_start().len()) / 2;
            let content = line.trim().trim_end_matches(" {");
            match content.strip_prefix("... on ") {
                Some(entity) => SelectionLine::Conditional(ConditionalLine {
                    depth,
                    entity: entity.to_owned(),
                }),
                None => {
                    let (response_key, rest) =
                        content.split_once(": ").unwrap_or((content, content));
                    SelectionLine::Selection(SelectionLineData {
                        depth,
                        response_key: response_key.to_owned(),
                        selectable_name: rest.split('(').next().unwrap_or(rest).to_owned(),
                    })
                }
            }
        })
        .collect()
}

#[cfg(test)]
/// The AST's nodes in pre-order, a `Conditional` as the line its `... on` prints.
pub(crate) fn ast_lines(ast: &isograph_compiler::NormalizationAst) -> Vec<SelectionLine> {
    selection_set_lines(ast.selections.reference(), 1)
}

#[cfg(test)]
fn selection_set_lines(
    selections: &isograph_compiler::NormalizationSelectionSet,
    depth: usize,
) -> Vec<SelectionLine> {
    use isograph_compiler::{NormalizationNode, NormalizationTarget};
    selections
        .0
        .iter()
        .flat_map(|node| match node {
            NormalizationNode::Selection(selection) => {
                std::iter::once(SelectionLine::Selection(SelectionLineData {
                    depth,
                    response_key: match selection.response_key.reference() {
                        ResponseKey::SelectableName => selection.selectable_name.to_string(),
                        ResponseKey::Alias(alias) => alias.to_string(),
                    },
                    selectable_name: selection.selectable_name.to_string(),
                }))
                .chain(match selection.target.reference() {
                    NormalizationTarget::Leaf => Vec::new(),
                    NormalizationTarget::Composite(target) => {
                        selection_set_lines(target.selections.reference(), depth + 1)
                    }
                })
                .collect::<Vec<_>>()
            }
            NormalizationNode::Conditional(conditional) => {
                let Condition::SelectionEquals(entity) = conditional.condition;
                std::iter::once(SelectionLine::Conditional(ConditionalLine {
                    depth,
                    entity: entity.to_string(),
                }))
                .chain(selection_set_lines(
                    conditional.selections.reference(),
                    depth + 1,
                ))
                .collect()
            }
        })
        .collect()
}

#[cfg(test)]
/// `lines` without the discriminator's lines, which the AST leaves implicit.
pub(crate) fn without_discriminator(
    lines: Vec<SelectionLine>,
    response_key: &str,
) -> Vec<SelectionLine> {
    lines
        .into_iter()
        .filter(|line| match line {
            SelectionLine::Selection(selection) => {
                !(selection.response_key == response_key
                    && selection.selectable_name == response_key)
            }
            SelectionLine::Conditional(_) => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, QueryText, SelectableName, VariableName};
    use graphql_schema_parser::OperationType;
    use intern::string_key::Intern;
    use std::collections::BTreeSet;

    use isograph_compiler::{
        CompletedIdentity, CompletedSelectionMap, CompositeShape, Concreteness, Condition,
        EntrypointOperation, EntrypointWrapper, FieldVariable, FixedRecordId, IdentitySelection,
        IntegerOutOfRange, MergedArgument, MergedArguments, MergedCompositeTarget,
        MergedConditional, MergedObject, MergedObjectEntry, MergedSelectableSelection,
        MergedSelection, MergedSelectionMap, MergedSelectionTarget, MergedValue,
        NonNullVariableType, NormalizationAst, NormalizationCompositeTarget, NormalizationNode,
        NormalizationObjectEntry, NormalizationRootRecord, NormalizationSelectionSet,
        NormalizationTarget, NormalizationValue, RecordEntity, ResponseKey, RootEntities,
        SelectionKey, VariableType, normalization_ast,
    };
    use isograph_parser::Boolean;
    use prelude::Postfix;

    use isograph_config::QueryTextFormat;

    use super::{SelectionLine, ast_lines, print_operation, text_lines, without_discriminator};
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
                entity: entity(s),
                shape: CompositeShape {
                    concreteness: Concreteness::Concrete,
                    identity,
                },
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
        CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(
            map(selections).reference(),
            root_entities_of(OperationType::Query).reference(),
        )
        .expect("the test's top level completes")
    }

    fn print(
        plan: &GraphQLFetchPlan,
        variables: &[FieldVariable],
        selections: &CompletedSelectionMap,
    ) -> QueryText {
        print_operation(
            QueryTextFormat::Expanded,
            &EntrypointOperation {
                name: name("X"),
                fetch_plan: plan,
                variables,
                selections,
            },
        )
    }

    fn lines(text: &QueryText) -> Vec<&str> {
        text.0.split('\n').collect()
    }

    /// The root type of `operation_type` as the only root entity.
    fn root_entities_of(operation_type: OperationType) -> RootEntities {
        RootEntities {
            operation: entity(root_type_name(operation_type)),
            others: BTreeSet::new(),
        }
    }

    fn root(operation_type: OperationType) -> GraphQLFetchPlan {
        GraphQLFetchPlan {
            operation_type,
            wrapper: EntrypointWrapper::Unwrapped(root_entities_of(operation_type)),
        }
    }

    #[test]
    fn a_root_query_prints_its_selections_with_the_discriminator_and_identity_below_the_top() {
        let text = print_operation(
            QueryTextFormat::Expanded,
            &EntrypointOperation {
                name: name("HomeRoute"),
                fetch_plan: root(OperationType::Query).reference(),
                variables: &[],
                selections: top_level(vec![composite("pets", vec![], id(), vec![leaf("tagline")])])
                    .reference(),
            },
        );
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

    /// Variables of every type shape, two with defaults.
    fn every_kind_of_variable() -> Vec<FieldVariable> {
        let variable =
            |name: &str, variable_type: VariableType, default_value: Option<MergedValue>| {
                FieldVariable {
                    name: name.intern().to::<VariableName>(),
                    variable_type,
                    default_value,
                }
            };
        let named = |name: &str| NonNullVariableType::Named(entity(name));
        vec![
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
        ]
    }

    #[test]
    fn variable_definitions_print_their_types_and_defaults() {
        let variables = every_kind_of_variable();
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

    /// The conventional root type of `operation_type`.
    fn root_type_name(operation_type: OperationType) -> &'static str {
        match operation_type {
            OperationType::Query => "Query",
            OperationType::Mutation => "Mutation",
            OperationType::Subscription => "Subscription",
        }
    }

    /// A selection of `s` whose target is `target` with `concreteness` and `identity`.
    fn composite_of(
        s: &str,
        arguments: Vec<(&str, MergedValue)>,
        target: &str,
        concreteness: Concreteness,
        identity: IdentitySelection,
        selections: Vec<MergedSelection>,
    ) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: key(s, arguments),
            target: MergedSelectionTarget::Composite(MergedCompositeTarget {
                entity: entity(target),
                shape: CompositeShape {
                    concreteness,
                    identity,
                },
                selections: map(selections),
            }),
        })
    }

    /// The query text and the normalization AST of an operation of `operation_type` whose
    /// top-level selections are `selections`, with `Query` and `Mutation` as root entities.
    fn artifact(
        operation_type: OperationType,
        selections: Vec<MergedSelection>,
    ) -> (QueryText, NormalizationAst) {
        let operation = entity(root_type_name(operation_type));
        let roots = RootEntities {
            operation,
            others: [entity("Query"), entity("Mutation")]
                .into_iter()
                .filter(|other| *other != operation)
                .collect(),
        };
        let completed = CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(
            map(selections).reference(),
            roots.reference(),
        )
        .expect("the test's top level completes");
        let ast = normalization_ast(completed.reference(), name("__typename"), operation)
            .expect("the test's integers fit");
        let plan = GraphQLFetchPlan {
            operation_type,
            wrapper: EntrypointWrapper::Unwrapped(roots),
        };
        (print(plan.reference(), &[], completed.reference()), ast)
    }

    fn assert_lines_match(text: &QueryText, ast: &NormalizationAst) {
        assert_eq!(
            ast_lines(ast),
            without_discriminator(text_lines(text), "__typename"),
            "{}",
            text.0
        );
    }

    fn union_feed() -> MergedSelection {
        composite_of(
            "newsfeed",
            vec![("limit", MergedValue::Integer(2))],
            "NewsfeedItem",
            Concreteness::Abstract,
            IdentitySelection::Omit,
            vec![
                conditional("BlogItem", id(), vec![leaf("title")]),
                leaf("cursor"),
            ],
        )
    }

    fn every_kind_of_argument() -> MergedSelection {
        let entry = |name: &str, value: MergedValue| MergedObjectEntry {
            name: name.intern().to(),
            value,
        };
        leaf_with(
            "f",
            vec![
                ("b", MergedValue::Boolean(Boolean::True)),
                ("i", MergedValue::Integer(-5)),
                ("n", MergedValue::Null),
                (
                    "o",
                    MergedValue::Object(
                        MergedObject::new(vec![
                            entry("v", MergedValue::Variable("x".intern().to())),
                            entry("s", MergedValue::String("a b".intern().to())),
                        ])
                        .expect("distinct keys"),
                    ),
                ),
                ("s", MergedValue::String("\"q\"".intern().to())),
                ("v", MergedValue::Variable("x".intern().to())),
            ],
        )
    }

    fn cases() -> Vec<(OperationType, Vec<MergedSelection>)> {
        vec![
            (
                OperationType::Query,
                vec![composite_of(
                    "pets",
                    vec![],
                    "Pet",
                    Concreteness::Concrete,
                    id(),
                    vec![leaf("id"), leaf("age")],
                )],
            ),
            (OperationType::Query, vec![union_feed()]),
            (
                OperationType::Query,
                vec![composite_of(
                    "checkin",
                    vec![],
                    "ICheckin",
                    Concreteness::Abstract,
                    id(),
                    vec![leaf("time")],
                )],
            ),
            (
                OperationType::Query,
                vec![composite_of(
                    "pet",
                    vec![("id", MergedValue::Variable("id".intern().to()))],
                    "Pet",
                    Concreteness::Concrete,
                    id(),
                    vec![composite_of(
                        "stats",
                        vec![],
                        "PetStats",
                        Concreteness::Concrete,
                        IdentitySelection::Omit,
                        vec![leaf("weight")],
                    )],
                )],
            ),
            (
                OperationType::Query,
                vec![composite_of(
                    "query",
                    vec![],
                    "Query",
                    Concreteness::Concrete,
                    id(),
                    vec![leaf("tags")],
                )],
            ),
            (OperationType::Mutation, vec![every_kind_of_argument()]),
        ]
    }

    /// The composite target of the top-level selection named `s`.
    fn top_target<'a>(ast: &'a NormalizationAst, s: &str) -> &'a NormalizationCompositeTarget {
        ast.selections
            .0
            .iter()
            .find_map(|node| match node {
                NormalizationNode::Selection(selection) if selection.selectable_name == name(s) => {
                    match selection.target.reference() {
                        NormalizationTarget::Composite(target) => target.wrap_some(),
                        NormalizationTarget::Leaf => None,
                    }
                }
                NormalizationNode::Selection(_) | NormalizationNode::Conditional(_) => None,
            })
            .expect("the AST has this composite selection")
    }

    #[test]
    fn the_ast_lines_are_the_query_lines_without_the_discriminator() {
        for (operation_type, selections) in cases() {
            let (text, ast) = artifact(operation_type, selections);
            assert_lines_match(text.reference(), ast.reference());
        }
    }

    #[test]
    fn every_composite_block_starts_with_the_discriminator_and_no_node_is_the_discriminator() {
        for (operation_type, selections) in cases() {
            let (text, ast) = artifact(operation_type, selections);
            let lines: Vec<&str> = text.0.split('\n').collect();
            for (index, line) in lines.iter().enumerate() {
                if line.ends_with('{') && index > 0 {
                    assert_eq!(lines[index + 1].trim(), "__typename", "{}", text.0);
                }
            }
            assert!(
                ast_lines(ast.reference()).iter().all(|line| match line {
                    SelectionLine::Selection(selection) =>
                        selection.selectable_name != "__typename",
                    SelectionLine::Conditional(_) => true,
                }),
                "{}",
                text.0
            );
            assert_eq!(ast.discriminator_response_key, name("__typename"));
        }
    }

    #[test]
    fn every_response_key_is_the_key_s_response_key() {
        let (_, ast) = artifact(OperationType::Mutation, vec![every_kind_of_argument()]);
        let NormalizationNode::Selection(f) = &ast.selections.0[0] else {
            unreachable!("the top-level selection is f")
        };
        assert!(matches!(f.response_key, ResponseKey::Alias(_)));
        let (_, ast) = artifact(OperationType::Query, cases().remove(0).1);
        let pets = top_target(ast.reference(), "pets");
        assert!(pets.selections.0.iter().all(|node| matches!(
            node,
            NormalizationNode::Selection(selection) if selection.response_key == ResponseKey::SelectableName
        )));
    }

    #[test]
    fn targets_carry_their_record_entity_and_identity() {
        let id_identity = CompletedIdentity::Select(name("id"));
        let (_, pets) = artifact(OperationType::Query, cases().remove(0).1);
        let pets_target = top_target(pets.reference(), "pets");
        assert_eq!(pets_target.entity, RecordEntity::Known(entity("Pet")));
        assert_eq!(pets_target.identity, id_identity);
        assert!(pets_target.selections.0.iter().any(|node| matches!(
            node,
            NormalizationNode::Selection(selection)
                if selection.selectable_name == name("id")
                    && selection.response_key == ResponseKey::SelectableName
                    && selection.target == NormalizationTarget::Leaf
        )));

        let (_, feed) = artifact(OperationType::Query, vec![union_feed()]);
        let newsfeed = top_target(feed.reference(), "newsfeed");
        assert_eq!(newsfeed.entity, RecordEntity::Discriminator);
        assert_eq!(newsfeed.identity, CompletedIdentity::Omit);
        let conditional = newsfeed
            .selections
            .0
            .iter()
            .find_map(|node| match node {
                NormalizationNode::Conditional(conditional) => conditional.wrap_some(),
                NormalizationNode::Selection(_) => None,
            })
            .expect("newsfeed has the BlogItem conditional");
        assert_eq!(
            conditional.condition,
            Condition::SelectionEquals(entity("BlogItem"))
        );
        assert_eq!(conditional.identity, id_identity);
        assert!(matches!(
            newsfeed.selections.0.last(),
            Some(NormalizationNode::Conditional(_))
        ));

        let (_, checkin) = artifact(OperationType::Query, cases().remove(2).1);
        let checkin = top_target(checkin.reference(), "checkin");
        assert_eq!(checkin.entity, RecordEntity::Discriminator);
        assert_eq!(checkin.identity, id_identity);
    }

    #[test]
    fn the_root_record_is_the_operation_s_root_and_a_nested_root_target_is_fixed() {
        let root = |entity_name: &str| NormalizationRootRecord {
            entity_name: entity(entity_name),
            id: FixedRecordId::root(),
        };
        let nested_query = || {
            vec![composite_of(
                "query",
                vec![],
                "Query",
                Concreteness::Concrete,
                id(),
                vec![leaf("tags")],
            )]
        };
        let (text, ast) = artifact(OperationType::Query, nested_query());
        assert_eq!(ast.root, root("Query"));
        let query = top_target(ast.reference(), "query");
        assert_eq!(query.entity, RecordEntity::Known(entity("Query")));
        assert_eq!(
            query.identity,
            CompletedIdentity::Fixed(FixedRecordId::root())
        );
        assert!(
            !text.0.split('\n').any(|line| line.trim() == "id"),
            "{}",
            text.0
        );

        let (_, ast) = artifact(OperationType::Mutation, nested_query());
        assert_eq!(ast.root, root("Mutation"));
        assert_eq!(
            top_target(ast.reference(), "query").identity,
            CompletedIdentity::Fixed(FixedRecordId::root())
        );
    }

    #[test]
    fn an_empty_composite_without_identity_is_an_empty_target_and_one_discriminator_line() {
        let (text, ast) = artifact(
            OperationType::Query,
            vec![composite_of(
                "stats",
                vec![],
                "PetStats",
                Concreteness::Concrete,
                IdentitySelection::Omit,
                vec![],
            )],
        );
        assert_eq!(
            top_target(ast.reference(), "stats"),
            &NormalizationCompositeTarget {
                entity: RecordEntity::Known(entity("PetStats")),
                identity: CompletedIdentity::Omit,
                selections: NormalizationSelectionSet(Vec::new()),
            }
        );
        assert_eq!(
            lines(text.reference()),
            vec!["query X {", "  stats {", "    __typename", "  }", "}"]
        );
    }

    #[test]
    fn arguments_narrow_every_value_kind_in_key_order() {
        let (_, ast) = artifact(OperationType::Mutation, vec![every_kind_of_argument()]);
        let NormalizationNode::Selection(f) = &ast.selections.0[0] else {
            unreachable!("the top-level selection is f")
        };
        let object_entry = |name: &str, value: NormalizationValue| NormalizationObjectEntry {
            name: name.intern().to(),
            value,
        };
        assert_eq!(
            f.arguments
                .0
                .iter()
                .map(|argument| (argument.name.to_string(), argument.value.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("b".to_owned(), NormalizationValue::Boolean(Boolean::True)),
                ("i".to_owned(), NormalizationValue::Integer(-5)),
                ("n".to_owned(), NormalizationValue::Null),
                (
                    "o".to_owned(),
                    NormalizationValue::Object(vec![
                        object_entry("s", NormalizationValue::String("a b".intern().to())),
                        object_entry("v", NormalizationValue::Variable("x".intern().to())),
                    ])
                ),
                (
                    "s".to_owned(),
                    NormalizationValue::String("\"q\"".intern().to())
                ),
                (
                    "v".to_owned(),
                    NormalizationValue::Variable("x".intern().to())
                ),
            ]
        );
    }

    #[test]
    fn an_integer_outside_i32_is_out_of_range() {
        let completed = CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(
            map(vec![leaf_with(
                "f",
                vec![("n", MergedValue::Integer(4_294_967_296))],
            )])
            .reference(),
            root_entities_of(OperationType::Query).reference(),
        )
        .expect("the map has a selection");
        assert_eq!(
            normalization_ast(completed.reference(), name("__typename"), entity("Query")),
            IntegerOutOfRange(4_294_967_296).wrap_err()
        );
        let in_range = CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(
            map(vec![leaf_with(
                "f",
                vec![("n", MergedValue::Integer(i64::from(i32::MIN)))],
            )])
            .reference(),
            root_entities_of(OperationType::Query).reference(),
        )
        .expect("the map has a selection");
        assert!(
            normalization_ast(in_range.reference(), name("__typename"), entity("Query")).is_ok()
        );
    }

    /// The plan of an operation of `operation_type`, with `Query` and `Mutation` as root
    /// entities.
    fn plan(operation_type: OperationType) -> GraphQLFetchPlan {
        GraphQLFetchPlan {
            operation_type,
            wrapper: EntrypointWrapper::Unwrapped(format_roots(operation_type)),
        }
    }

    /// The root entities of an operation of `operation_type`, with `Query` and `Mutation` as
    /// root entities.
    fn format_roots(operation_type: OperationType) -> RootEntities {
        let operation = entity(root_type_name(operation_type));
        RootEntities {
            operation,
            others: [entity("Query"), entity("Mutation")]
                .into_iter()
                .filter(|other| *other != operation)
                .collect(),
        }
    }

    /// The operations the printer tests build, the variable definitions, and a list and an
    /// empty object argument.
    fn format_fixtures() -> Vec<(GraphQLFetchPlan, Vec<FieldVariable>, CompletedSelectionMap)> {
        let lists = leaf_with(
            "g",
            vec![
                (
                    "e",
                    MergedValue::Object(MergedObject::new(vec![]).expect("no keys")),
                ),
                (
                    "l",
                    MergedValue::List(vec![
                        MergedValue::Integer(1),
                        MergedValue::Integer(5),
                        MergedValue::Variable("x".intern().to()),
                        MergedValue::Boolean(Boolean::True),
                    ]),
                ),
            ],
        );
        let complete = |operation_type: OperationType, selections: Vec<MergedSelection>| {
            let plan = plan(operation_type);
            let completed = CompletedSelectionMap::top_level::<GraphQLNetworkProtocol>(
                map(selections).reference(),
                format_roots(operation_type).reference(),
            )
            .expect("the test's top level completes");
            (plan, completed)
        };
        cases()
            .into_iter()
            .chain(std::iter::once((OperationType::Query, lists.wrap_vec())))
            .map(|(operation_type, selections)| {
                let (plan, completed) = complete(operation_type, selections);
                (plan, Vec::new(), completed)
            })
            .chain(std::iter::once({
                let (plan, completed) = complete(OperationType::Query, leaf("a").wrap_vec());
                (plan, every_kind_of_variable(), completed)
            }))
            .collect()
    }

    fn print_in(
        format: QueryTextFormat,
        plan: &GraphQLFetchPlan,
        variables: &[FieldVariable],
        selections: &CompletedSelectionMap,
    ) -> String {
        print_operation(
            format,
            &EntrypointOperation {
                name: name("X"),
                fetch_plan: plan,
                variables,
                selections,
            },
        )
        .0
    }

    /// `text` parsed by graphql-parser and printed back in its own format, which writes no
    /// source position: two texts of the same operation print the same.
    fn parsed_and_printed(text: &str) -> String {
        graphql_parser::parse_query::<String>(text)
            .unwrap_or_else(|error| panic!("graphql-parser rejects {text}: {error}"))
            .to_string()
    }

    #[test]
    fn collapsed_text_has_no_newline_and_no_two_consecutive_spaces() {
        for (plan, variables, selections) in format_fixtures() {
            let text = print_in(
                QueryTextFormat::Collapsed,
                plan.reference(),
                variables.as_slice(),
                selections.reference(),
            );
            assert!(!text.contains('\n'), "{text}");
            assert!(!text.contains("  "), "{text}");
        }
    }

    #[test]
    fn both_formats_parse_to_the_same_operation() {
        for (plan, variables, selections) in format_fixtures() {
            let [expanded, collapsed] = [QueryTextFormat::Expanded, QueryTextFormat::Collapsed]
                .map(|format| {
                    print_in(
                        format,
                        plan.reference(),
                        variables.as_slice(),
                        selections.reference(),
                    )
                });
            assert_eq!(
                parsed_and_printed(expanded.as_str()),
                parsed_and_printed(collapsed.as_str()),
                "{expanded}\n{collapsed}"
            );
        }
    }

    #[test]
    fn collapsed_text_writes_a_space_only_between_two_name_characters() {
        assert_eq!(
            print_in(
                QueryTextFormat::Collapsed,
                root(OperationType::Query).reference(),
                &[],
                top_level(vec![
                    composite("pets", vec![], id(), vec![leaf("tagline")],)
                ])
                .reference(),
            ),
            "query X{pets{__typename id tagline}}"
        );
        assert_eq!(
            print_in(
                QueryTextFormat::Collapsed,
                root(OperationType::Mutation).reference(),
                &[],
                top_level(vec![every_kind_of_argument()]).reference(),
            ),
            "mutation X{f____b___l_true____i___l_n5____n___l_null____o___o_s__s_a_32_b_v__v_x_c____s___s__34_q_34_____v___v_x:f(b:true i:-5 n:null o:{s:\"a b\"v:$x}s:\"\\\"q\\\"\"v:$x)}"
        );
        assert_eq!(
            print_in(
                QueryTextFormat::Collapsed,
                root(OperationType::Query).reference(),
                every_kind_of_variable().as_slice(),
                top_level(vec![leaf("a")]).reference(),
            ),
            "query X($limit:Int=3$ids:[ID!]!$maybe:[ID]$nested:[[Int!]]!$flag:Boolean!=false){a}"
        );
        assert_eq!(
            print_in(
                QueryTextFormat::Collapsed,
                root(OperationType::Query).reference(),
                &[],
                top_level(vec![union_feed()]).reference(),
            ),
            "query X{newsfeed____limit___l_2:newsfeed(limit:2){__typename cursor...on BlogItem{__typename id title}}}"
        );
    }
}
