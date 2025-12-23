use std::collections::BTreeMap;

use common_lang_types::{
    Diagnostic, EntityName, SelectableName, WithGenericLocation, WithNonFatalDiagnostics,
};
use isograph_lang_types::VariableDeclaration;

use crate::{
    CompilationProfile, CreateError, DataModelStage, FlattenedStage, IsographDatabase, NestedStage,
    NetworkProtocol, ValidatedStage,
};

// TODO use a type parameter here to ensure that there are no non-fatal diagnostics,
// though realistically that's impossible - we can always drop them!
// Perhaps we can have non-droppable diagnostics.
pub type MapWithNonfatalDiagnostics<TKey, TValue> = WithNonFatalDiagnostics<BTreeMap<TKey, TValue>>;

pub type DataModelSchema<TNetworkProtocol, TStage> = MapWithNonfatalDiagnostics<
    EntityName,
    WithGenericLocation<
        DataModelEntity<TNetworkProtocol, TStage>,
        <TStage as DataModelStage>::Location,
    >,
>;
pub type NestedDataModelSchema<TNetworkProtocol> = DataModelSchema<TNetworkProtocol, NestedStage>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct DataModelEntity<TNetworkProtocol: NetworkProtocol, TStage: DataModelStage> {
    pub name: WithGenericLocation<EntityName, TStage::Location>,
    pub selectables: TStage::Selectables<TNetworkProtocol>,
    pub network_protocol_associated_data: TNetworkProtocol::EntityAssociatedData,
}
pub type NestedDataModelEntity<TNetworkProtocol> = DataModelEntity<TNetworkProtocol, NestedStage>;
pub type FlattenedDataModelEntity<TNetworkProtocol> =
    DataModelEntity<TNetworkProtocol, FlattenedStage>;
pub type ValidatedDataModelEntity<TNetworkProtocol> =
    DataModelEntity<TNetworkProtocol, ValidatedStage>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct DataModelSelectable<TNetworkProtocol: NetworkProtocol, TStage: DataModelStage> {
    pub name: WithGenericLocation<SelectableName, TStage::Location>,
    pub arguments: Vec<VariableDeclaration>,
    pub network_protocol_associated_data: TNetworkProtocol::SelectableAssociatedData,
    pub target_entity: TStage::Resolution<EntityName, TargetEntityParseError>,
}
pub type NestedDataModelSelectable<TNetworkProtocol> =
    DataModelSelectable<TNetworkProtocol, NestedStage>;
pub type FlattenedDataModelSelectable<TNetworkProtocol> =
    DataModelSelectable<TNetworkProtocol, FlattenedStage>;
pub type ValidatedDataModelSelectable<TNetworkProtocol> =
    DataModelSelectable<TNetworkProtocol, ValidatedStage>;

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
