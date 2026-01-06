use std::collections::BTreeMap;

use common_lang_types::{
    Diagnostic, EntityName, SelectableName, WithGenericLocation, WithNonFatalDiagnostics,
};
use isograph_lang_types::{
    Description, SelectionType, TypeAnnotationDeclaration, VariableDeclaration,
};

use crate::{
    CompilationProfile, CreateError, DataModelStage, FlattenedStage, IsInlineFragment,
    IsographDatabase, NestedStage, NetworkProtocol, ServerObjectSelectionInfo, TargetPlatform,
    ValidatedStage,
};

// TODO use a type parameter here to ensure that there are no non-fatal diagnostics,
// though realistically that's impossible - we can always drop them!
// Perhaps we can have non-droppable diagnostics.
pub type MapWithNonfatalDiagnostics<TKey, TValue> = WithNonFatalDiagnostics<BTreeMap<TKey, TValue>>;

pub type DataModelSchema<TCompilationProfile, TStage> = MapWithNonfatalDiagnostics<
    EntityName,
    WithGenericLocation<
        DataModelEntity<TCompilationProfile, TStage>,
        <TStage as DataModelStage>::Location,
    >,
>;
pub type NestedDataModelSchema<TNetworkProtocol> = DataModelSchema<TNetworkProtocol, NestedStage>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct DataModelEntity<TCompilationProfile: CompilationProfile, TStage: DataModelStage> {
    pub name: WithGenericLocation<EntityName, TStage::Location>,
    pub description: Option<WithGenericLocation<Description, TStage::Location>>,

    pub selectables: TStage::Selectables<TCompilationProfile>,
    pub network_protocol_associated_data:
        <<TCompilationProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::EntityAssociatedData,
    pub target_platform_associated_data:
        <<TCompilationProfile as CompilationProfile>::TargetPlatform as TargetPlatform>::EntityAssociatedData,

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
    pub description: Option<WithGenericLocation<Description, TStage::Location>>,

    pub arguments: Vec<VariableDeclaration>,
    pub target_entity: TStage::Resolution<TypeAnnotationDeclaration, TargetEntityParseError>,
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

#[derive(Copy, Clone)]
pub struct TargetEntityParseError;

impl CreateError for TargetEntityParseError {
    fn create_error<TCompilationProfile: CompilationProfile>(
        self,
        _db: &IsographDatabase<TCompilationProfile>,
    ) -> Diagnostic {
        // // 1. Look up the Nested Node in the DB
        // let nested_node = db.get_nested_selectable(key);
        // // 2. Grab the location from the Nested Node
        // let location = nested_node.target_entity.location;
        // // 3. Return the diagnostic
        // Diagnostic::new("Target entity not found", location)
        Diagnostic::new("Target entity parse error".to_string(), None)
    }
}
