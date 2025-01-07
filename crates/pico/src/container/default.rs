use core::hash::Hash;
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
}
