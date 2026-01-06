use isograph_lang_types::DefinitionLocation;
use pico::MemoRef;

use crate::{
    ClientObjectSelectable, FlattenedDataModelSelectable, MemoRefClientSelectable,
    MemoRefServerSelectable,
};

pub type MemoRefSelectable<TCompilationProfile> = DefinitionLocation<
    MemoRefServerSelectable<TCompilationProfile>,
    MemoRefClientSelectable<TCompilationProfile>,
>;

pub type BorrowedObjectSelectable<'a, TCompilationProfile> = DefinitionLocation<
    &'a FlattenedDataModelSelectable<TCompilationProfile>,
    &'a ClientObjectSelectable<TCompilationProfile>,
>;

pub type MemoRefObjectSelectable<TCompilationProfile> = DefinitionLocation<
    MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>,
    MemoRef<ClientObjectSelectable<TCompilationProfile>>,
>;
