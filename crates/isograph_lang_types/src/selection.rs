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

pub type UnvalidatedSelection = SelectionTypeContainingSelections<(), ()>;

pub type UnvalidatedScalarFieldSelection = ScalarSelection<()>;

pub type SelectionTypeContainingSelections<TScalarField, TLinkedField> =
    SelectionType<ScalarSelection<TScalarField>, ObjectSelection<TScalarField, TLinkedField>>;

impl<TScalarField, TLinkedField> SelectionTypeContainingSelections<TScalarField, TLinkedField> {
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
#[resolve_position(parent_type=SelectionParentType<'a>, resolved_node=IsographResolvedNode<'a>, self_type_generics=<()>)]
// TODO remove type parameter
pub struct ScalarSelection<TScalarField> {
    // TODO make this WithSpan instead of WithLocation
    pub name: WithLocation<SelectableName>,
    // TODO make this WithSpan instead of WithLocation
    pub reader_alias: Option<WithLocation<SelectableAlias>>,
    /// TODO do not use this field! Instead, we need to look things up
    /// from the database instead of accessing this field.
    pub deprecated_associated_data: TScalarField,
    // TODO make this WithSpan instead of WithLocation
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
    pub scalar_selection_directive_set: ScalarSelectionDirectiveSet,
}

pub type ScalarSelectionPath<'a> =
    PositionResolutionPath<&'a ScalarSelection<()>, SelectionParentType<'a>>;

impl<TScalarField> ScalarSelection<TScalarField> {
    pub fn name_or_alias(&self) -> WithLocation<SelectableNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(SelectableNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(SelectableNameOrAlias::from))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=SelectionParentType<'a>, resolved_node=IsographResolvedNode<'a>, self_type_generics=<(), ()>)]
// TODO remove the type parameters
pub struct ObjectSelection<TScalar, TLinked> {
    // TODO make this WithSpan instead of WithLocation
    pub name: WithLocation<SelectableName>,
    // TODO make this WithSpan instead of WithLocation
    pub reader_alias: Option<WithLocation<SelectableAlias>>,
    pub deprecated_associated_data: TLinked,
    #[resolve_field]
    pub selection_set: WithSpan<SelectionSet<TScalar, TLinked>>,
    // TODO make this WithSpan instead of WithLocation
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
    pub object_selection_directive_set: ObjectSelectionDirectiveSet,
}

pub type ObjectSelectionPath<'a> =
    PositionResolutionPath<&'a ObjectSelection<(), ()>, SelectionParentType<'a>>;

// TODO can we replace this directly with SelectionSetPath?
#[derive(Debug)]
pub enum SelectionParentType<'a> {
    SelectionSet(SelectionSetPath<'a>),
}

impl<TScalarField, TLinkedField> ObjectSelection<TScalarField, TLinkedField> {
    pub fn name_or_alias(&self) -> WithLocation<SelectableNameOrAlias> {
        self.reader_alias
            .map(|item| item.map(SelectableNameOrAlias::from))
            .unwrap_or_else(|| self.name.map(SelectableNameOrAlias::from))
    }
}
