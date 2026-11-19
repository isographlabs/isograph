use common_lang_types::{
    ArgumentName, EntityName, SelectableName, StringLiteralValue, ValueKeyName, VariableName,
};
use isograph_parser::Boolean;
use prelude::Postfix;

use crate::merged_selection::{
    CompletedCompositeTarget, CompletedIdentity, CompletedSelection, CompletedSelectionMap,
    CompletedSelectionTarget, Condition, FixedRecordId, MergedArguments, MergedValue, RecordEntity,
    ResponseKey,
};

/// The normalization AST of one query's response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationAst {
    /// The protocol's discriminator, selected with no alias in every composite response
    /// object: its selectable name is its response key.
    pub discriminator_response_key: SelectableName,
    /// The record the top-level selections are written onto.
    pub root: NormalizationRootRecord,
    pub selections: NormalizationSelectionSet,
}

/// The operation's root entity's one record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationRootRecord {
    pub entity_name: EntityName,
    pub id: FixedRecordId,
}

/// The nodes a normalizer walks, in query order. Empty for a composite selection whose only
/// selection is the discriminator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationSelectionSet(pub Vec<NormalizationNode>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizationNode {
    Selection(NormalizationSelection),
    Conditional(NormalizationConditional),
}

/// A selection of a selectable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationSelection {
    pub response_key: ResponseKey,
    pub selectable_name: SelectableName,
    pub arguments: NormalizationArguments,
    pub target: NormalizationTarget,
}

/// Whether a selection's target entity has selections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizationTarget {
    Leaf,
    Composite(NormalizationCompositeTarget),
}

/// The selections on a composite target entity, and which records its response objects are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationCompositeTarget {
    pub entity: RecordEntity,
    pub identity: CompletedIdentity,
    pub selections: NormalizationSelectionSet,
}

/// The records of a conditional's response objects are of its condition's entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationConditional {
    pub condition: Condition,
    pub identity: CompletedIdentity,
    pub selections: NormalizationSelectionSet,
}

/// Arguments in `MergedArguments` order: sorted by name, no name twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NormalizationArguments(pub Vec<NormalizationArgument>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationArgument {
    pub name: ArgumentName,
    pub value: NormalizationValue,
}

