use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use u64_newtypes::u64_newtype;

use crate::database::Database;

u64_newtype!(ParamId);

impl ParamId {
    pub fn new<T: Hash + Clone + 'static>(db: &Database, param: T) -> Self {
        let mut s = DefaultHasher::new();
        param.hash(&mut s);
        let param_id = s.finish().into();
        db.insert_param(param_id, param);
        param_id
    }
}
