use pico::MemoRef;

use crate::FlattenedDataModelEntity;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, Ord, PartialOrd)]
pub struct IsConcrete(pub bool);

pub type MemoRefServerEntity<TCompilationProfile> =
    MemoRef<FlattenedDataModelEntity<TCompilationProfile>>;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
pub struct ServerObjectSelectionInfo {
    pub is_concrete: IsConcrete,
}
