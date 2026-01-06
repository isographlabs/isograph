use common_lang_types::EntityName;
use isograph_lang_types::{Description, SelectionType};
use pico::MemoRef;

use crate::{CompilationProfile, NetworkProtocol, TargetPlatform};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, Ord, PartialOrd)]
pub struct IsConcrete(pub bool);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerEntity<TCompilationProfile: CompilationProfile> {
    pub description: Option<Description>,
    pub name: EntityName,
    // TODO this is obviously a hack
    // IsConcrete is used in (at least) two situations: first, it is used to add a __typename
    // selection if the entity is not concrete (i.e. needed by the network protocol when
    // generating query text), and to add the concrete type into the normalization AST (thus
    // needed by the target platform, when that trait is responsible for creating the
    // normalization AST.)
    // That's awkward!
    pub selection_info: SelectionType<(), ServerObjectSelectionInfo>,
    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityAssociatedData,
    pub target_platform_associated_data:
        <<TCompilationProfile as CompilationProfile>::TargetPlatform as TargetPlatform>::EntityAssociatedData,
}

pub type MemoRefServerEntity<TCompilationProfile> = MemoRef<ServerEntity<TCompilationProfile>>;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
pub struct ServerObjectSelectionInfo {
    pub is_concrete: IsConcrete,
}
