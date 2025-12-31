use isograph_lang_types::DefinitionLocation;
use pico::MemoRef;

use crate::{
    ClientObjectSelectable, MemoRefClientSelectable, MemoRefServerSelectable, ServerSelectable,
};

pub type MemoRefSelectable<TCompilationProfile> = DefinitionLocation<
    MemoRefServerSelectable<TCompilationProfile>,
    MemoRefClientSelectable<TCompilationProfile>,
>;

pub type BorrowedObjectSelectable<'a, TCompilationProfile> = DefinitionLocation<
    &'a ServerSelectable<TCompilationProfile>,
    &'a ClientObjectSelectable<TCompilationProfile>,
>;

pub type MemoRefObjectSelectable<TCompilationProfile> = DefinitionLocation<
    MemoRef<ServerSelectable<TCompilationProfile>>,
    MemoRef<ClientObjectSelectable<TCompilationProfile>>,
>;
