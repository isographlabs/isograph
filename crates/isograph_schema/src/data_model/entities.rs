use common_lang_types::EntityName;
use isograph_lang_types::{Description, SelectionType};
use pico::MemoRef;

use crate::{CompilationProfile, NetworkProtocol, TargetPlatform};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub struct IsConcrete(pub bool);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerEntity<TCompilationProfile: CompilationProfile> {
    pub description: Option<Description>,
    pub name: EntityName,
    // TODO this is obviously a hack
    pub selection_info: SelectionType<(), IsConcrete>,
    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityAssociatedData,
    pub target_platform_associated_data:
        <<TCompilationProfile as CompilationProfile>::TargetPlatform as TargetPlatform>::EntityAssociatedData,
}

pub type MemoRefServerEntity<TCompilationProfile> = MemoRef<ServerEntity<TCompilationProfile>>;
