use intern::string_key::Intern;
use intern::Lookup;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use string_key_newtype::string_key_newtype;

use crate::container::Container;
use crate::database::Database;
use crate::storage::StorageMut;

string_key_newtype!(ParamId);

impl ParamId {
    pub fn intern<T, Db>(db: &mut Db, param: T) -> Self
    where
        T: Hash + Clone + 'static,
        Db: Database,
    {
        let mut s = DefaultHasher::new();
        param.hash(&mut s);
        let param_id = s.finish().to_string().intern().into();
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

    pub fn as_u64(&self) -> u64 {
        self.lookup().parse::<u64>().unwrap()
    }
}
