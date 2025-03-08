use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{dyn_eq::DynEq, epoch::Epoch, intern::Key, ParamId};

pub trait Source {
    fn get_key(&self) -> Key;
}

#[derive(Debug, PartialEq, Eq)]
pub struct SourceId<T> {
    pub key: Key,
    phantom: PhantomData<T>,
}

// We have to implement Clone and Copy ourselves. Otherwise,
// Clone and Copy would only be implemented if T: Clone or T: Copy,
// which is not correct! T only appears as part of PhantomData,
// so a SourceId is cloneable/copiable, regardless of what T is.
impl<T> Clone for SourceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SourceId<T> {}

impl<T> Hash for SourceId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T> SourceId<T> {
    pub fn new(source: &impl Source) -> Self {
        Self {
            key: source.get_key(),
            phantom: PhantomData,
        }
    }
}

impl<T> From<ParamId> for SourceId<T> {
    fn from(value: ParamId) -> Self {
        Self {
            key: value.inner().into(),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct SourceNode {
    pub time_updated: Epoch,
    pub value: Box<dyn DynEq>,
}
