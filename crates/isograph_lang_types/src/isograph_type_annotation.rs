#![allow(unused)]

use std::{
    collections::{BTreeSet, btree_set::Union},
    fmt::Debug,
};

use common_lang_types::{Span, WithSpan};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation,
};

/// This is annoying! We should find a better way to model lists.
/// This gets us closer to a good solution, so it's fine.
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Debug)]
pub enum TypeAnnotation<TInner> {
    Scalar(TInner),
    Union(UnionTypeAnnotation<TInner>),
    Plural(Box<TypeAnnotation<TInner>>),
}

impl<TInner: Ord> TypeAnnotation<TInner> {
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

    pub fn as_ref(&self) -> TypeAnnotation<&TInner> {
        match self {
            TypeAnnotation::Scalar(s) => TypeAnnotation::Scalar(s),
            TypeAnnotation::Union(union_type_annotation) => {
                TypeAnnotation::Union(union_type_annotation.as_ref())
            }
            TypeAnnotation::Plural(type_annotation) => {
                TypeAnnotation::Plural(Box::new(TypeAnnotation::as_ref(type_annotation)))
            }
        }
    }
}

impl<TInner: Ord> TypeAnnotation<TInner> {
    pub fn inner(&self) -> &TInner {
        match self {
            TypeAnnotation::Scalar(s) => s,
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.inner(),
        }
    }

    pub fn into_inner(self) -> TInner {
        match self {
            TypeAnnotation::Scalar(s) => s,
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.into_inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.into_inner(),
        }
    }

    // TODO this function should not exist, as we should not be treating "null" as special,
    // ideally
    pub fn inner_non_null(&self) -> &TInner {
        match self {
            TypeAnnotation::Scalar(s) => s,
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.inner_non_null(),
        }
    }

    pub fn map<TInner2: Ord>(
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

    // TODO implement as_ref
}

#[derive(Default, Ord, PartialEq, PartialOrd, Eq, Clone, Debug)]
pub struct UnionTypeAnnotation<TInner> {
    pub variants: BTreeSet<UnionVariant<TInner>>,
    // TODO this is incredibly hacky. null should be in the variants set, but
    // that doesn't work for a variety of reasons, namely mapping, etc.
    pub nullable: bool,
}

impl<TInner: Ord> UnionTypeAnnotation<TInner> {
    pub fn new_nullable(variant: UnionVariant<TInner>) -> Self {
        let mut variants = BTreeSet::new();
        variants.insert(variant);
        UnionTypeAnnotation {
            variants,
            nullable: true,
        }
    }

    pub fn inner(&self) -> &TInner {
        if let Some(item) = self.variants.first() {
            match item {
                UnionVariant::Scalar(s) => s,
                UnionVariant::Plural(type_annotation) => type_annotation.inner_non_null(),
            }
        } else {
            panic!("Expected self.variants to not be empty");
        }
    }

    pub fn into_inner(self) -> TInner {
        if let Some(item) = self.variants.into_iter().next() {
            match item {
                UnionVariant::Scalar(s) => s,
                UnionVariant::Plural(type_annotation) => type_annotation.into_inner(),
            }
        } else {
            panic!("Expected self.variants to not be empty");
        }
    }

    pub fn as_ref(&self) -> UnionTypeAnnotation<&TInner> {
        UnionTypeAnnotation {
            variants: self
                .variants
                .iter()
                .map(|union_variant| match union_variant {
                    UnionVariant::Scalar(scalar) => UnionVariant::Scalar(scalar),
                    UnionVariant::Plural(type_annotation) => {
                        UnionVariant::Plural(type_annotation.as_ref())
                    }
                })
                .collect(),
            nullable: self.nullable,
        }
    }
}

#[derive(Ord, PartialEq, PartialOrd, Eq, Clone, Debug)]
pub enum UnionVariant<TInner> {
    Scalar(TInner),
    Plural(TypeAnnotation<TInner>),
}

fn graphql_type_annotation_from_union_variant<TValue: Ord + Copy + Debug>(
    union_type_annotation: &UnionTypeAnnotation<TValue>,
) -> GraphQLTypeAnnotation<TValue> {
    if union_type_annotation.nullable {
        return match union_type_annotation.variants.iter().next().unwrap() {
            UnionVariant::Scalar(scalar_entity_name) => {
                GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                    *scalar_entity_name,
                    Span::todo_generated(),
                )))
            }
            UnionVariant::Plural(type_annotation) => {
                GraphQLTypeAnnotation::List(Box::new(GraphQLListTypeAnnotation(
                    graphql_type_annotation_from_type_annotation(type_annotation),
                )))
            }
        };
    }

    GraphQLTypeAnnotation::NonNull(
        match union_type_annotation.variants.iter().next().unwrap() {
            UnionVariant::Scalar(scalar_entity_name) => Box::new(
                GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                    *scalar_entity_name,
                    Span::todo_generated(),
                ))),
            ),
            UnionVariant::Plural(type_annotation) => Box::new(GraphQLNonNullTypeAnnotation::List(
                GraphQLListTypeAnnotation(graphql_type_annotation_from_type_annotation(
                    type_annotation,
                )),
            )),
        },
    )
}

pub fn graphql_type_annotation_from_type_annotation<TValue: Ord + Copy + Debug>(
    other: &TypeAnnotation<TValue>,
) -> GraphQLTypeAnnotation<TValue> {
    match other {
        TypeAnnotation::Scalar(scalar_entity_name) => GraphQLTypeAnnotation::Named(
            GraphQLNamedTypeAnnotation(WithSpan::new(*scalar_entity_name, Span::todo_generated())),
        ),
        TypeAnnotation::Plural(type_annotation) => {
            GraphQLTypeAnnotation::List(Box::new(GraphQLListTypeAnnotation(
                graphql_type_annotation_from_type_annotation(type_annotation),
            )))
        }
        TypeAnnotation::Union(union_type_annotation) => {
            graphql_type_annotation_from_union_variant(union_type_annotation)
        }
    }
}
