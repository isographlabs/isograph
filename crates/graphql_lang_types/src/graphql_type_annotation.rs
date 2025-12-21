use std::{fmt, ops::Deref};

use common_lang_types::{EmbeddedLocation, EntityName, Span, WithEmbeddedLocation};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLTypeAnnotation {
    Named(GraphQLNamedTypeAnnotation),
    List(Box<GraphQLListTypeAnnotation>),
    NonNull(Box<GraphQLNonNullTypeAnnotation>),
}

impl GraphQLTypeAnnotation {
    pub fn inner(&self) -> EntityName {
        match self {
            GraphQLTypeAnnotation::Named(named) => named.0.item,
            GraphQLTypeAnnotation::List(list) => list.0.inner(),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.inner(),
        }
    }

    pub fn embedded_location(&self) -> EmbeddedLocation {
        match self {
            GraphQLTypeAnnotation::Named(named) => named.0.embedded_location,
            GraphQLTypeAnnotation::List(list) => list.0.embedded_location(),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.embedded_location(),
        }
    }

    pub fn span(&self) -> Span {
        self.embedded_location().span
    }

    /// If a TypeAnnotation is of the form X!, i.e. it is a NonNull named type, then
    /// this method returns Some(X). Otherwise, returns None.
    pub fn inner_non_null_named_type(&self) -> Option<&GraphQLNamedTypeAnnotation> {
        match self {
            GraphQLTypeAnnotation::Named(_) => None,
            GraphQLTypeAnnotation::List(_) => None,
            GraphQLTypeAnnotation::NonNull(non_null) => match non_null.as_ref() {
                GraphQLNonNullTypeAnnotation::Named(named) => Some(named),
                GraphQLNonNullTypeAnnotation::List(_) => None,
            },
        }
    }

    pub fn is_nullable(&self) -> bool {
        matches!(
            self,
            GraphQLTypeAnnotation::Named(_) | GraphQLTypeAnnotation::List(_)
        )
    }
}

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
            GraphQLNonNullTypeAnnotation::Named(named) => named.0.item,
            GraphQLNonNullTypeAnnotation::List(list) => list.0.inner(),
        }
    }

    pub fn embedded_location(&self) -> EmbeddedLocation {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => named.0.embedded_location,
            GraphQLNonNullTypeAnnotation::List(list) => list.0.embedded_location(),
        }
    }

    pub fn span(&self) -> Span {
        self.embedded_location().span
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
pub struct GraphQLNamedTypeAnnotation(pub WithEmbeddedLocation<EntityName>);

impl Deref for GraphQLNamedTypeAnnotation {
    type Target = WithEmbeddedLocation<EntityName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for GraphQLNamedTypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.item)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLListTypeAnnotation(pub GraphQLTypeAnnotation);

impl Deref for GraphQLListTypeAnnotation {
    type Target = GraphQLTypeAnnotation;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for GraphQLListTypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("[{}]", self.0))
    }
}
