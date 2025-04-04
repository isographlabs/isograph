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
