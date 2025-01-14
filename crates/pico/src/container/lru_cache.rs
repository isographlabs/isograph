use core::hash::Hash;
use lru::{IntoIter, Iter, IterMut, LruCache};
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

impl<K, V> Default for LruCacheContainer<K, V>
where
    K: Hash + PartialEq + Eq + Clone + Debug,
    V: Debug,
{
    fn default() -> Self {
        Self::new(10000)
    }
}

impl<K, V> FromIterator<(K, V)> for LruCacheContainer<K, V>
where
    K: Hash + PartialEq + Eq + Clone + Debug,
    V: Debug,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut cache = LruCacheContainer::default();
        for (key, value) in iter {
            cache.insert(key, value);
        }
        cache
    }
}
