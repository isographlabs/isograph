use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hash, Hasher},
};

use dashmap::Entry;
use tinyvec::ArrayVec;

use crate::{index::Index, Database, ParamId};

pub fn init_param_vec() -> ArrayVec<[ParamId; 8]> {
    ArrayVec::<[ParamId; 8]>::default()
}

pub fn intern_borrowed_param<Db: Database, T: Hash + Clone + 'static>(
    db: &Db,
    param: &T,
) -> ParamId {
    let param_id = hash(param).into();
    if let Entry::Vacant(v) = db.get_storage().internal.param_id_to_index.entry(param_id) {
        let idx = db
            .get_storage()
            .internal
            .params
            .push(Box::new(param.clone()));
        v.insert(Index::new(idx));
    }
    param_id
}

pub fn intern_owned_param<Db: Database, T: Hash + Clone + 'static>(db: &Db, param: T) -> ParamId {
    let param_id = hash(&param).into();
    if let Entry::Vacant(v) = db.get_storage().internal.param_id_to_index.entry(param_id) {
        let idx = db.get_storage().internal.params.push(Box::new(param));
        v.insert(Index::new(idx));
    }
    param_id
}

pub fn get_param<Db: Database>(db: &Db, param_id: ParamId) -> Option<&Box<dyn Any>> {
    db.get_storage().internal.get_param(param_id)
}

pub fn hash<T: Hash + 'static>(value: &T) -> u64 {
    let mut s = DefaultHasher::new();
    // hash `TypeId` to prevent collisions for newtypes
    TypeId::of::<T>().hash(&mut s);
    value.hash(&mut s);
    s.finish()
}
