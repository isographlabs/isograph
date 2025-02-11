use intern::InternSerdes;
use intern::{intern_struct, InternId};
use serde::{Deserialize, Serialize};
use u64_newtypes::u64_newtype;

use crate::{DerivedNodeId, SourceId};

u64_newtype!(HashKey);

intern_struct! {
    pub struct HashId = Intern<HashKey> {
      serdes("InternSerdes<HashId>");
      const EMPTY = HashKey(0);
    }
}

impl Default for HashId {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ParamId(HashId);

impl ParamId {
    pub fn inner(&self) -> HashId {
        self.0
    }
}

impl From<u64> for ParamId {
    fn from(value: u64) -> Self {
        Self(HashId::intern(HashKey(value)))
    }
}

impl<T> From<SourceId<T>> for ParamId {
    fn from(value: SourceId<T>) -> Self {
        Self(value.key.0)
    }
}

impl<T> From<&SourceId<T>> for ParamId {
    fn from(value: &SourceId<T>) -> Self {
        Self(value.key.0)
    }
}

impl From<DerivedNodeId> for ParamId {
    fn from(value: DerivedNodeId) -> Self {
        let idx: u64 = value.index().into();
        Self::from(idx)
    }
}

impl From<&DerivedNodeId> for ParamId {
    fn from(value: &DerivedNodeId) -> Self {
        Self::from(value.index() as u64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Key(HashId);

impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Self(HashId::intern(HashKey(value)))
    }
}

impl From<HashId> for Key {
    fn from(value: HashId) -> Self {
        Self(value)
    }
}
