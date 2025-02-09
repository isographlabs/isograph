use boxcar::Vec;

#[derive(Debug)]
pub struct Arena<T>(Vec<T>);

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&self, value: T) -> usize {
        let idx = self.0.count();
        self.0.push(value);
        idx
    }

    pub fn get(&self, idx: usize) -> &T {
        debug_assert!(idx <= self.0.count());
        &self.0[idx]
    }
}
