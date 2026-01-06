use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

use common_lang_types::{DiagnosticResult, EntityName, WithNonFatalDiagnostics};
use pico::MemoRef;

use crate::{
    IsographDatabase, NestedDataModelSchema, NetworkProtocol, ParseTypeSystemOutcome,
    RootOperationName, TargetPlatform,
};

pub trait CompilationProfile:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type NetworkProtocol: NetworkProtocol;
    type TargetPlatform: TargetPlatform;

    // TODO this should return a Vec<Result<...>>, not a Result<Vec<...>>, probably
    #[expect(clippy::type_complexity)]
    fn deprecated_parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> &DiagnosticResult<(
        WithNonFatalDiagnostics<ParseTypeSystemOutcome<Self>>,
        // TODO just seems awkward that we return fetchable types
        BTreeMap<EntityName, RootOperationName>,
    )>;

    fn parse_nested_data_model_schema(
        db: &IsographDatabase<Self>,
    ) -> MemoRef<NestedDataModelSchema<Self>>;
}
