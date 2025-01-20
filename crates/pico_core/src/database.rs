use crate::{
    dyn_eq::DynEq,
    epoch::Epoch,
    source::Source,
    source::SourceId,
    storage::{Storage, StorageMut},
};

pub trait Database {
    fn storage(&self) -> &impl Storage<Self>;
    fn storage_mut(&mut self) -> &mut impl StorageMut<Self>;
    fn current_epoch(&self) -> Epoch;
    fn increment_epoch(&mut self) -> Epoch;
    fn get<T: Clone + 'static>(&mut self, id: SourceId<T>) -> T;
    fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T>;
    fn remove<T>(&mut self, id: SourceId<T>);
}
