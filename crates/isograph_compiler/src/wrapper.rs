use common_lang_types::EntityName;
use isograph_parser::{ListTypeAnnotation, TypeAnnotation, UnionVariant};
use prelude::Postfix;

/// A type former over `T` (mental-model.md, Wrapper). `List` is `[W]`; `Union` is `W | ...`,
/// and a nullable `W` is `Union([W, Null])`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Wrapper<T> {
    Inner(T),
    List(Box<Wrapper<T>>),
    Union(UnionWrapper<T>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnionWrapper<T>(pub Vec<WrapperVariant<T>>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WrapperVariant<T> {
    Inner(T),
    List(Box<Wrapper<T>>),
    Null,
}

impl<T> Wrapper<T> {
    /// The same structure with every `Inner` replaced by `f` of it, or the first error `f`
    /// returns.
    pub fn try_map<U, E>(&self, f: &mut impl FnMut(&T) -> Result<U, E>) -> Result<Wrapper<U>, E> {
        match self {
            Wrapper::Inner(inner) => Wrapper::Inner(f(inner)?).wrap_ok(),
            Wrapper::List(list) => Wrapper::List(list.try_map(f)?.boxed()).wrap_ok(),
            Wrapper::Union(union) => Wrapper::Union(UnionWrapper(
                union
                    .0
                    .iter()
                    .map(|variant| {
                        match variant {
                            WrapperVariant::Inner(inner) => WrapperVariant::Inner(f(inner)?),
                            WrapperVariant::List(list) => {
                                WrapperVariant::List(list.try_map(f)?.boxed())
                            }
                            WrapperVariant::Null => WrapperVariant::Null,
                        }
                        .wrap_ok()
                    })
                    .collect::<Result<Vec<_>, E>>()?,
            ))
            .wrap_ok(),
        }
    }

    /// `Nullable` when the top level is a `Union` with a `Null` variant.
    pub fn nullability(&self) -> Nullability {
        match self {
            Wrapper::Union(union)
                if union
                    .0
                    .iter()
                    .any(|variant| matches!(variant, WrapperVariant::Null)) =>
            {
                Nullability::Nullable
            }
            Wrapper::Inner(_) | Wrapper::List(_) | Wrapper::Union(_) => Nullability::NonNull,
        }
    }

    /// Every `Inner`, in order.
    pub fn inner_values(&self) -> impl Iterator<Item = &T> {
        let values: Box<dyn Iterator<Item = &T> + '_> = match self {
            Wrapper::Inner(inner) => std::iter::once(inner).boxed(),
            Wrapper::List(list) => list.inner_values().boxed(),
            Wrapper::Union(union) => union
                .0
                .iter()
                .flat_map(|variant| -> Box<dyn Iterator<Item = &T> + '_> {
                    match variant {
                        WrapperVariant::Inner(inner) => std::iter::once(inner).boxed(),
                        WrapperVariant::List(list) => list.inner_values().boxed(),
                        WrapperVariant::Null => std::iter::empty().boxed(),
                    }
                })
                .boxed(),
        };
        values
    }
}

/// Whether a wrapper admits `null` at its top level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nullability {
    Nullable,
    NonNull,
}

/// The wrapper `annotation` writes. `None` when a list's inner type did not parse.
pub fn wrapper_of_type_annotation(annotation: &TypeAnnotation) -> Option<Wrapper<EntityName>> {
    match annotation {
        TypeAnnotation::Named(named) => Wrapper::Inner(named.name.item.0).wrap_some(),
        TypeAnnotation::List(list) => Wrapper::List(wrapper_of_list(list)?.boxed()).wrap_some(),
        TypeAnnotation::Union(union) => Wrapper::Union(UnionWrapper(
            union
                .0
                .iter()
                .map(|variant| match variant.item.reference() {
                    UnionVariant::Named(named) => {
                        WrapperVariant::Inner(named.name.item.0).wrap_some()
                    }
                    UnionVariant::List(list) => {
                        WrapperVariant::List(wrapper_of_list(list)?.boxed()).wrap_some()
                    }
                    UnionVariant::Null(_) => WrapperVariant::Null.wrap_some(),
                })
                .collect::<Option<Vec<_>>>()?,
        ))
        .wrap_some(),
    }
}

