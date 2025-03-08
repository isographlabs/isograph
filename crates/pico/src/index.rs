use std::{any::type_name, fmt, marker::PhantomData};

#[derive(Clone, Copy)]
pub struct Index<T> {
    pub idx: usize,
    phantom: PhantomData<T>,
}

impl<T> Index<T> {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            phantom: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Index<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Index<{}>[{:?}]", type_name::<T>(), self.idx)
    }
}
