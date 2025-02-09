use std::{
    any::Any,
    hash::{DefaultHasher, Hash, Hasher},
};

use dashmap::mapref::one::Ref;

use crate::{Database, DerivedNode, DerivedNodeId, DerivedNodeValue, ParamId};

pub fn intern_param<T: Hash + Clone + 'static>(db: &Database, param: T) -> ParamId {
    let param_id = hash(&param).into();
    eprintln!("intern, this deadlocks");
    if !db.params.contains_key(&param_id) {
        db.params.insert(param_id, Box::new(param));
    }
    param_id
}

pub fn get_derived_node_value(
    db: &Database,
    derived_node_id: DerivedNodeId,
) -> Option<&DerivedNodeValue> {
    db.get_derived_node(derived_node_id)
        .map(|derived_node| db.get_derived_node_value(derived_node.derived_node_index))
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
