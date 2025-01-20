use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::key::Key;

pub trait Source {
    fn get_key(&self) -> Key;
}

#[derive(Debug, PartialEq, Eq)]
pub struct SourceId<T> {
    pub key: Key,
    phantom: PhantomData<T>,
}

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
