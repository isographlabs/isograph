use common_lang_types::EntityName;
use datafusion::prelude::*;
use intern::string_key::Intern;
use isograph_schema::{
    MergedScalarFieldSelection, MergedServerSelection, NameAndArguments, NormalizationKey,
};
use sql_network_protocol::query_generation::logical_plan_builder::build_logical_plan;
use sql_network_protocol::substrait::serialize::{
    serialize_to_substrait, write_substrait_artifact,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[tokio::test]
async fn test_substrait_round_trip() {
    // 1. Build a LogicalPlan from a simple selection map
    let mut selection_map = BTreeMap::new();

    selection_map.insert(
        NormalizationKey::Id,
        MergedServerSelection::ScalarField(MergedScalarFieldSelection {
            name: "id".intern().into(),
            arguments: vec![],
            is_fallible: false,
        }),
    );

    selection_map.insert(
        NormalizationKey::ServerField(NameAndArguments {
            name: "name".intern().into(),
            arguments: Default::default(),
        }),
        MergedServerSelection::ScalarField(MergedScalarFieldSelection {
            name: "name".intern().into(),
            arguments: vec![],
            is_fallible: false,
        }),
    );

    let entity_name: EntityName = "users".intern().into();

    // Build LogicalPlan
    let logical_plan =
        build_logical_plan(entity_name, &selection_map).expect("Failed to build logical plan");

    // 2. Serialize to Substrait
    let ctx = SessionContext::new();
    let session_state = ctx.state();

    let substrait_bytes = serialize_to_substrait(&logical_plan, &session_state)
        .expect("Failed to serialize to Substrait");

    assert!(
        !substrait_bytes.is_empty(),
        "Substrait bytes should not be empty"
    );

    // 3. Write to disk
    let test_path = PathBuf::from("/tmp/test_query_plan.bin");
    write_substrait_artifact(substrait_bytes.clone(), &test_path)
        .expect("Failed to write Substrait artifact");

    // 4. Verify file exists
    assert!(test_path.exists(), "Substrait file should exist");

    // Clean up
    std::fs::remove_file(test_path).ok();
}
