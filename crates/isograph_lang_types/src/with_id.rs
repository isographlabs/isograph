pub trait HasId {
    type Id;
}

// TODO this struct doesn't need to exist; it only existed because ids were
// not part of the entity. Now, we are using names as ids, and names exist
// on the entities.
pub struct WithId<T: HasId> {
    pub item: T,
    pub id: T::Id,
}

impl<T: HasId> WithId<T> {
    pub fn new(id: T::Id, item: T) -> Self {
        WithId { item, id }
    }
}

#[macro_export]
macro_rules! impl_with_id {
    ( $type:ident<$($param:ident: $bound:ident),*>, $id:ident) => {
        impl<$($param: $bound),*> ::isograph_lang_types::HasId for $type<$($param),*> {
            type Id = $id;
        }
        impl<$($param: $bound),*> ::isograph_lang_types::HasId for &$type<$($param),*> {
            type Id = $id;
        }
        impl<$($param: $bound),*> ::isograph_lang_types::HasId for &mut $type<$($param),*> {
            type Id = $id;
        }
    };
}
