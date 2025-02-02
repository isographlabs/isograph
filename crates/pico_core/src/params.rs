use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::database::Database;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamId(u64);

impl ParamId {
    pub fn intern<T, Db>(db: &Db, param: T) -> Self
    where
        T: Hash + Clone + 'static,
        Db: Database,
    {
        let mut s = DefaultHasher::new();
        param.hash(&mut s);
        let param_id = Self(s.finish());
        if !db.storage().contains_param(param_id) {
            db.storage().insert_param(param_id, param);
        }
        param_id
    }
}
