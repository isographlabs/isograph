use common_lang_types::SelectableName;

/// What the core reads about an entity the protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityShape {
    /// A selection of it has no selections (GraphQL scalars and enums).
    Leaf,
    /// Never the target of a selection (GraphQL input objects).
    Input,
    /// A selection of it selects its selectables; the protocol adds the discriminator and
    /// this identity selection to every selection set on it.
    Composite(IdentitySelection),
}

/// Whether the protocol selects the identity, the selectable whose value identifies the
/// response object's record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentitySelection {
    Select(SelectableName),
    Omit,
}

/// What a selection of a protocol selectable is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectableShape {
    /// A selection of the selectable's target entity.
    Direct,
    /// A conditional: its selections apply when the response object is the selectable's
    /// target entity, and select on that entity.
    Refinement,
}
