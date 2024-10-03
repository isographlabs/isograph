use std::{fmt, ops::Deref};

use common_lang_types::WithSpan;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLTypeAnnotation<TValue> {
    Named(GraphQLNamedTypeAnnotation<TValue>),
    List(Box<GraphQLListTypeAnnotation<TValue>>),
    NonNull(Box<GraphQLNonNullTypeAnnotation<TValue>>),
}

impl<TValue> GraphQLTypeAnnotation<TValue> {
    pub fn inner(&self) -> &TValue {
        match self {
            GraphQLTypeAnnotation::Named(named) => &named.0.item,
            GraphQLTypeAnnotation::List(list) => list.0.inner(),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.inner(),
        }
    }

    pub fn inner_mut(&mut self) -> &mut TValue {
        match self {
            GraphQLTypeAnnotation::Named(named) => &mut named.0.item,
            GraphQLTypeAnnotation::List(list) => list.0.inner_mut(),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.inner_mut(),
        }
    }

    pub fn map<F, TNewValue>(self, f: F) -> GraphQLTypeAnnotation<TNewValue>
    where
        F: FnOnce(TValue) -> TNewValue,
    {
        match self {
            GraphQLTypeAnnotation::Named(named) => GraphQLTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(WithSpan::new(f(named.0.item), named.0.span)),
            ),
            GraphQLTypeAnnotation::List(list) => GraphQLTypeAnnotation::List(Box::new(list.map(f))),
            GraphQLTypeAnnotation::NonNull(non_null) => {
                GraphQLTypeAnnotation::NonNull(Box::new(non_null.map(f)))
            }
        }
    }

    pub fn and_then<F, TNewValue, E>(self, f: F) -> Result<GraphQLTypeAnnotation<TNewValue>, E>
    where
        F: FnOnce(TValue) -> Result<TNewValue, E>,
    {
        Ok(match self {
            GraphQLTypeAnnotation::Named(named) => GraphQLTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(WithSpan::new(f(named.0.item)?, named.0.span)),
            ),
            GraphQLTypeAnnotation::List(list) => {
                GraphQLTypeAnnotation::List(Box::new(list.and_then(f)?))
            }
            GraphQLTypeAnnotation::NonNull(non_null) => {
                GraphQLTypeAnnotation::NonNull(Box::new(non_null.and_then(f)?))
            }
        })
    }

    /// If a TypeAnnotation is of the form X!, i.e. it is a NonNull named type, then
    /// this method returns Some(X). Otherwise, returns None.
    pub fn inner_non_null_named_type(&self) -> Option<&GraphQLNamedTypeAnnotation<TValue>> {
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

impl<TValue: fmt::Display> fmt::Display for GraphQLTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLTypeAnnotation::Named(named) => named.fmt(f),
            GraphQLTypeAnnotation::List(list) => list.fmt(f),
            GraphQLTypeAnnotation::NonNull(non_null) => non_null.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLNonNullTypeAnnotation<TValue> {
    Named(GraphQLNamedTypeAnnotation<TValue>),
    List(GraphQLListTypeAnnotation<TValue>),
}

impl<TValue> GraphQLNonNullTypeAnnotation<TValue> {
    pub fn inner(&self) -> &TValue {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => &named.0.item,
            GraphQLNonNullTypeAnnotation::List(list) => list.0.inner(),
        }
    }

    pub fn inner_mut(&mut self) -> &mut TValue {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => &mut named.0.item,
            GraphQLNonNullTypeAnnotation::List(list) => list.0.inner_mut(),
        }
    }

    pub fn map<F, TNewValue>(self, f: F) -> GraphQLNonNullTypeAnnotation<TNewValue>
    where
        F: FnOnce(TValue) -> TNewValue,
    {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => GraphQLNonNullTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(WithSpan::new(f(named.0.item), named.0.span)),
            ),
            GraphQLNonNullTypeAnnotation::List(list) => {
                GraphQLNonNullTypeAnnotation::List(list.map(f))
            }
        }
    }

    pub fn and_then<F, TNewValue, E>(
        self,
        f: F,
    ) -> Result<GraphQLNonNullTypeAnnotation<TNewValue>, E>
    where
        F: FnOnce(TValue) -> Result<TNewValue, E>,
    {
        Ok(match self {
            GraphQLNonNullTypeAnnotation::Named(named) => GraphQLNonNullTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(WithSpan::new(f(named.0.item)?, named.0.span)),
            ),
            GraphQLNonNullTypeAnnotation::List(list) => {
                GraphQLNonNullTypeAnnotation::List(list.and_then(f)?)
            }
        })
    }
}

impl<TValue: fmt::Display> fmt::Display for GraphQLNonNullTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLNonNullTypeAnnotation::Named(named) => f.write_fmt(format_args!("{}!", named)),
            GraphQLNonNullTypeAnnotation::List(list) => f.write_fmt(format_args!("{}!", list)),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLNamedTypeAnnotation<TValue>(pub WithSpan<TValue>);

impl<TValue> Deref for GraphQLNamedTypeAnnotation<TValue> {
    type Target = WithSpan<TValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TValue: fmt::Display> fmt::Display for GraphQLNamedTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLListTypeAnnotation<TValue>(pub GraphQLTypeAnnotation<TValue>);

impl<TValue> GraphQLListTypeAnnotation<TValue> {
    pub fn map<F, TNewValue>(self, f: F) -> GraphQLListTypeAnnotation<TNewValue>
    where
        F: FnOnce(TValue) -> TNewValue,
    {
        GraphQLListTypeAnnotation(self.0.map(f))
    }

    pub fn and_then<F, TNewValue, E>(self, f: F) -> Result<GraphQLListTypeAnnotation<TNewValue>, E>
    where
        F: FnOnce(TValue) -> Result<TNewValue, E>,
    {
        Ok(GraphQLListTypeAnnotation(self.0.and_then(f)?))
    }
}

impl<TValue> Deref for GraphQLListTypeAnnotation<TValue> {
    type Target = GraphQLTypeAnnotation<TValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TValue: fmt::Display> fmt::Display for GraphQLListTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("[{}]", self.0))
    }
}
