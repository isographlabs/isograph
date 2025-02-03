use std::fmt;

use crate::epoch::Epoch;

#[derive(Clone, Copy)]
pub struct Index {
    pub epoch: Epoch,
    pub idx: usize,
}

impl Index {
    pub fn new(epoch: Epoch, idx: usize) -> Self {
        Self { epoch, idx }
    }
}

impl fmt::Debug for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Index[{:?}:{:?}]", self.epoch, self.idx)
    }
}
