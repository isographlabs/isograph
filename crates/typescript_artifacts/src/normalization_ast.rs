use common_lang_types::EntityNameAndSelectableName;
use isograph_compiler::{
    CompletedIdentity, Condition, NormalizationArguments, NormalizationAst,
    NormalizationCompositeTarget, NormalizationNode, NormalizationRootRecord,
    NormalizationSelectionSet, NormalizationTarget, NormalizationValue, RecordEntity, ResponseKey,
};
use isograph_parser::Boolean;

use crate::entrypoint::{RUNTIME_PACKAGE, normalization_ast_path};
use crate::module::{
    ArtifactModule, ArtifactObjectEntry, ArtifactValue, DeclaredExportType, DeclaredTypeArgument,
    DefaultExportType, NamedTypeImport,
};

const NORMALIZATION_AST_TYPE: &str = "NormalizationAst";

/// The normalization AST artifact of `entrypoint`: its AST, declared as a `NormalizationAst` of
/// its root entity, as the default export.
pub(crate) fn normalization_ast_artifact(
    entrypoint: EntityNameAndSelectableName,
    ast: &NormalizationAst,
) -> ArtifactModule {
    ArtifactModule {
        path: normalization_ast_path(entrypoint),
        default_export: ast_value(ast),
        default_export_type: DefaultExportType::Declared(DeclaredExportType {
            binding: "normalizationAst".to_owned(),
            type_import: NamedTypeImport {
                specifier: RUNTIME_PACKAGE.to_owned(),
                imported: NORMALIZATION_AST_TYPE.to_owned(),
                local: NORMALIZATION_AST_TYPE.to_owned(),
            },
            type_arguments: vec![DeclaredTypeArgument::StringLiteral(
                ast.root.entity_name.to_string(),
            )],
        }),
    }
}

fn entry(key: &str, value: ArtifactValue) -> ArtifactObjectEntry {
    ArtifactObjectEntry {
        key: key.to_owned(),
        value,
    }
}

fn string(value: impl ToString) -> ArtifactValue {
    ArtifactValue::String(value.to_string())
}

fn kind(value: &str) -> ArtifactObjectEntry {
    entry("kind", string(value))
}

fn ast_value(ast: &NormalizationAst) -> ArtifactValue {
    ArtifactValue::Object(vec![
        entry(
            "discriminatorResponseKey",
            string(ast.discriminator_response_key),
        ),
        entry("root", root_record_value(&ast.root)),
        entry("selections", selection_set_value(&ast.selections)),
    ])
}

fn root_record_value(root: &NormalizationRootRecord) -> ArtifactValue {
    ArtifactValue::Object(vec![
        entry("entityName", string(root.entity_name)),
        entry("id", string(&root.id)),
    ])
}

fn selection_set_value(selection_set: &NormalizationSelectionSet) -> ArtifactValue {
    ArtifactValue::Array(selection_set.0.iter().map(node_value).collect())
}

fn node_value(node: &NormalizationNode) -> ArtifactValue {
    match node {
        NormalizationNode::Selection(selection) => ArtifactValue::Object(vec![
            kind("Selection"),
            entry(
                "responseKey",
                match &selection.response_key {
                    ResponseKey::SelectableName => ArtifactValue::Null,
                    ResponseKey::Alias(alias) => string(alias),
                },
            ),
            entry("selectableName", string(selection.selectable_name)),
            entry("arguments", arguments_value(&selection.arguments)),
            entry(
                "target",
                match &selection.target {
                    NormalizationTarget::Leaf => ArtifactValue::Null,
                    NormalizationTarget::Composite(target) => composite_target_value(target),
                },
            ),
        ]),
        NormalizationNode::Conditional(conditional) => ArtifactValue::Object(vec![
            kind("Conditional"),
            entry("condition", condition_value(&conditional.condition)),
            entry("identity", identity_value(&conditional.identity)),
            entry("selections", selection_set_value(&conditional.selections)),
        ]),
    }
}

