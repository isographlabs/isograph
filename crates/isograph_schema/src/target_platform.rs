use std::{fmt::Debug, hash::Hash};

pub trait TargetPlatform:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
}
