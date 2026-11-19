use common_lang_types::{EntityNameAndSelectableName, SelectableName, SelectionName};

use crate::merged_selection::Condition;
use crate::normalization_ast::NormalizationArguments;

/// What a reader reads for one selection set, in selection order. Empty for a field with no
/// selection set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSelectionSet(pub Vec<ReaderSelection>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReaderSelection {
    /// A server selectable whose target is a scalar: the stored value, whole.
    Scalar(ReaderScalarSelection),
    /// A server selectable whose target has selectables: the selections, read at every record
    /// the stored value links to.
    Linked(ReaderLinkedSelection),
    /// A refinement: the selections, read on the same record when the condition holds.
    Refinement(ReaderRefinementSelection),
    /// A client field: the value its resolver returns.
    Resolver(ReaderResolverSelection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderScalarSelection {
    /// The key of the value in the resolver's `data`: the reader alias, else the selectable name.
    pub alias: SelectionName,
    pub selectable: SelectableName,
    pub arguments: NormalizationArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderLinkedSelection {
    pub alias: SelectionName,
    pub selectable: SelectableName,
    pub arguments: NormalizationArguments,
    pub selections: ReaderSelectionSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderRefinementSelection {
    pub alias: SelectionName,
    pub condition: Condition,
    pub selections: ReaderSelectionSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderResolverSelection {
    pub alias: SelectionName,
    /// The selected client field, whose reader artifact the node imports.
    pub field: EntityNameAndSelectableName,
    /// The arguments the selection passes, and the default of each of the field's variables it
    /// does not pass.
    pub arguments: NormalizationArguments,
}
