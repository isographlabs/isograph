pub trait Container<K, V> {
    fn contains_key(&self, k: &K) -> bool;
    fn get(&self, k: &K) -> Option<&V>;
    fn get_mut(&mut self, k: &K) -> Option<&mut V>;
    fn insert(&mut self, k: K, v: V) -> Option<V>;
    fn remove(&mut self, k: &K) -> Option<V>;
}
