use graphql_lang_types::{
    DescriptionValue, LinkedFieldAlias, LinkedFieldName, OutputTypeName, ScalarFieldAlias,
    ScalarFieldName, WithSpan,
};

use crate::string_key_types::ResolverFieldName;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ResolverDeclaration {
    pub description: Option<WithSpan<DescriptionValue>>,
    pub parent_type: WithSpan<OutputTypeName>,
    pub resolver_field_name: WithSpan<ResolverFieldName>,
    pub selection_set_and_unwraps: Option<SelectionSetAndUnwraps>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct SelectionSetAndUnwraps {
    pub selection_set: Vec<WithSpan<Selection>>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Selection {
    ScalarField(ScalarFieldSelection),
    LinkedField(LinkedFieldSelection),
    // FieldGroup(FieldGroupSelection),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ScalarFieldSelection {
    pub alias: Option<WithSpan<ScalarFieldAlias>>,
    pub field_name: WithSpan<ScalarFieldName>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct LinkedFieldSelection {
    pub alias: Option<WithSpan<LinkedFieldAlias>>,
    pub field_name: WithSpan<LinkedFieldName>,
    pub unwraps: Vec<WithSpan<Unwrap>>,
    pub selection_set: Vec<WithSpan<Selection>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Unwrap {
    ActualUnwrap,
    SkippedUnwrap,
    // FakeUnwrap?
}
