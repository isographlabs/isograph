use std::{fmt, ops::Deref};

use common_lang_types::{ValidTypeAnnotationInnerType, WithSpan};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TypeAnnotation<T: ValidTypeAnnotationInnerType> {
    Named(NamedTypeAnnotation<T>),
    List(Box<ListTypeAnnotation<T>>),
    NonNull(Box<NonNullTypeAnnotation<T>>),
}

impl<T: ValidTypeAnnotationInnerType> TypeAnnotation<T> {
    pub fn inner(&self) -> &T {
        match self {
            TypeAnnotation::Named(named) => &named.0.item,
            TypeAnnotation::List(list) => list.0.inner(),
            TypeAnnotation::NonNull(non_null) => non_null.inner(),
        }
    }
}

impl<T: ValidTypeAnnotationInnerType + fmt::Display> fmt::Display for TypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Named(named) => named.fmt(f),
            TypeAnnotation::List(list) => list.fmt(f),
            TypeAnnotation::NonNull(non_null) => non_null.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonNullTypeAnnotation<T: ValidTypeAnnotationInnerType> {
    Named(NamedTypeAnnotation<T>),
    List(ListTypeAnnotation<T>),
}

impl<T: ValidTypeAnnotationInnerType> NonNullTypeAnnotation<T> {
    pub fn inner(&self) -> &T {
        match self {
            NonNullTypeAnnotation::Named(named) => &named.0.item,
            NonNullTypeAnnotation::List(list) => list.0.inner(),
        }
    }
}

impl<T: ValidTypeAnnotationInnerType + fmt::Display> fmt::Display for NonNullTypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonNullTypeAnnotation::Named(named) => f.write_fmt(format_args!("{}!", named)),
            NonNullTypeAnnotation::List(list) => f.write_fmt(format_args!("{}!", list)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NamedTypeAnnotation<T: ValidTypeAnnotationInnerType>(pub WithSpan<T>);

impl<T: ValidTypeAnnotationInnerType> Deref for NamedTypeAnnotation<T> {
    type Target = WithSpan<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ValidTypeAnnotationInnerType + fmt::Display> fmt::Display for NamedTypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ListTypeAnnotation<T: ValidTypeAnnotationInnerType>(pub TypeAnnotation<T>);

impl<T: ValidTypeAnnotationInnerType> Deref for ListTypeAnnotation<T> {
    type Target = TypeAnnotation<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ValidTypeAnnotationInnerType + fmt::Display> fmt::Display for ListTypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("[{}]", self.0))
    }
}
