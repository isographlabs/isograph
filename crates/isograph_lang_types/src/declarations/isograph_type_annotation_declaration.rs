#![allow(unused)]

use std::{
    collections::{BTreeSet, btree_set::Union},
    fmt::Debug,
};

use common_lang_types::{
    EmbeddedLocation, EntityName, Span, TextSource, WithEmbeddedLocation, WithLocationPostfix,
    WithSpan, WithSpanPostfix,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation,
};
use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};

use crate::{
    ClientPointerDeclarationPath, EntityNameWrapper, IsographResolvedNode, VariableDeclarationPath,
};

/// This is annoying! We should find a better way to model lists.
/// This gets us closer to a good solution, so it's fine.
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Debug, Hash)]
pub enum TypeAnnotationDeclaration {
    Scalar(EntityNameWrapper),
    Union(UnionTypeAnnotationDeclaration),
    Plural(Box<WithEmbeddedLocation<TypeAnnotationDeclaration>>),
}

impl std::fmt::Display for TypeAnnotationDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeAnnotationDeclaration::Scalar(entity_name_wrapper) => {
                write!(f, "{}", entity_name_wrapper)
            }
            TypeAnnotationDeclaration::Union(union_type_annotation) => {
                write!(f, "{}", union_type_annotation)
            }
            TypeAnnotationDeclaration::Plural(plural) => write!(f, "[{}]", &plural.item),
        }
    }
}

impl ResolvePosition for TypeAnnotationDeclaration {
    type Parent<'a> = TypeAnnotationDeclarationParentType<'a>;

    type ResolvedNode<'a> = IsographResolvedNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, _position: Span) -> Self::ResolvedNode<'a> {
        // Note: we are implementing this manually because EntityNameWrapper and UnionTypeAnnotation
        // are not wrapped in WithEmbeddedLocation, and it would be a hassle to modify the parser
        // to do so (and of limited immediate value.)
        //
        // But we should eventually do just that!
        Self::ResolvedNode::TypeAnnotation(self.path(parent))
    }
}

#[derive(Debug)]
pub enum TypeAnnotationDeclarationParentType<'a> {
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    VariableDeclarationInner(VariableDeclarationPath<'a>),
}

pub type TypeAnnotationDeclarationPath<'a> =
    PositionResolutionPath<&'a TypeAnnotationDeclaration, TypeAnnotationDeclarationParentType<'a>>;

impl TypeAnnotationDeclaration {
    pub fn from_graphql_type_annotation(other: GraphQLTypeAnnotation) -> Self {
        match other {
            GraphQLTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotationDeclaration::Union(UnionTypeAnnotationDeclaration::new_nullable(
                    UnionVariant::Scalar(named_type_annotation.0.into()),
                ))
            }
            GraphQLTypeAnnotation::List(list_type_annotation) => {
                let inner = (*list_type_annotation)
                    .0
                    .map(TypeAnnotationDeclaration::from_graphql_type_annotation);

                TypeAnnotationDeclaration::Union(UnionTypeAnnotationDeclaration::new_nullable(
                    UnionVariant::Plural(inner),
                ))
            }
            GraphQLTypeAnnotation::NonNull(non_null_type_annotation) => {
                TypeAnnotationDeclaration::from_non_null_type_annotation(*non_null_type_annotation)
            }
        }
    }

    pub fn from_non_null_type_annotation(other: GraphQLNonNullTypeAnnotation) -> Self {
        match other {
            GraphQLNonNullTypeAnnotation::Named(named_type_annotation) => {
                TypeAnnotationDeclaration::Scalar(named_type_annotation.0.into())
            }
            GraphQLNonNullTypeAnnotation::List(list_type_annotation) => {
                let inner = list_type_annotation
                    .0
                    .map(TypeAnnotationDeclaration::from_graphql_type_annotation);
                TypeAnnotationDeclaration::Plural(inner.boxed())
            }
        }
    }
}

impl TypeAnnotationDeclaration {
    pub fn inner(&self) -> EntityNameWrapper {
        match self {
            TypeAnnotationDeclaration::Scalar(s) => s.dereference(),
            TypeAnnotationDeclaration::Union(union_type_annotation) => {
                union_type_annotation.inner()
            }
            TypeAnnotationDeclaration::Plural(type_annotation) => type_annotation.item.inner(),
        }
    }

    // TODO this function should not exist, as we should not be treating "null" as special,
    // ideally
    pub fn inner_non_null(&self) -> EntityNameWrapper {
        match self {
            TypeAnnotationDeclaration::Scalar(s) => s.dereference(),
            TypeAnnotationDeclaration::Union(union_type_annotation) => {
                union_type_annotation.inner()
            }
            TypeAnnotationDeclaration::Plural(type_annotation) => {
                type_annotation.item.inner_non_null()
            }
        }
    }