fn composite_target_value(target: &NormalizationCompositeTarget) -> ArtifactValue {
    ArtifactValue::Object(vec![
        entry("entity", entity_value(&target.entity)),
        entry("identity", identity_value(&target.identity)),
        entry("selections", selection_set_value(&target.selections)),
    ])
}

fn entity_value(entity: &RecordEntity) -> ArtifactValue {
    match entity {
        RecordEntity::Known(entity_name) => ArtifactValue::Object(vec![
            kind("Known"),
            entry("entityName", string(entity_name)),
        ]),
        RecordEntity::Discriminator => ArtifactValue::Object(vec![kind("Discriminator")]),
    }
}

fn identity_value(identity: &CompletedIdentity) -> ArtifactValue {
    match identity {
        CompletedIdentity::Select(name) => {
            ArtifactValue::Object(vec![kind("Select"), entry("selectableName", string(name))])
        }
        CompletedIdentity::Omit => ArtifactValue::Object(vec![kind("Omit")]),
        CompletedIdentity::Fixed(id) => {
            ArtifactValue::Object(vec![kind("Fixed"), entry("id", string(id))])
        }
    }
}

fn condition_value(condition: &Condition) -> ArtifactValue {
    match condition {
        Condition::SelectionEquals(entity_name) => ArtifactValue::Object(vec![
            kind("SelectionEquals"),
            entry("entityName", string(entity_name)),
        ]),
    }
}

fn arguments_value(arguments: &NormalizationArguments) -> ArtifactValue {
    if arguments.0.is_empty() {
        return ArtifactValue::Null;
    }
    ArtifactValue::Array(
        arguments
            .0
            .iter()
            .map(|argument| {
                ArtifactValue::Array(vec![string(argument.name), argument_value(&argument.value)])
            })
            .collect(),
    )
}

