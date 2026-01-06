use pico::MemoRef;

use crate::FlattenedDataModelEntity;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, Ord, PartialOrd)]
pub struct IsConcrete(pub bool);

pub type ServerEntity<TCompilationProfile> = FlattenedDataModelEntity<TCompilationProfile>;

pub type MemoRefServerEntity<TCompilationProfile> = MemoRef<ServerEntity<TCompilationProfile>>;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
pub struct ServerObjectSelectionInfo {
    pub is_concrete: IsConcrete,
}
