use crate::{
    dyn_eq::DynEq,
    epoch::Epoch,
    source::{Source, SourceId},
    storage::Storage,
};

pub trait Database {
    fn storage(&self) -> &Storage<Self>;
    fn storage_mut(&mut self) -> &mut Storage<Self>;
    fn current_epoch(&self) -> Epoch;
    fn increment_epoch(&mut self) -> Epoch;
    fn get<T: Clone + 'static>(&self, id: SourceId<T>) -> T;
    fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T>;
    fn remove<T>(&mut self, id: SourceId<T>);
}