fn argument_value(value: &NormalizationValue) -> ArtifactValue {
    let literal = |value| ArtifactValue::Object(vec![kind("Literal"), entry("value", value)]);
    match value {
        NormalizationValue::Variable(name) => {
            ArtifactValue::Object(vec![kind("Variable"), entry("name", string(name))])
        }
        NormalizationValue::String(text) => {
            ArtifactValue::Object(vec![kind("String"), entry("value", string(text))])
        }
        NormalizationValue::Integer(integer) => literal(ArtifactValue::Number(f64::from(*integer))),
        NormalizationValue::Boolean(Boolean::True) => literal(ArtifactValue::Boolean(true)),
        NormalizationValue::Boolean(Boolean::False) => literal(ArtifactValue::Boolean(false)),
        NormalizationValue::Null => literal(ArtifactValue::Null),
        NormalizationValue::Object(entries) => ArtifactValue::Object(vec![
            kind("Object"),
            entry(
                "value",
                ArtifactValue::Array(
                    entries
                        .iter()
                        .map(|object_entry| {
                            ArtifactValue::Array(vec![
                                string(object_entry.name),
                                argument_value(&object_entry.value),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, EntityNameAndSelectableName, SelectableName};
    use intern::string_key::Intern;
    use isograph_compiler::{
        CompletedIdentity, Condition, FixedRecordId, NormalizationAlias, NormalizationArgument,
        NormalizationArguments, NormalizationAst, NormalizationCompositeTarget,
        NormalizationConditional, NormalizationNode, NormalizationObjectEntry,
        NormalizationRootRecord, NormalizationSelection, NormalizationSelectionSet,
        NormalizationTarget, NormalizationValue, RecordEntity, ResponseKey,
    };
    use isograph_parser::Boolean;
    use prelude::Postfix;

    use super::normalization_ast_artifact;
    use crate::entrypoint::entrypoint_artifact;
    use crate::module::module_of_artifact;
    use crate::swc::print;

    fn name(s: &str) -> SelectableName {
        s.intern().to()
    }

    fn entity(s: &str) -> EntityName {
        s.intern().to()
    }

    fn home_route() -> EntityNameAndSelectableName {
        EntityNameAndSelectableName {
            parent_entity_name: entity("Query"),
            selectable_name: name("HomeRoute"),
        }
    }

    fn leaf(s: &str) -> NormalizationNode {
        NormalizationNode::Selection(NormalizationSelection {
            response_key: ResponseKey::SelectableName,
            selectable_name: name(s),
            arguments: NormalizationArguments::default(),
            target: NormalizationTarget::Leaf,
        })
    }

    fn composite(
        s: &str,
        response_key: ResponseKey,
        arguments: Vec<(&str, NormalizationValue)>,
        entity: RecordEntity,
        identity: CompletedIdentity,
        selections: Vec<NormalizationNode>,
    ) -> NormalizationNode {
        NormalizationNode::Selection(NormalizationSelection {
            response_key,
            selectable_name: name(s),
            arguments: NormalizationArguments(
                arguments
                    .into_iter()
                    .map(|(name, value)| NormalizationArgument {
                        name: name.intern().to(),
                        value,
                    })
                    .collect(),
            ),
            target: NormalizationTarget::Composite(NormalizationCompositeTarget {
                entity,
                identity,
                selections: NormalizationSelectionSet(selections),
            }),
        })
    }

    fn ast(root: &str, selections: Vec<NormalizationNode>) -> NormalizationAst {
        NormalizationAst {
            discriminator_response_key: name("__typename"),
            root: NormalizationRootRecord {
                entity_name: entity(root),
                id: FixedRecordId::root(),
            },
            selections: NormalizationSelectionSet(selections),
        }
    }

    fn printed(ast: &NormalizationAst) -> String {
        print(
            module_of_artifact(normalization_ast_artifact(home_route(), ast).reference())
                .reference(),
        )
    }

    fn sample() -> NormalizationAst {
        ast(
            "Query",
            vec![
                composite(
                    "pet",
                    ResponseKey::Alias(NormalizationAlias("pet____id___v_id".to_owned())),
                    vec![("id", NormalizationValue::Variable("id".intern().to()))],
                    RecordEntity::Known(entity("Pet")),
                    CompletedIdentity::Select(name("id")),
                    vec![leaf("id"), leaf("firstName")],
                ),
                composite(
                    "feed",
                    ResponseKey::SelectableName,
                    vec![],
                    RecordEntity::Discriminator,
                    CompletedIdentity::Omit,
                    vec![NormalizationNode::Conditional(NormalizationConditional {
                        condition: Condition::SelectionEquals(entity("Pet")),
                        identity: CompletedIdentity::Select(name("id")),
                        selections: NormalizationSelectionSet(vec![leaf("tagline")]),
                    })],
                ),
                composite(
                    "query",
                    ResponseKey::SelectableName,
                    vec![],
                    RecordEntity::Known(entity("Query")),
                    CompletedIdentity::Fixed(FixedRecordId::root()),
                    vec![],
                ),
                composite(
                    "f",
                    ResponseKey::Alias(NormalizationAlias("f____x".to_owned())),
                    vec![
                        ("b", NormalizationValue::Boolean(Boolean::True)),
                        ("i", NormalizationValue::Integer(5)),
                        ("n", NormalizationValue::Null),
                        (
                            "o",
                            NormalizationValue::Object(vec![NormalizationObjectEntry {
                                name: "k".intern().to(),
                                value: NormalizationValue::Boolean(Boolean::False),
                            }]),
                        ),
                        ("s", NormalizationValue::String("text".intern().to())),
                    ],
                    RecordEntity::Discriminator,
                    CompletedIdentity::Omit,
                    vec![],
                ),
            ],
        )
    }

    #[test]
    fn the_module_imports_the_type_declares_the_const_and_exports_it() {
        let text = printed(sample().reference());
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0],
            "import type { NormalizationAst } from \"@isograph/runtime\";"
        );
        assert!(
            lines
                .iter()
                .any(|line| line
                    .starts_with("const normalizationAst: NormalizationAst<\"Query\"> = {")),
            "{text}"
        );
        assert_eq!(
            lines.last().copied(),
            "export default normalizationAst;".wrap_some()
        );
        assert!(
            text.contains("discriminatorResponseKey: \"__typename\""),
            "{text}"
        );
    }

    #[test]
    fn the_root_record_names_the_root_entity_and_the_root_id() {
        let text = printed(sample().reference());
        assert!(text.contains("root: {"), "{text}");
        assert!(text.contains("entityName: \"Query\""), "{text}");
        assert!(text.contains("id: \"__ROOT\""), "{text}");
        let mutation = printed(ast("Mutation", vec![]).reference());
        assert!(
            mutation.contains("NormalizationAst<\"Mutation\">"),
            "{mutation}"
        );
        assert!(mutation.contains("entityName: \"Mutation\""), "{mutation}");
        assert!(mutation.contains("selections: []"), "{mutation}");
    }

    #[test]
    fn selections_print_their_response_key_name_arguments_and_target() {
        let text = printed(sample().reference());
        let compact: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            compact.contains("kind: \"Selection\", responseKey: null, selectableName: \"firstName\", arguments: null, target: null"),
            "{text}"
        );
        assert!(
            compact.contains("responseKey: \"pet____id___v_id\""),
            "{text}"
        );
        assert!(
            compact.contains("[ \"id\", { kind: \"Variable\", name: \"id\" } ]"),
            "{text}"
        );
    }

    #[test]
    fn targets_print_their_entity_identity_and_selections() {
        let compact: String = printed(sample().reference())
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            compact.contains("entity: { kind: \"Known\", entityName: \"Pet\" }"),
            "{compact}"
        );
        assert!(
            compact.contains("identity: { kind: \"Select\", selectableName: \"id\" }"),
            "{compact}"
        );
        assert!(
            compact.contains("entity: { kind: \"Discriminator\" }"),
            "{compact}"
        );
        assert!(
            compact.contains("identity: { kind: \"Omit\" }"),
            "{compact}"
        );
        assert!(
            compact.contains("identity: { kind: \"Fixed\", id: \"__ROOT\" }"),
            "{compact}"
        );
        assert!(compact.contains("selections: []"), "{compact}");
    }

    #[test]
    fn a_conditional_prints_its_condition_and_identity() {
        let compact: String = printed(sample().reference())
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            compact.contains("kind: \"Conditional\", condition: { kind: \"SelectionEquals\", entityName: \"Pet\" }, identity: { kind: \"Select\", selectableName: \"id\" }"),
            "{compact}"
        );
    }

    #[test]
    fn argument_values_print_every_kind() {
        let compact: String = printed(sample().reference())
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for expected in [
            "[ \"b\", { kind: \"Literal\", value: true } ]",
            "[ \"i\", { kind: \"Literal\", value: 5 } ]",
            "[ \"n\", { kind: \"Literal\", value: null } ]",
            "[ \"o\", { kind: \"Object\", value: [ [ \"k\", { kind: \"Literal\", value: false } ] ] } ]",
            "[ \"s\", { kind: \"String\", value: \"text\" } ]",
        ] {
            assert!(compact.contains(expected), "{expected} in {compact}");
        }
    }

    #[test]
    fn the_entrypoint_is_declared_as_an_isograph_entrypoint_of_its_types_and_root() {
        let text = print(
            module_of_artifact(entrypoint_artifact(home_route(), entity("Query")).reference())
                .reference(),
        );
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0],
            "import type { IsographEntrypoint } from \"@isograph/runtime\";"
        );
        assert!(
            lines.contains(&"import type { Query__HomeRoute__param } from \"./param_type\";"),
            "{text}"
        );
        assert!(
            lines
                .contains(&"import type { Query__HomeRoute__output_type } from \"./output_type\";"),
            "{text}"
        );
        assert!(
            text.contains("const artifact: IsographEntrypoint<Query__HomeRoute__param, Query__HomeRoute__output_type, \"Query\"> = {"),
            "{text}"
        );
        assert_eq!(
            lines.last().copied(),
            "export default artifact;".wrap_some()
        );
        assert!(
            text.contains("queryText: Query__HomeRoute__query_text"),
            "{text}"
        );
        assert!(
            text.contains("normalizationAst: Query__HomeRoute__normalization_ast"),
            "{text}"
        );
    }
}
