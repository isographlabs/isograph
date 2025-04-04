pub trait HasId {
    type Id;
}

pub struct WithId<T: HasId> {
    pub item: T,
    pub id: T::Id,
}
