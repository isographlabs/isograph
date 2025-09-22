use std::any::Any;

pub trait DynEq: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn dyn_eq(&self, other: &dyn DynEq) -> bool;
}

impl<T> DynEq for T
where
    T: Any + PartialEq,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<T>() {
            return self == other;
        }
        false
    }
}

impl PartialEq<dyn DynEq> for dyn DynEq {
    fn eq(&self, other: &dyn DynEq) -> bool {
        self.dyn_eq(other)
    }
}

impl core::fmt::Debug for dyn DynEq {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynEq {{ .. }}")
    }
}
