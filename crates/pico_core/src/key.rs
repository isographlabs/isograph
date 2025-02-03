use std::hash::{DefaultHasher, Hash, Hasher};

use u64_newtypes::u64_newtype;

u64_newtype!(Key);

impl Key {
    pub fn new<T: Hash>(key: &T) -> Self {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        s.finish().into()
    }
}
