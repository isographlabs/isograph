use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hash, Hasher},
};

use tinyvec::ArrayVec;

use crate::{index::Index, Database, DerivedNode, DerivedNodeId, ParamId};

pub fn init_param_vec() -> ArrayVec<[ParamId; 8]> {
    ArrayVec::<[ParamId; 8]>::default()
}

pub fn intern_param<T: Hash + Clone + 'static>(db: &Database, param: &T) -> ParamId {
    let param_id = hash(param).into();
    if !db.contains_param(param_id) {
        let idx = db
            .epoch_to_generation_map
            .get(&db.current_epoch)
            .unwrap()
            .insert_param(Box::new(param.clone()));
        db.param_id_to_index
            .insert(param_id, Index::new(db.current_epoch, idx));
    }
    param_id
}

pub fn get_derived_node(db: &Database, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
    db.get_derived_node(derived_node_id)
}

pub fn get_param(db: &Database, param_id: ParamId) -> Option<&Box<dyn Any>> {
    db.get_param(param_id)
}

pub fn hash<T: Hash + 'static>(value: &T) -> u64 {
    let mut s = DefaultHasher::new();
    // hash `TypeId` to prevent collisions for newtypes
    TypeId::of::<T>().hash(&mut s);
    value.hash(&mut s);
    s.finish()
}
