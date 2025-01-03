use intern::string_key::Intern;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use string_key_newtype::string_key_newtype;

use crate::database::Database;

string_key_newtype!(ParamId);

impl ParamId {
    pub fn new<T: Hash>(t: &T) -> Self {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish().to_string().intern().into()
    }
}

pub fn param_id<T: Hash + Clone + 'static>(db: &mut Database, param: T) -> ParamId {
    let param_id = ParamId::new(&param);
    db.params.put(param_id, Box::new(param));
    param_id
}
