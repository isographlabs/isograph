#![allow(unused)]

use std::collections::BTreeSet;

use common_lang_types::WithSpan;
use graphql_lang_types::{GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation};

/// This is annoying! We should find a better way to model lists.
/// This gets us closer to a good solution, so it's fine.
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum TypeAnnotation<TInner: Ord + Copy> {
    Scalar(TInner),
    Union(UnionTypeAnnotation<TInner>),
    Plural(Box<TypeAnnotation<TInner>>),
}

impl<TInner: Ord + Copy> TypeAnnotation<WithSpan<TInner>> {
    pub fn from_graphql_type_annotation(
        other: GraphQLTypeAnnotation<TInner>,
        null: WithSpan<TInner>,
    ) -> Self {
        match other {
            GraphQLTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotation::Union(UnionTypeAnnotation::new_nullable(
                    UnionVariant::Scalar(named_type_annotation.0),
                    null,
                ))
            }
            GraphQLTypeAnnotation::List(list_type_annotation) => {
                let inner =
                    TypeAnnotation::from_graphql_type_annotation((*list_type_annotation).0, null);
                TypeAnnotation::Union(UnionTypeAnnotation::new_nullable(
                    UnionVariant::Plural(inner),
                    null,
                ))
            }
            GraphQLTypeAnnotation::NonNull(non_null_type_annotation) => {
                TypeAnnotation::from_non_null_type_annotation(non_null_type_annotation, null)
            }
        }
    }

    pub fn from_non_null_type_annotation(
        other: Box<GraphQLNonNullTypeAnnotation<TInner>>,
        null: WithSpan<TInner>,
    ) -> Self {
        match *other {
            GraphQLNonNullTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotation::Scalar(named_type_annotation.0)
            }
            GraphQLNonNullTypeAnnotation::List(list_type_annotation) => {
                let inner =
                    TypeAnnotation::from_graphql_type_annotation(list_type_annotation.0, null);
                TypeAnnotation::Plural(Box::new(inner))
            }
        }
    }
}

#[derive(Debug, Default, Ord, PartialEq, PartialOrd, Eq)]
pub struct UnionTypeAnnotation<TInner: Ord + Copy> {
    pub variants: BTreeSet<UnionVariant<TInner>>,
}

impl<TInner: Ord + Copy> UnionTypeAnnotation<TInner> {
    pub fn new_nullable(variant: UnionVariant<TInner>, null: TInner) -> Self {
        let mut variants = BTreeSet::new();
        variants.insert(variant);
        variants.insert(UnionVariant::Scalar(null));
        UnionTypeAnnotation { variants }
    }

    pub fn merge(&mut self, other: Self) {
        self.variants.extend(other.variants);
    }
}

#[derive(Debug, Ord, PartialEq, PartialOrd, Eq)]
pub enum UnionVariant<TInner: Ord + Copy> {
    Scalar(TInner),
    Plural(TypeAnnotation<TInner>),
}
