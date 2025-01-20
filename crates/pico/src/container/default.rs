use core::hash::Hash;
use std::collections::hash_map::{IntoIter, Iter, IterMut};
use std::collections::HashMap;
use std::fmt::Debug;

use pico_core::container::Container;

#[derive(Debug, Default)]
pub struct DefaultContainer<K: Hash + Eq, V>(HashMap<K, V>);

impl<K: Hash + Eq, V> DefaultContainer<K, V> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<K, V> Container<K, V> for DefaultContainer<K, V>
where
    K: Hash + PartialEq + Eq + Clone + Debug,
    V: Debug,
{
    type Iter<'a>
        = Iter<'a, K, V>
    where
        K: 'a,
        V: 'a;
    type IterMut<'a>
        = IterMut<'a, K, V>
    where
        K: 'a,
        V: 'a;
    type IntoIter = IntoIter<K, V>;

    fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }

    fn get(&self, k: &K) -> Option<&V> {
        self.0.get(k)
    }

    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.0.get_mut(k)
    }

    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.insert(k, v)
    }

    fn remove(&mut self, k: &K) -> Option<V> {
        self.0.remove(k)
    }

    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.0.iter_mut()
    }

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V> FromIterator<(K, V)> for DefaultContainer<K, V>
where
    K: Hash + PartialEq + Eq + Clone + Debug,
    V: Debug,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(HashMap::from_iter(iter))
    }
}
