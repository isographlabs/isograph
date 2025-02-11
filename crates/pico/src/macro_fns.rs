use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hash, Hasher},
};

use tinyvec::ArrayVec;

use crate::{index::Index, Database, DerivedNode, DerivedNodeId, ParamId};

pub fn init_param_vec() -> ArrayVec<[ParamId; 8]> {
    ArrayVec::<[ParamId; 8]>::default()
}

pub fn intern_param<T: 'static>(db: &Database, param_id: ParamId, param: T) {
    let idx = db
        .epoch_to_generation_map
        .get(&db.current_epoch)
        .unwrap()
        .insert_param(Box::new(param));
    db.param_id_to_index
        .insert(param_id, Index::new(db.current_epoch, idx));
}

pub fn param_exists(db: &Database, param_id: ParamId) -> bool {
    db.contains_param(param_id)
}

pub fn get_derived_node(db: &Database, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
    db.get_derived_node(derived_node_id)
}

pub fn get_param(db: &Database, param_id: ParamId) -> Option<&Box<dyn Any>> {
    db.get_param(param_id)
}

pub fn hash<T: Hash + 'static>(value: &T) -> u64 {
    let mut s = DefaultHasher::new();
    // hash `TypeId` to prevent collisions for new types
    TypeId::of::<T>().hash(&mut s);
    value.hash(&mut s);
    s.finish()
}
