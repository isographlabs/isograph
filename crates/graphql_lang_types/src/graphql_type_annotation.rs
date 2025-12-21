use std::{fmt, ops::Deref};

use common_lang_types::{EntityName, WithEmbeddedLocation};
use prelude::Postfix;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLTypeAnnotation {
    Named(GraphQLNamedTypeAnnotation),
    List(Box<GraphQLListTypeAnnotation>),
    NonNull(Box<GraphQLNonNullTypeAnnotation>),
}

impl GraphQLTypeAnnotation {
    pub fn inner(&self) -> EntityName {
        match self {
            GraphQLTypeAnnotation::Named(named) => named.0,
            GraphQLTypeAnnotation::List(list) => list.0.item.inner(),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.inner(),
        }
    }

    pub fn is_nullable(&self) -> bool {
        matches!(
            self,
            GraphQLTypeAnnotation::Named(_) | GraphQLTypeAnnotation::List(_)
        )
    }
}

// Should this impl Display??
impl fmt::Display for GraphQLTypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLTypeAnnotation::Named(named) => named.fmt(f),
            GraphQLTypeAnnotation::List(list) => list.fmt(f),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLNonNullTypeAnnotation {
    Named(GraphQLNamedTypeAnnotation),
    List(GraphQLListTypeAnnotation),
}

impl GraphQLNonNullTypeAnnotation {
    pub fn inner(&self) -> EntityName {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => named.0,
            GraphQLNonNullTypeAnnotation::List(list) => list.0.item.inner(),
        }
    }
}

impl fmt::Display for GraphQLNonNullTypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => f.write_fmt(format_args!("{named}!")),
            GraphQLNonNullTypeAnnotation::List(list) => f.write_fmt(format_args!("{list}!")),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLNamedTypeAnnotation(pub EntityName);

impl Deref for GraphQLNamedTypeAnnotation {
    type Target = EntityName;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TODO should we impl Display here??
impl fmt::Display for GraphQLNamedTypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLListTypeAnnotation(pub WithEmbeddedLocation<GraphQLTypeAnnotation>);

impl Deref for GraphQLListTypeAnnotation {
    type Target = WithEmbeddedLocation<GraphQLTypeAnnotation>;

    fn deref(&self) -> &Self::Target {
        self.0.reference()
    }
}

// TODO should we impl Display here??
impl fmt::Display for GraphQLListTypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("[{}]", self.0.item))
    }
}
