use intern::string_key::Intern;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

use string_key_newtype::string_key_newtype;

string_key_newtype!(SourceKey);

impl SourceKey {
    pub fn intern<T: Hash>(key: &T) -> Self {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        s.finish().to_string().intern().into()
    }
}

pub trait Source {
    fn get_key(&self) -> SourceKey;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SourceId<T> {
    pub key: SourceKey,
    phantom: PhantomData<T>,
}

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