    pub fn is_nullable(&self) -> bool {
        // TODO this will have to change at some point, but for now, a Union is only used
        // to represent nullability.
        match self {
            TypeAnnotationDeclaration::Scalar(entity_name_wrapper) => false,
            TypeAnnotationDeclaration::Union(union_type_annotation) => {
                union_type_annotation.nullable
            }
            TypeAnnotationDeclaration::Plural(_) => false,
        }
    }
}

#[derive(Default, Ord, PartialEq, PartialOrd, Eq, Clone, Debug, Hash)]
pub struct UnionTypeAnnotationDeclaration {
    pub variants: BTreeSet<UnionVariant>,
    // TODO this is incredibly hacky. null should be in the variants set, but
    // that doesn't work for a variety of reasons, namely mapping, etc.
    pub nullable: bool,
}

impl std::fmt::Display for UnionTypeAnnotationDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let count = self.variants.len();
        for (index, variant) in self.variants.iter().enumerate() {
            let add_pipe = self.nullable || (index != count - 1);
            write!(f, "{}", variant)?;
            if add_pipe {
                write!(f, " | ")?;
            }
        }
        if self.nullable {
            write!(f, "null")?;
        }
        write!(f, ")")
    }
}

impl UnionTypeAnnotationDeclaration {
    pub fn new_nullable(variant: UnionVariant) -> Self {
        let mut variants = BTreeSet::new();
        variants.insert(variant);
        UnionTypeAnnotationDeclaration {
            variants,
            nullable: true,
        }
    }

    pub fn inner(&self) -> EntityNameWrapper {
        if let Some(item) = self.variants.first() {
            match item {
                UnionVariant::Scalar(s) => s.dereference(),
                UnionVariant::Plural(type_annotation) => type_annotation.item.inner_non_null(),
            }
        } else {
            panic!("Expected self.variants to not be empty");
        }
    }
}

#[derive(Ord, PartialEq, PartialOrd, Eq, Clone, Debug, Hash)]
pub enum UnionVariant {
    Scalar(EntityNameWrapper),
    Plural(WithEmbeddedLocation<TypeAnnotationDeclaration>),
}

impl std::fmt::Display for UnionVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnionVariant::Scalar(entity_name_wrapper) => write!(f, "{}", entity_name_wrapper),
            UnionVariant::Plural(plural) => write!(f, "{}", &plural.item),
        }
    }
}

fn graphql_type_annotation_from_union_variant(
    union_type_annotation: &UnionTypeAnnotationDeclaration,
) -> GraphQLTypeAnnotation {
    if union_type_annotation.nullable {
        return match union_type_annotation.variants.iter().next().unwrap() {
            UnionVariant::Scalar(scalar_entity_name) => {
                GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(scalar_entity_name.0))
            }
            UnionVariant::Plural(type_annotation) => GraphQLTypeAnnotation::List(
                GraphQLListTypeAnnotation(
                    type_annotation
                        .as_ref()
                        .map(graphql_type_annotation_from_type_annotation),
                )
                .boxed(),
            ),
        };
    }

    GraphQLTypeAnnotation::NonNull(
        match union_type_annotation.variants.iter().next().unwrap() {
            UnionVariant::Scalar(scalar_entity_name) => GraphQLNonNullTypeAnnotation::Named(
                GraphQLNamedTypeAnnotation(scalar_entity_name.0),
            )
            .boxed(),
            UnionVariant::Plural(type_annotation) => {
                GraphQLNonNullTypeAnnotation::List(GraphQLListTypeAnnotation(
                    type_annotation
                        .as_ref()
                        .map(graphql_type_annotation_from_type_annotation),
                ))
                .boxed()
            }
        },
    )
}

pub fn graphql_type_annotation_from_type_annotation(
    other: &TypeAnnotationDeclaration,
) -> GraphQLTypeAnnotation {
    match other {
        TypeAnnotationDeclaration::Scalar(scalar_entity_name) => GraphQLTypeAnnotation::NonNull(
            GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(scalar_entity_name.0))
                .boxed(),
        ),
        TypeAnnotationDeclaration::Plural(type_annotation) => GraphQLTypeAnnotation::List(
            GraphQLListTypeAnnotation(
                type_annotation
                    .as_ref()
                    .as_ref()
                    .map(graphql_type_annotation_from_type_annotation),
            )
            .boxed(),
        ),
        TypeAnnotationDeclaration::Union(union_type_annotation) => {
            graphql_type_annotation_from_union_variant(union_type_annotation)
        }
    }
}
