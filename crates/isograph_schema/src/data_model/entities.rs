use common_lang_types::{EntityName, JavascriptName};
use isograph_lang_types::{Description, SelectionType};
use pico::MemoRef;

use crate::{CompilationProfile, NetworkProtocol};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub struct IsConcrete(pub bool);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ServerEntity<TCompilationProfile: CompilationProfile> {
    pub description: Option<Description>,
    pub name: EntityName,
    // TODO this is obviously a hack
    pub selection_info: SelectionType<JavascriptName, IsConcrete>,
    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityAssociatedData,
}

pub type MemoRefServerEntity<TCompilationProfile> = MemoRef<ServerEntity<TCompilationProfile>>;
