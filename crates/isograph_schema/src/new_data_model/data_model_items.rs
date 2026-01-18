use std::collections::BTreeMap;

use common_lang_types::{
    EntityName, SelectableName, WithGenericLocation, WithGenericNonFatalDiagnostics,
};
use isograph_lang_types::{
    DefinitionLocation, Description, SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};
use pico::MemoRef;

use crate::{
    CompilationProfile, DataModelStage, FlattenedStage, NestedStage, NetworkProtocol,
    TargetPlatform, ValidatedStage, flatten::BothFlattenedResults,
};

pub type MapWithNonfatalDiagnostics<TKey, TValue, TError> =
    WithGenericNonFatalDiagnostics<BTreeMap<TKey, TValue>, TError>;

pub type DataModelSchema<TCompilationProfile, TStage> = MapWithNonfatalDiagnostics<
    EntityName,
    WithGenericLocation<
        DataModelEntity<TCompilationProfile, TStage>,
        <TStage as DataModelStage>::Location,
    >,
    <TStage as DataModelStage>::Error,
>;
pub type NestedDataModelSchema<TCompilationProfile> =
    DataModelSchema<TCompilationProfile, NestedStage>;
// TODO this is differently shaped than nested data model schema...
// which is indicative of a problem! We are ignoring the top-level non fatal
// diagnostics, because we don't care about it. Anyway, TODO figure out
// what to do here.
pub type FlattenedDataModelSchema<TCompilationProfile> = BTreeMap<
    EntityName,
    WithGenericLocation<
        BothFlattenedResults<NestedDataModelEntity<TCompilationProfile>>,
        <FlattenedStage as DataModelStage>::Location,
    >,
>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct EntityAssociatedData<TCompilationProfile: CompilationProfile> {
    pub network_protocol:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityAssociatedData,
    pub target_platform:
        <<TCompilationProfile as CompilationProfile>::TargetPlatform as TargetPlatform>::EntityAssociatedData,
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct DataModelEntity<TCompilationProfile: CompilationProfile, TStage: DataModelStage> {
    pub name: WithGenericLocation<EntityName, TStage::Location>,
    pub description: Option<WithGenericLocation<Description, TStage::Location>>,

    pub selectables: TStage::Selectables<TCompilationProfile>,
    pub associated_data: DefinitionLocation<EntityAssociatedData<TCompilationProfile>, ()>,

    // TODO this is obviously a hack
    // IsConcrete is used in (at least) two situations: first, it is used to add a __typename
    // selection if the entity is not concrete (i.e. needed by the network protocol when
    // generating query text), and to add the concrete type into the normalization AST (thus
    // needed by the target platform, when that trait is responsible for creating the
    // normalization AST.)
    // That's awkward!
    pub selection_info: SelectionType<(), ServerObjectSelectionInfo>,
}
pub type NestedDataModelEntity<TCompilationProfile> =
    DataModelEntity<TCompilationProfile, NestedStage>;
pub type FlattenedDataModelEntity<TCompilationProfile> =
    DataModelEntity<TCompilationProfile, FlattenedStage>;
pub type ValidatedDataModelEntity<TCompilationProfile> =
    DataModelEntity<TCompilationProfile, ValidatedStage>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct DataModelSelectable<TCompilationProfile: CompilationProfile, TStage: DataModelStage> {
    pub name: WithGenericLocation<SelectableName, TStage::Location>,
    pub parent_entity_name: WithGenericLocation<EntityName, TStage::Location>,
    pub description: Option<WithGenericLocation<Description, TStage::Location>>,

    pub arguments: Vec<VariableDeclaration>,
    // Note: we never actually produce any error results here! Note that that's fine.
    // This still forces us to learn how to handle results :) and we will have errors here
    // at some point! (e.g. if the field is something like `fieldName: @asdf`)
    pub target_entity: WithGenericLocation<Result<TypeAnnotationDeclaration, TStage::Error>, TStage::Location>,
    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::SelectableAssociatedData,
    pub target_platform_associated_data:
        <<TCompilationProfile as CompilationProfile>::TargetPlatform as TargetPlatform>::SelectableAssociatedData,

    // TODO this is obviously a GraphQL-ism! But it's used in a bunch of places, so it's
    // not really easy to move it to TargetPlatform. However, we know it at parse time,
    // because only asConcreteType fields are inline fragments.
    pub is_inline_fragment: IsInlineFragment,
}
pub type NestedDataModelSelectable<TCompilationProfile> =
    DataModelSelectable<TCompilationProfile, NestedStage>;
pub type FlattenedDataModelSelectable<TCompilationProfile> =
    DataModelSelectable<TCompilationProfile, FlattenedStage>;
pub type ValidatedDataModelSelectable<TCompilationProfile> =
    DataModelSelectable<TCompilationProfile, ValidatedStage>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
pub struct IsInlineFragment(pub bool);

impl From<bool> for IsInlineFragment {
    fn from(value: bool) -> Self {
        IsInlineFragment(value)
    }
}

pub type MemoRefServerSelectable<TCompilationProfile> =
    MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, Ord, PartialOrd)]
pub struct IsConcrete(pub bool);

pub type MemoRefServerEntity<TCompilationProfile> =
    MemoRef<FlattenedDataModelEntity<TCompilationProfile>>;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
pub struct ServerObjectSelectionInfo {
    pub is_concrete: IsConcrete,
}
