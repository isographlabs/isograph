use std::{fmt, ops::Deref};

use intern::{string_key::StringKey, Lookup};

use super::WithSpan;

/// A trait for designating a given type as representing a type.
/// So for example, TypeAnnotation is generic over T: TypeTrait,
/// so you can validly create a TypeAnnotation<OutputTypeName>
/// if the type annotation must refer to an output type.
pub trait TypeTrait: fmt::Display + Lookup + From<StringKey> {}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TypeAnnotation<T: TypeTrait> {
    Named(NamedTypeAnnotation<T>),
    List(Box<ListTypeAnnotation<T>>),
    NonNull(Box<NonNullTypeAnnotation<T>>),
}

impl<T: TypeTrait> fmt::Display for TypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Named(named) => named.fmt(f),
            TypeAnnotation::List(list) => list.fmt(f),
            TypeAnnotation::NonNull(non_null) => non_null.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonNullTypeAnnotation<T: TypeTrait> {
    Named(NamedTypeAnnotation<T>),
    List(ListTypeAnnotation<T>),
}

impl<T: TypeTrait> fmt::Display for NonNullTypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonNullTypeAnnotation::Named(named) => f.write_fmt(format_args!("{}!", named)),
            NonNullTypeAnnotation::List(list) => f.write_fmt(format_args!("{}!", list)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NamedTypeAnnotation<T: TypeTrait>(pub WithSpan<T>);

impl<T: TypeTrait> Deref for NamedTypeAnnotation<T> {
    type Target = WithSpan<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TypeTrait> fmt::Display for NamedTypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ListTypeAnnotation<T: TypeTrait>(pub TypeAnnotation<T>);

impl<T: TypeTrait> Deref for ListTypeAnnotation<T> {
    type Target = TypeAnnotation<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TypeTrait> fmt::Display for ListTypeAnnotation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("[{}]", self.0))
    }
}
