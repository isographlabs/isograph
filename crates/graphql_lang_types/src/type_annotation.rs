use std::{fmt, ops::Deref};

use common_lang_types::WithSpan;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TypeAnnotation<TValue> {
    Named(NamedTypeAnnotation<TValue>),
    List(Box<ListTypeAnnotation<TValue>>),
    NonNull(Box<NonNullTypeAnnotation<TValue>>),
}

impl<TValue> TypeAnnotation<TValue> {
    pub fn inner(&self) -> &TValue {
        match self {
            TypeAnnotation::Named(named) => &named.0.item,
            TypeAnnotation::List(list) => list.0.inner(),
            TypeAnnotation::NonNull(non_null) => non_null.inner(),
        }
    }

    pub fn map<F, TNewValue>(self, f: F) -> TypeAnnotation<TNewValue>
    where
        F: FnOnce(TValue) -> TNewValue,
    {
        match self {
            TypeAnnotation::Named(named) => TypeAnnotation::Named(NamedTypeAnnotation(
                WithSpan::new(f(named.0.item), named.0.span),
            )),
            TypeAnnotation::List(list) => TypeAnnotation::List(Box::new(list.map(f))),
            TypeAnnotation::NonNull(non_null) => TypeAnnotation::NonNull(Box::new(non_null.map(f))),
        }
    }

    pub fn and_then<F, TNewValue, E>(self, f: F) -> Result<TypeAnnotation<TNewValue>, E>
    where
        F: FnOnce(TValue) -> Result<TNewValue, E>,
    {
        Ok(match self {
            TypeAnnotation::Named(named) => TypeAnnotation::Named(NamedTypeAnnotation(
                WithSpan::new(f(named.0.item)?, named.0.span),
            )),
            TypeAnnotation::List(list) => TypeAnnotation::List(Box::new(list.and_then(f)?)),
            TypeAnnotation::NonNull(non_null) => {
                TypeAnnotation::NonNull(Box::new(non_null.and_then(f)?))
            }
        })
    }
}

impl<TValue: fmt::Display> fmt::Display for TypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Named(named) => named.fmt(f),
            TypeAnnotation::List(list) => list.fmt(f),
            TypeAnnotation::NonNull(non_null) => non_null.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonNullTypeAnnotation<TValue> {
    Named(NamedTypeAnnotation<TValue>),
    List(ListTypeAnnotation<TValue>),
}

impl<TValue> NonNullTypeAnnotation<TValue> {
    pub fn inner(&self) -> &TValue {
        match self {
            NonNullTypeAnnotation::Named(named) => &named.0.item,
            NonNullTypeAnnotation::List(list) => list.0.inner(),
        }
    }

    pub fn map<F, TNewValue>(self, f: F) -> NonNullTypeAnnotation<TNewValue>
    where
        F: FnOnce(TValue) -> TNewValue,
    {
        match self {
            NonNullTypeAnnotation::Named(named) => NonNullTypeAnnotation::Named(
                NamedTypeAnnotation(WithSpan::new(f(named.0.item), named.0.span)),
            ),
            NonNullTypeAnnotation::List(list) => NonNullTypeAnnotation::List(list.map(f)),
        }
    }

    pub fn and_then<F, TNewValue, E>(self, f: F) -> Result<NonNullTypeAnnotation<TNewValue>, E>
    where
        F: FnOnce(TValue) -> Result<TNewValue, E>,
    {
        Ok(match self {
            NonNullTypeAnnotation::Named(named) => NonNullTypeAnnotation::Named(
                NamedTypeAnnotation(WithSpan::new(f(named.0.item)?, named.0.span)),
            ),
            NonNullTypeAnnotation::List(list) => NonNullTypeAnnotation::List(list.and_then(f)?),
        })
    }
}

impl<TValue: fmt::Display> fmt::Display for NonNullTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonNullTypeAnnotation::Named(named) => f.write_fmt(format_args!("{}!", named)),
            NonNullTypeAnnotation::List(list) => f.write_fmt(format_args!("{}!", list)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NamedTypeAnnotation<TValue>(pub WithSpan<TValue>);

impl<TValue> Deref for NamedTypeAnnotation<TValue> {
    type Target = WithSpan<TValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TValue: fmt::Display> fmt::Display for NamedTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ListTypeAnnotation<TValue>(pub TypeAnnotation<TValue>);

impl<TValue> ListTypeAnnotation<TValue> {
    pub fn map<F, TNewValue>(self, f: F) -> ListTypeAnnotation<TNewValue>
    where
        F: FnOnce(TValue) -> TNewValue,
    {
        ListTypeAnnotation(self.0.map(f))
    }

    pub fn and_then<F, TNewValue, E>(self, f: F) -> Result<ListTypeAnnotation<TNewValue>, E>
    where
        F: FnOnce(TValue) -> Result<TNewValue, E>,
    {
        Ok(ListTypeAnnotation(self.0.and_then(f)?))
    }
}

impl<TValue> Deref for ListTypeAnnotation<TValue> {
    type Target = TypeAnnotation<TValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TValue: fmt::Display> fmt::Display for ListTypeAnnotation<TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("[{}]", self.0))
    }
}
