use std::{
    any::Any,
    hash::{DefaultHasher, Hash, Hasher},
};

use dashmap::mapref::one::Ref;

use crate::{Database, DerivedNode, DerivedNodeId, ParamId};

pub fn intern_param<T: Hash + Clone + 'static>(db: &Database, param: T) -> ParamId {
    let param_id = hash(&param).into();
    eprintln!("intern, this deadlocks");
    if !db.params.contains_key(&param_id) {
        db.params.insert(param_id, Box::new(param));
    }
    param_id
}

pub fn get_derived_node<'db>(
    db: &'db Database,
    derived_node_id: DerivedNodeId,
) -> Option<&'db DerivedNode> {
    db.get_derived_node(derived_node_id)
}

pub fn get_param<'db>(
    db: &'db Database,
    param_id: ParamId,
) -> Option<impl std::ops::Deref<Target = Box<dyn Any>> + 'db> {
    db.get_param(param_id)
}

pub fn hash<T: Hash>(value: &T) -> u64 {
    let mut s = DefaultHasher::new();
    value.hash(&mut s);
    s.finish()
}
