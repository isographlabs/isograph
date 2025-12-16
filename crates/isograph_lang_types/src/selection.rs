use common_lang_types::{
    SelectableAlias, SelectableName, SelectableNameOrAlias, VariableName, WithLocation, WithSpan,
};
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;

use crate::{
    IsographResolvedNode, NonConstantValue, ObjectSelectionDirectiveSet,
    ScalarSelectionDirectiveSet, SelectionFieldArgument, SelectionSet, SelectionSetPath,
    SelectionType,
};

pub type Selection = SelectionType<ScalarSelection, ObjectSelection>;

impl Selection {
    pub fn name_or_alias(&self) -> WithLocation<SelectableNameOrAlias> {
        match self {
            SelectionType::Scalar(scalar_field) => scalar_field.name_or_alias(),
            SelectionType::Object(linked_field) => linked_field.name_or_alias(),
        }
    }

    pub fn name(&self) -> SelectableName {
        match self {
            SelectionType::Scalar(s) => s.name.item,
            SelectionType::Object(o) => o.name.item,
        }
    }

    pub fn variables<'a>(&'a self) -> impl Iterator<Item = VariableName> + 'a {
        let get_variable = |x: &'a WithLocation<SelectionFieldArgument>| match x.item.value.item {
            NonConstantValue::Variable(v) => Some(v),
            _ => None,
        };
        match self {
            SelectionType::Scalar(scalar_field) => {
                scalar_field.arguments.iter().flat_map(get_variable)
            }
            SelectionType::Object(linked_field) => {
                linked_field.arguments.iter().flat_map(get_variable)
            }
        }
    }

    pub fn is_updatable(&self) -> bool {
        match self {
            SelectionType::Scalar(s) => matches!(
                s.scalar_selection_directive_set,
                ScalarSelectionDirectiveSet::Updatable(_)
            ),
            SelectionType::Object(o) => matches!(
                o.object_selection_directive_set,
                ObjectSelectionDirectiveSet::Updatable(_)
            ),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=SelectionParentType<'a>, resolved_node=IsographResolvedNode<'a>)]
// TODO remove type parameter
pub struct ScalarSelection {
    // TODO make this WithSpan instead of WithLocation
    pub name: WithLocation<SelectableName>,
    // TODO make this WithSpan instead of WithLocation
    pub reader_alias: Option<WithLocation<SelectableAlias>>,
    // TODO make this WithSpan instead of WithLocation
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
    pub scalar_selection_directive_set: ScalarSelectionDirectiveSet,
}

pub type ScalarSelectionPath<'a> =
    PositionResolutionPath<&'a ScalarSelection, SelectionParentType<'a>>;

impl ScalarSelection {
    pub fn name_or_alias(&self) -> WithLocation<SelectableNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(SelectableNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(SelectableNameOrAlias::from))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=SelectionParentType<'a>, resolved_node=IsographResolvedNode<'a>)]
// TODO remove the type parameters
pub struct ObjectSelection {
    // TODO make this WithSpan instead of WithLocation
    pub name: WithLocation<SelectableName>,
    // TODO make this WithSpan instead of WithLocation
    pub reader_alias: Option<WithLocation<SelectableAlias>>,
    #[resolve_field]
    pub selection_set: WithSpan<SelectionSet>,
    // TODO make this WithSpan instead of WithLocation
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
    pub object_selection_directive_set: ObjectSelectionDirectiveSet,
}

pub type ObjectSelectionPath<'a> =
    PositionResolutionPath<&'a ObjectSelection, SelectionParentType<'a>>;

// TODO can we replace this directly with SelectionSetPath?
#[derive(Debug)]
pub enum SelectionParentType<'a> {
    SelectionSet(SelectionSetPath<'a>),
}

impl ObjectSelection {
    pub fn name_or_alias(&self) -> WithLocation<SelectableNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(SelectableNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(SelectableNameOrAlias::from))
    }
}
