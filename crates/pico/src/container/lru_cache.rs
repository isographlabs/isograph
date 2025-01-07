use core::hash::Hash;
use lru::LruCache;
use std::{fmt::Debug, num::NonZeroUsize};

use pico_core::container::Container;

#[derive(Debug)]
pub struct LruCacheContainer<K: Hash + Eq, V>(LruCache<K, V>);

impl<K: Hash + Eq, V> LruCacheContainer<K, V> {
    pub fn new(cache_size: usize) -> Self {
        let cap = NonZeroUsize::new(cache_size).unwrap();
        Self(LruCache::new(cap))
    }
}

impl<K, V> Container<K, V> for LruCacheContainer<K, V>
where
    K: Hash + PartialEq + Eq + Clone + Debug,
    V: Debug,
{
    fn contains_key(&self, key: &K) -> bool {
        self.0.contains(key)
    }

    fn get(&self, k: &K) -> Option<&V> {
        self.0.peek(k)
    }

    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.0.get_mut(k)
    }

    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.put(k, v)
    }

    fn remove(&mut self, k: &K) -> Option<V> {
        self.0.pop(k)
    }
}

impl<K, V> Default for LruCacheContainer<K, V>
where
    K: Hash + PartialEq + Eq + Clone + Debug,
    V: Debug,
{
    fn default() -> Self {
        Self::new(10000)
    }
}
