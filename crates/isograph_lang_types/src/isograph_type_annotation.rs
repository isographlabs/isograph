#![allow(unused)]

use std::{
    collections::{BTreeSet, btree_set::Union},
    fmt::Debug,
};

use common_lang_types::{
    EmbeddedLocation, EntityName, Span, TextSource, WithLocationPostfix, WithSpan, WithSpanPostfix,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation,
};
use prelude::Postfix;

/// This is annoying! We should find a better way to model lists.
/// This gets us closer to a good solution, so it's fine.
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Debug, Hash)]
pub enum TypeAnnotation {
    Scalar(EntityName),
    Union(UnionTypeAnnotation),
    Plural(Box<TypeAnnotation>),
}

impl TypeAnnotation {
    pub fn from_graphql_type_annotation(other: GraphQLTypeAnnotation) -> Self {
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

    pub fn from_non_null_type_annotation(other: GraphQLNonNullTypeAnnotation) -> Self {
        match other {
            GraphQLNonNullTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotation::Scalar(named_type_annotation.0.item)
            }
            GraphQLNonNullTypeAnnotation::List(list_type_annotation) => {
                let inner = TypeAnnotation::from_graphql_type_annotation(list_type_annotation.0);
                TypeAnnotation::Plural(inner.boxed())
            }
        }
    }
}

impl TypeAnnotation {
    pub fn inner(&self) -> EntityName {
        match self {
            TypeAnnotation::Scalar(s) => s.dereference(),
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.inner(),
        }
    }

    // TODO this function should not exist, as we should not be treating "null" as special,
    // ideally
    pub fn inner_non_null(&self) -> EntityName {
        match self {
            TypeAnnotation::Scalar(s) => s.dereference(),
            TypeAnnotation::Union(union_type_annotation) => union_type_annotation.inner(),
            TypeAnnotation::Plural(type_annotation) => type_annotation.inner_non_null(),
        }
    }

    pub fn is_nullable(&self) -> bool {
        // TODO this will have to change at some point, but for now, a Union is only used
        // to represent nullability.
        matches!(self, TypeAnnotation::Union(_))
    }
}

#[derive(Default, Ord, PartialEq, PartialOrd, Eq, Clone, Debug, Hash)]
pub struct UnionTypeAnnotation {
    pub variants: BTreeSet<UnionVariant>,
    // TODO this is incredibly hacky. null should be in the variants set, but
    // that doesn't work for a variety of reasons, namely mapping, etc.
    pub nullable: bool,
}

impl UnionTypeAnnotation {
    pub fn new_nullable(variant: UnionVariant) -> Self {
        let mut variants = BTreeSet::new();
        variants.insert(variant);
        UnionTypeAnnotation {
            variants,
            nullable: true,
        }
    }

    pub fn inner(&self) -> EntityName {
        if let Some(item) = self.variants.first() {
            match item {
                UnionVariant::Scalar(s) => s.dereference(),
                UnionVariant::Plural(type_annotation) => type_annotation.inner_non_null(),
            }
        } else {
            panic!("Expected self.variants to not be empty");
        }
    }
}

#[derive(Ord, PartialEq, PartialOrd, Eq, Clone, Debug, Hash)]
pub enum UnionVariant {
    Scalar(EntityName),
    Plural(TypeAnnotation),
}

fn graphql_type_annotation_from_union_variant(
    union_type_annotation: &UnionTypeAnnotation,
) -> GraphQLTypeAnnotation {
    if union_type_annotation.nullable {
        return match union_type_annotation.variants.iter().next().unwrap() {
            UnionVariant::Scalar(scalar_entity_name) => {
                GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                    (*scalar_entity_name)
                        .with_embedded_location(EmbeddedLocation::todo_generated()),
                ))
            }
            UnionVariant::Plural(type_annotation) => GraphQLTypeAnnotation::List(
                GraphQLListTypeAnnotation(graphql_type_annotation_from_type_annotation(
                    type_annotation,
                ))
                .boxed(),
            ),
        };
    }

    GraphQLTypeAnnotation::NonNull(
        match union_type_annotation.variants.iter().next().unwrap() {
            UnionVariant::Scalar(scalar_entity_name) => {
                GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                    (*scalar_entity_name)
                        .with_embedded_location(EmbeddedLocation::todo_generated()),
                ))
                .boxed()
            }
            UnionVariant::Plural(type_annotation) => {
                GraphQLNonNullTypeAnnotation::List(GraphQLListTypeAnnotation(
                    graphql_type_annotation_from_type_annotation(type_annotation),
                ))
                .boxed()
            }
        },
    )
}

pub fn graphql_type_annotation_from_type_annotation(
    other: &TypeAnnotation,
) -> GraphQLTypeAnnotation {
    match other {
        TypeAnnotation::Scalar(scalar_entity_name) => {
            GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                (*scalar_entity_name).with_embedded_location(EmbeddedLocation::todo_generated()),
            ))
        }
        TypeAnnotation::Plural(type_annotation) => GraphQLTypeAnnotation::List(
            GraphQLListTypeAnnotation(graphql_type_annotation_from_type_annotation(
                type_annotation,
            ))
            .boxed(),
        ),
        TypeAnnotation::Union(union_type_annotation) => {
            graphql_type_annotation_from_union_variant(union_type_annotation)
        }
    }
}
