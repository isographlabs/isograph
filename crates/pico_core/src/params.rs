use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::container::Container;
use crate::database::Database;
use crate::storage::StorageMut;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamId(u64);

impl ParamId {
    pub fn intern<T, Db>(db: &mut Db, param: T) -> Self
    where
        T: Hash + Clone + 'static,
        Db: Database,
    {
        let mut s = DefaultHasher::new();
        param.hash(&mut s);
        let param_id = Self(s.finish());
        db.storage_mut().params().insert(param_id, Box::new(param));
        param_id
    }

    pub fn get<T, Db>(&self, db: &mut Db) -> Option<T>
    where
        T: Hash + Clone + 'static,
        Db: Database,
    {
        self.get_ref(db).cloned()
    }

    pub fn get_ref<'db, T, Db>(&self, db: &'db mut Db) -> Option<&'db T>
    where
        T: Hash + Clone + 'static,
        Db: Database,
    {
        db.storage_mut().params().get(self)?.downcast_ref::<T>()
    }
}