/// `MergedValue` with integers narrowed to GraphQL's 32-bit `Int`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizationValue {
    Variable(VariableName),
    String(StringLiteralValue),
    Integer(i32),
    Boolean(Boolean),
    Null,
    Object(Vec<NormalizationObjectEntry>),
    List(Vec<NormalizationValue>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizationObjectEntry {
    pub name: ValueKeyName,
    pub value: NormalizationValue,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is outside GraphQL's 32-bit `Int` range.")]
pub struct IntegerOutOfRange(pub i64);

/// The normalization AST of the response to the operation whose top-level selections are
/// `selections`.
pub fn normalization_ast(
    selections: &CompletedSelectionMap,
    discriminator_response_key: SelectableName,
    root_entity: EntityName,
) -> Result<NormalizationAst, IntegerOutOfRange> {
    NormalizationAst {
        discriminator_response_key,
        root: NormalizationRootRecord {
            entity_name: root_entity,
            id: FixedRecordId::root(),
        },
        selections: normalization_selection_set(selections)?,
    }
    .wrap_ok()
}

/// One node per selection of `selections`, in key order, without the discriminator.
fn normalization_selection_set(
    selections: &CompletedSelectionMap,
) -> Result<NormalizationSelectionSet, IntegerOutOfRange> {
    selections
        .iter()
        .filter_map(normalization_node)
        .collect::<Result<Vec<_>, _>>()
        .map(NormalizationSelectionSet)
}

/// `selection` as a node; `None` for the discriminator.
fn normalization_node(
    selection: &CompletedSelection,
) -> Option<Result<NormalizationNode, IntegerOutOfRange>> {
    match selection {
        CompletedSelection::Discriminator => None,
        CompletedSelection::Identity(name) => {
            NormalizationNode::Selection(NormalizationSelection {
                response_key: ResponseKey::SelectableName,
                selectable_name: name.dereference(),
                arguments: NormalizationArguments::default(),
                target: NormalizationTarget::Leaf,
            })
            .wrap_ok()
            .wrap_some()
        }
        CompletedSelection::Selectable(selection) => normalization_selection(
            selection.key.response_key(),
            selection.key.name,
            selection.key.arguments.reference(),
            selection.target.reference(),
        )
        .map(NormalizationNode::Selection)
        .wrap_some(),
        CompletedSelection::Conditional(conditional) => {
            normalization_selection_set(conditional.selections.reference())
                .map(|selections| {
                    NormalizationNode::Conditional(NormalizationConditional {
                        condition: conditional.condition.clone(),
                        identity: conditional.identity.clone(),
                        selections,
                    })
                })
                .wrap_some()
        }
    }
}

/// The node of a selection of `selectable_name` with `arguments` and `target`.
fn normalization_selection(
    response_key: ResponseKey,
    selectable_name: SelectableName,
    arguments: &MergedArguments,
    target: &CompletedSelectionTarget,
) -> Result<NormalizationSelection, IntegerOutOfRange> {
    NormalizationSelection {
        response_key,
        selectable_name,
        arguments: normalization_arguments(arguments)?,
        target: match target {
            CompletedSelectionTarget::Leaf => NormalizationTarget::Leaf,
            CompletedSelectionTarget::Composite(target) => {
                NormalizationTarget::Composite(normalization_composite_target(target)?)
            }
        },
    }
    .wrap_ok()
}

fn normalization_composite_target(
    target: &CompletedCompositeTarget,
) -> Result<NormalizationCompositeTarget, IntegerOutOfRange> {
    NormalizationCompositeTarget {
        entity: target.entity.clone(),
        identity: target.identity.clone(),
        selections: normalization_selection_set(target.selections.reference())?,
    }
    .wrap_ok()
}

/// `arguments` with every integer narrowed to GraphQL's 32-bit `Int`.
pub(crate) fn normalization_arguments(
    arguments: &MergedArguments,
) -> Result<NormalizationArguments, IntegerOutOfRange> {
    arguments
        .iter()
        .map(|argument| {
            NormalizationArgument {
                name: argument.name,
                value: normalization_value(argument.value.reference())?,
            }
            .wrap_ok()
        })
        .collect::<Result<Vec<_>, _>>()
        .map(NormalizationArguments)
}

fn normalization_value(value: &MergedValue) -> Result<NormalizationValue, IntegerOutOfRange> {
    match value {
        MergedValue::Variable(name) => NormalizationValue::Variable(name.dereference()).wrap_ok(),
        MergedValue::String(string) => NormalizationValue::String(string.dereference()).wrap_ok(),
        MergedValue::Integer(integer) => i32::try_from(integer.dereference())
            .map(NormalizationValue::Integer)
            .map_err(|_| IntegerOutOfRange(integer.dereference())),
        MergedValue::Boolean(boolean) => {
            NormalizationValue::Boolean(boolean.dereference()).wrap_ok()
        }
        MergedValue::Null => NormalizationValue::Null.wrap_ok(),
        MergedValue::Object(object) => object
            .iter()
            .map(|entry| {
                NormalizationObjectEntry {
                    name: entry.name,
                    value: normalization_value(entry.value.reference())?,
                }
                .wrap_ok()
            })
            .collect::<Result<Vec<_>, _>>()
            .map(NormalizationValue::Object),
        MergedValue::List(items) => items
            .iter()
            .map(normalization_value)
            .collect::<Result<Vec<_>, _>>()
            .map(NormalizationValue::List),
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, SelectableName};
    use intern::string_key::Intern;
    use prelude::Postfix;

    use super::{
        NormalizationCompositeTarget, NormalizationNode, NormalizationRootRecord,
        NormalizationSelection, NormalizationSelectionSet, NormalizationTarget, normalization_ast,
    };
    use crate::merged_selection::{
        CompletedIdentity, CompletedSelectionMap, Condition, FixedRecordId, MergedCompositeTarget,
        MergedConditional, MergedSelectableSelection, MergedSelection, MergedSelectionMap,
        MergedSelectionTarget, RecordEntity, ResponseKey, SelectionKey,
    };
    use crate::test_profile::TestNetworkProtocol;
    use crate::{CompositeShape, Concreteness, IdentitySelection, RootEntities};

    fn name(s: &str) -> SelectableName {
        s.intern().to()
    }

    fn entity(s: &str) -> EntityName {
        s.intern().to()
    }

    fn leaf(s: &str) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: SelectionKey {
                name: name(s),
                arguments: Default::default(),
            },
            target: MergedSelectionTarget::Leaf,
        })
    }

    fn map(selections: Vec<MergedSelection>) -> MergedSelectionMap {
        selections
            .into_iter()
            .map(MergedSelectionMap::single)
            .try_fold(MergedSelectionMap::empty(), MergedSelectionMap::merge)
            .expect("the test's selections merge")
    }

    fn composite(
        s: &str,
        target: &str,
        shape: CompositeShape,
        selections: Vec<MergedSelection>,
    ) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: SelectionKey {
                name: name(s),
                arguments: Default::default(),
            },
            target: MergedSelectionTarget::Composite(MergedCompositeTarget {
                entity: entity(target),
                shape,
                selections: map(selections),
            }),
        })
    }

    fn leaf_node(s: &str) -> NormalizationNode {
        NormalizationNode::Selection(NormalizationSelection {
            response_key: ResponseKey::SelectableName,
            selectable_name: name(s),
            arguments: Default::default(),
            target: NormalizationTarget::Leaf,
        })
    }

    fn ast_of(selections: Vec<MergedSelection>) -> super::NormalizationAst {
        let roots = RootEntities {
            operation: entity("Query"),
            others: Default::default(),
        };
        let completed = CompletedSelectionMap::top_level::<TestNetworkProtocol>(
            map(selections).reference(),
            roots.reference(),
        )
        .expect("the test's top level completes");
        normalization_ast(completed.reference(), name("__typename"), entity("Query"))
            .expect("the test has no integers")
    }

    #[test]
    fn the_discriminator_has_no_node_the_identity_is_a_leaf_node_and_conditionals_follow() {
        let ast = ast_of(vec![composite(
            "pet",
            "Pet",
            CompositeShape {
                concreteness: Concreteness::Concrete,
                identity: IdentitySelection::Select(name("id")),
            },
            vec![
                leaf("age"),
                MergedSelection::Conditional(MergedConditional {
                    condition: Condition::SelectionEquals(entity("Dog")),
                    identity: IdentitySelection::Omit,
                    selections: map(vec![leaf("bark")]),
                }),
            ],
        )]);
        assert_eq!(
            ast.root,
            NormalizationRootRecord {
                entity_name: entity("Query"),
                id: FixedRecordId::root(),
            }
        );
        assert_eq!(ast.discriminator_response_key, name("__typename"));
        assert_eq!(
            ast.selections,
            NormalizationSelectionSet(vec![NormalizationNode::Selection(NormalizationSelection {
                response_key: ResponseKey::SelectableName,
                selectable_name: name("pet"),
                arguments: Default::default(),
                target: NormalizationTarget::Composite(NormalizationCompositeTarget {
                    entity: RecordEntity::Known(entity("Pet")),
                    identity: CompletedIdentity::Select(name("id")),
                    selections: NormalizationSelectionSet(vec![
                        leaf_node("id"),
                        leaf_node("age"),
                        NormalizationNode::Conditional(super::NormalizationConditional {
                            condition: Condition::SelectionEquals(entity("Dog")),
                            identity: CompletedIdentity::Omit,
                            selections: NormalizationSelectionSet(vec![leaf_node("bark")]),
                        }),
                    ]),
                }),
            })])
        );
    }

    #[test]
    fn a_composite_with_only_the_discriminator_has_no_nodes() {
        let ast = ast_of(vec![composite(
            "u",
            "Union",
            CompositeShape {
                concreteness: Concreteness::Abstract,
                identity: IdentitySelection::Omit,
            },
            vec![],
        )]);
        let [NormalizationNode::Selection(u)] = ast.selections.0.as_slice() else {
            unreachable!("the AST has one selection")
        };
        assert_eq!(
            u.target,
            NormalizationTarget::Composite(NormalizationCompositeTarget {
                entity: RecordEntity::Discriminator,
                identity: CompletedIdentity::Omit,
                selections: NormalizationSelectionSet(Vec::new()),
            })
        );
    }

    #[test]
    fn a_leaf_named_like_the_discriminator_at_the_top_level_is_a_node() {
        let ast = ast_of(vec![leaf("__typename")]);
        assert_eq!(
            ast.selections,
            NormalizationSelectionSet(leaf_node("__typename").wrap_vec())
        );
    }
}
