#![allow(unused)]

use std::{collections::BTreeSet, fmt::Debug};

use common_lang_types::WithSpan;
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};

/// This is annoying! We should find a better way to model lists.
/// This gets us closer to a good solution, so it's fine.
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Debug)]
pub enum TypeAnnotation<TInner: Ord + Debug> {
    Scalar(TInner),
    Union(UnionTypeAnnotation<TInner>),
    Plural(Box<TypeAnnotation<TInner>>),
}

impl<TInner: Ord + Copy + Debug> TypeAnnotation<TInner> {
    pub fn from_graphql_type_annotation(other: GraphQLTypeAnnotation<TInner>) -> Self {
        match other {
            GraphQLTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotation::Union(UnionTypeAnnotation::new_nullable(UnionVariant::Scalar(
                    named_type_annotation.0.item,
                )))
            }
            GraphQLTypeAnnotation::List(list_type_annotation) => {
                let inner = TypeAnnotation::from_graphql_type_annotation((*list_type_annotation).0);
                TypeAnnotation::Union(UnionTypeAnnotation::new_nullable(UnionVariant::Plural(
                    inner,
                )))
            }
            GraphQLTypeAnnotation::NonNull(non_null_type_annotation) => {
                TypeAnnotation::from_non_null_type_annotation(*non_null_type_annotation)
            }
        }
    }

    pub fn from_non_null_type_annotation(other: GraphQLNonNullTypeAnnotation<TInner>) -> Self {
        match other {
            GraphQLNonNullTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotation::Scalar(named_type_annotation.0.item)
            }
            GraphQLNonNullTypeAnnotation::List(list_type_annotation) => {
                let inner = TypeAnnotation::from_graphql_type_annotation(list_type_annotation.0);
                TypeAnnotation::Plural(Box::new(inner))
            }
        }
    }
}

impl<TInner: Ord + Copy + Debug> TypeAnnotation<TInner> {
    pub fn inner(&self) -> TInner {
        match self {
            TypeAnnotation::Scalar(s) => *s,
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.inner(),
        }
    }

    // TODO this function should not exist, as we should not be treating "null" as special,
    // ideally
    pub fn inner_non_null(&self) -> TInner {
        match self {
            TypeAnnotation::Scalar(s) => *s,
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.inner_non_null(),
        }
    }
}

impl<TInner: Ord + Debug> TypeAnnotation<TInner> {
    pub fn map<TInner2: Ord + Debug>(
        self,
        map: &mut impl FnMut(TInner) -> TInner2,
    ) -> TypeAnnotation<TInner2> {
        match self {
            TypeAnnotation::Scalar(s) => TypeAnnotation::Scalar(map(s)),
            TypeAnnotation::Union(union_type_annotation) => {
                TypeAnnotation::Union(UnionTypeAnnotation {
                    variants: union_type_annotation
                        .variants
                        .into_iter()
                        .map(|x| match x {
                            UnionVariant::Scalar(s) => UnionVariant::Scalar(map(s)),
                            UnionVariant::Plural(type_annotation) => {
                                UnionVariant::Plural(type_annotation.map(map))
                            }
                        })
                        .collect(),
                    nullable: union_type_annotation.nullable,
                })
            }
            TypeAnnotation::Plural(type_annotation) => {
                TypeAnnotation::Plural(Box::new(type_annotation.map(map)))
            }
        }
    }
}

#[derive(Default, Ord, PartialEq, PartialOrd, Eq, Clone, Debug)]
pub struct UnionTypeAnnotation<TInner: Ord + Debug> {
    pub variants: BTreeSet<UnionVariant<TInner>>,
    // TODO this is incredibly hacky. null should be in the variants set, but
    // that doesn't work for a variety of reasons, namely mapping, etc.
    pub nullable: bool,
}

impl<TInner: Ord + Copy + Debug> UnionTypeAnnotation<TInner> {
    pub fn new_nullable(variant: UnionVariant<TInner>) -> Self {
        let mut variants = BTreeSet::new();
        variants.insert(variant);
        UnionTypeAnnotation {
            variants,
            nullable: true,
        }
    }

    pub fn inner(&self) -> TInner {
        if let Some(item) = self.variants.first() {
            match item {
                UnionVariant::Scalar(s) => return *s,
                UnionVariant::Plural(type_annotation) => return type_annotation.inner_non_null(),
            }
        }
        panic!("Expected self.variants to not be empty");
    }
}

#[derive(Ord, PartialEq, PartialOrd, Eq, Clone, Debug)]
pub enum UnionVariant<TInner: Ord + Debug> {
    Scalar(TInner),
    Plural(TypeAnnotation<TInner>),
}
