pub trait Container<K, V>: FromIterator<(K, V)> {
    fn contains_key(&self, k: &K) -> bool;
    fn get(&self, k: &K) -> Option<&V>;
    fn get_mut(&mut self, k: &K) -> Option<&mut V>;
    fn insert(&mut self, k: K, v: V) -> Option<V>;
    fn remove(&mut self, k: &K) -> Option<V>;

    type Iter<'a>: Iterator<Item = (&'a K, &'a V)>
    where
        K: 'a,
        V: 'a,
        Self: 'a;
    type IterMut<'a>: Iterator<Item = (&'a K, &'a mut V)>
    where
        K: 'a,
        V: 'a,
        Self: 'a;
    type IntoIter: Iterator<Item = (K, V)>;

    fn iter(&self) -> Self::Iter<'_>;
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
    fn into_iter(self) -> Self::IntoIter;
}
