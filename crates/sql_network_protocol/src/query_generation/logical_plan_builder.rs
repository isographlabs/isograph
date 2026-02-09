use common_lang_types::EntityName;
use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::datasource::{provider_as_source, MemTable};
use datafusion::logical_expr::{col, LogicalPlan, LogicalPlanBuilder};
use intern::Lookup;
use isograph_schema::{MergedSelectionMap, MergedServerSelection};
use prelude::Postfix;
use std::sync::Arc;

/// Builds a DataFusion LogicalPlan from Isograph's MergedSelectionMap
///
/// For Phase 1: Simple SELECT with column projection (no WHERE, no JOINs)
pub fn build_logical_plan(
    entity_name: EntityName,
    merged_selection_map: &MergedSelectionMap,
) -> Result<LogicalPlan, String> {
    // Determine root table name (lowercase entity name)
    let table_name = determine_root_table(entity_name);

    // Extract column names from selections
    let columns = extract_projections(merged_selection_map)?;

    if columns.is_empty() {
        return Err("No columns selected".to_string());
    }

    // Create a dummy schema for the table
    // In a real implementation, this would come from schema introspection
    let schema = create_dummy_schema(&columns);
    let schema_arc = Arc::new(schema);

    // Create an empty RecordBatch (MemTable requires at least one partition)
    let empty_batch = RecordBatch::new_empty(schema_arc.clone());

    // Create an empty MemTable (just for schema)
    let mem_table = MemTable::try_new(schema_arc, vec![vec![empty_batch]])
        .map_err(|e| format!("Failed to create table: {}", e))?;

    // Convert to TableSource
    let table_source = provider_as_source(Arc::new(mem_table));

    // Build logical plan: Table scan + projection
    let projections: Vec<_> = columns.iter().map(|c| col(c)).collect();

    let plan = LogicalPlanBuilder::scan(
        &table_name,
        table_source,
        None, // no filter for Phase 1
    )
    .map_err(|e| format!("Failed to create table scan: {}", e))?
    .project(projections)
    .map_err(|e| format!("Failed to create projection: {}", e))?
    .build()
    .map_err(|e| format!("Failed to build plan: {}", e))?;

    Ok(plan)
}

/// Determines the root table name from entity name
///
/// For Phase 1: Just use lowercase entity name
fn determine_root_table(entity_name: EntityName) -> String {
    entity_name.lookup().to_lowercase()
}

/// Extracts column names from MergedSelectionMap
///
/// For Phase 1: Only extract ScalarField selections
fn extract_projections(merged_selection_map: &MergedSelectionMap) -> Result<Vec<String>, String> {
    let mut columns = Vec::new();

    for selection in merged_selection_map.values() {
        match selection.reference() {
            MergedServerSelection::ScalarField(scalar_field) => {
                // Add the column name
                columns.push(scalar_field.name.to_string());
            }
            MergedServerSelection::LinkedField(_) => {
                // Skip for Phase 1 (no JOINs)
            }
            MergedServerSelection::ClientObjectSelectable(_) => {
                // Skip client-only selections
            }
            MergedServerSelection::InlineFragment(_) => {
                // Skip inline fragments
            }
        }
    }

    Ok(columns)
}

/// Creates a dummy schema for the table columns
///
/// For Phase 1: All columns are TEXT (SQLite default)
fn create_dummy_schema(columns: &[String]) -> Schema {
    let fields: Vec<Field> = columns
        .iter()
        .map(|col_name| Field::new(col_name, DataType::Utf8, true))
        .collect();

    Schema::new(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use intern::string_key::Intern;
    use isograph_schema::{MergedScalarFieldSelection, NormalizationKey};
    use std::collections::BTreeMap;

    #[test]
    fn test_determine_root_table() {
        let entity_name: EntityName = "planets".intern().into();
        assert_eq!(determine_root_table(entity_name), "planets");
    }

    #[test]
    fn test_extract_projections_with_scalar_fields() {
        let mut selection_map = BTreeMap::new();

        let scalar_field = MergedScalarFieldSelection {
            name: "id".intern().into(),
            arguments: vec![],
            is_fallible: false,
        };

        selection_map.insert(
            NormalizationKey::Id,
            MergedServerSelection::ScalarField(scalar_field),
        );

        let columns = extract_projections(&selection_map).unwrap();
        assert_eq!(columns, vec!["id"]);
    }

    #[test]
    fn test_create_dummy_schema() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let schema = create_dummy_schema(&columns);

        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), "id");
        assert_eq!(schema.field(1).name(), "name");
    }
}
