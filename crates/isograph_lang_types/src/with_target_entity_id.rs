pub trait HasTargetId {
    type TargetId;
}

pub struct WithTargetId<T: HasTargetId> {
    pub item: T,
    pub id: T::TargetId,
}

impl<T: HasTargetId> WithTargetId<T> {
    pub fn new(id: T::TargetId, item: T) -> Self {
        WithTargetId { item, id }
    }
}

#[macro_export]
macro_rules! impl_with_target_id {
    ( $type:ident<$($param:ident: $bound:ident),*>, $id:ident) => {
        impl<$($param: $bound),*> ::isograph_lang_types::HasTargetId for $type<$($param),*> {
            type TargetId = $id;
        }
        impl<$($param: $bound),*> ::isograph_lang_types::HasTargetId for &$type<$($param),*> {
            type TargetId = $id;
        }
        impl<$($param: $bound),*> ::isograph_lang_types::HasTargetId for &mut $type<$($param),*> {
            type TargetId = $id;
        }
    };
}
