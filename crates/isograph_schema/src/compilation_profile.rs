use std::{fmt::Debug, hash::Hash};

use crate::{NetworkProtocol, TargetPlatform};

pub trait CompilationProfile:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type NetworkProtocol: NetworkProtocol;
    type TargetPlatform: TargetPlatform;
}