fn wrapper_of_list(list: &ListTypeAnnotation) -> Option<Wrapper<EntityName>> {
    wrapper_of_type_annotation(list.inner.as_ref()?.item.reference())
}

#[cfg(test)]
mod tests {
    use common_lang_types::EntityName;
    use intern::string_key::Intern;
    use isograph_parser::{IsoLiteralItem, parse_iso_literal};
    use prelude::Postfix;

    use super::{Nullability, UnionWrapper, Wrapper, WrapperVariant, wrapper_of_type_annotation};

    fn entity(name: &str) -> EntityName {
        name.intern().to()
    }

    /// The wrapper of the `to` clause of `field Pet.owner to <to> { id }`.
    fn wrapper_of_to(to: &str) -> Option<Wrapper<EntityName>> {
        let parsed = parse_iso_literal(&format!("field Pet.owner to {to} {{ id }}"));
        let Some(IsoLiteralItem::Selectable(declaration)) = parsed
            .item
            .as_ref()
            .and_then(|item| item.item.item.as_ref())
            .map(|item| item.item.reference())
        else {
            panic!("the fixture parses to a field declaration");
        };
        wrapper_of_type_annotation(
            declaration
                .target_type
                .as_ref()
                .expect("the fixture has a to clause")
                .item
                .reference(),
        )
    }

    fn nullable(wrapper: WrapperVariant<EntityName>) -> Wrapper<EntityName> {
        Wrapper::Union(UnionWrapper(vec![wrapper, WrapperVariant::Null]))
    }

    #[test]
    fn to_clauses_convert_structurally() {
        let pet = entity("Pet");
        assert_eq!(
            wrapper_of_to("Pet"),
            nullable(WrapperVariant::Inner(pet)).wrap_some()
        );
        assert_eq!(wrapper_of_to("Pet!"), Wrapper::Inner(pet).wrap_some());
        assert_eq!(
            wrapper_of_to("[Pet!]!"),
            Wrapper::List(Wrapper::Inner(pet).boxed()).wrap_some()
        );
        assert_eq!(
            wrapper_of_to("[Pet]"),
            nullable(WrapperVariant::List(
                nullable(WrapperVariant::Inner(pet)).boxed()
            ))
            .wrap_some()
        );
        assert_eq!(wrapper_of_to("[!]"), None);
    }

    #[test]
    fn inner_values_are_in_order() {
        let wrapper = Wrapper::Union(UnionWrapper(vec![
            WrapperVariant::Inner(entity("A")),
            WrapperVariant::List(Wrapper::Inner(entity("B")).boxed()),
            WrapperVariant::Null,
        ]));
        assert_eq!(
            wrapper.inner_values().copied().collect::<Vec<_>>(),
            vec![entity("A"), entity("B")]
        );
    }

    #[test]
    fn a_top_level_null_variant_is_nullable() {
        let nullability = |to: &str| {
            wrapper_of_to(to)
                .expect("the fixture's to clause parses")
                .nullability()
        };
        assert_eq!(nullability("Pet"), Nullability::Nullable);
        assert_eq!(nullability("[Pet]!"), Nullability::NonNull);
        assert_eq!(nullability("Pet!"), Nullability::NonNull);
    }

    #[test]
    fn try_map_keeps_the_structure_and_returns_the_first_error() {
        let wrapper = wrapper_of_to("[Pet]").expect("the fixture's to clause parses");
        assert_eq!(
            wrapper.try_map(&mut |name| name.to_string().len().wrap_ok::<()>()),
            nullable_usize(3).wrap_ok()
        );
        assert_eq!(
            wrapper.try_map(&mut |name| name.to_string().wrap_err::<usize>()),
            "Pet".to_owned().wrap_err()
        );
    }

    fn nullable_usize(value: usize) -> Wrapper<usize> {
        Wrapper::Union(UnionWrapper(vec![
            WrapperVariant::List(
                Wrapper::Union(UnionWrapper(vec![
                    WrapperVariant::Inner(value),
                    WrapperVariant::Null,
                ]))
                .boxed(),
            ),
            WrapperVariant::Null,
        ]))
    }
}
