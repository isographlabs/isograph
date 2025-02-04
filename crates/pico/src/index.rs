use std::{any::type_name, fmt, marker::PhantomData};

use crate::epoch::Epoch;

#[derive(Clone, Copy)]
pub struct Index<T> {
    pub epoch: Epoch,
    pub idx: usize,
    phantom: PhantomData<T>,
}

impl<T> Index<T> {
    pub fn new(epoch: Epoch, idx: usize) -> Self {
        Self {
            epoch,
            idx,
            phantom: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Index<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Index<{}>[{:?}:{:?}]",
            type_name::<T>(),
            self.epoch,
            self.idx,
        )
    }
}
