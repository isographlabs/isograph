use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(u64);

impl Key {
    pub fn new<T: Hash>(key: &T) -> Self {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        Self(s.finish())
    }
}

impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Key(value)
    }
}
