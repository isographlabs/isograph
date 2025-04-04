pub trait HasId {
    type Id;
}

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
