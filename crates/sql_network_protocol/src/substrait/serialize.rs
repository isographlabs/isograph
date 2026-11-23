use datafusion::execution::SessionState;
use datafusion::logical_expr::LogicalPlan;
use datafusion_substrait::logical_plan::producer;
use std::io::Result as IoResult;
use std::path::Path;

/// Serializes a DataFusion LogicalPlan to Substrait binary format
pub fn serialize_to_substrait(
    logical_plan: &LogicalPlan,
    session_state: &SessionState,
) -> Result<Vec<u8>, String> {
    use prost::Message;

    // Convert to Substrait plan
    let substrait_plan = producer::to_substrait_plan(logical_plan, session_state)
        .map_err(|e| format!("Failed to convert to Substrait plan: {}", e))?;

    // Serialize to bytes using prost
    Ok(substrait_plan.encode_to_vec())
}

/// Writes Substrait bytes to disk
pub fn write_substrait_artifact(plan_bytes: Vec<u8>, output_path: &Path) -> IoResult<()> {
    std::fs::write(output_path, plan_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::prelude::*;

    #[tokio::test]
    async fn test_serialize_simple_plan() {
        let ctx = SessionContext::new();

        // Create a simple logical plan
        let df = ctx
            .sql("SELECT 1 as id, 'test' as name")
            .await
            .expect("Failed to create dataframe");

        let logical_plan = df.logical_plan().clone();
        let session_state = ctx.state();

        // Serialize to Substrait
        let result = serialize_to_substrait(&logical_plan, &session_state);

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
