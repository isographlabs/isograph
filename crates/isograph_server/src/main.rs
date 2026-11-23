use arrow::ipc::writer::FileWriter;
use axum::{extract::Json, http::StatusCode, routing::post, Router};
use datafusion::prelude::*;
use datafusion_substrait::logical_plan::consumer;
use prost::Message;
use serde::Deserialize;
use std::collections::HashMap;
use substrait::proto::Plan;

#[derive(Debug, Deserialize)]
struct QueryRequest {
    plan_id: String,
    params: HashMap<String, serde_json::Value>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/query", post(execute_query))
        .route("/health", axum::routing::get(health_check));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("Failed to bind to port 8080");

    tracing::info!("isograph-server listening on 0.0.0.0:8080");

    axum::serve(listener, app).await.expect("Server failed");
}

async fn execute_query(Json(req): Json<QueryRequest>) -> Result<Vec<u8>, StatusCode> {
    tracing::info!(
        "Received query request: plan_id={}, params={:?}",
        req.plan_id,
        req.params
    );

    // 1. Load Substrait plan from disk
    let plan_path = format!(
        "__isograph/{}/query_plan.bin",
        req.plan_id.replace("__", "/")
    );
    let plan_contents = std::fs::read_to_string(&plan_path).map_err(|e| {
        tracing::error!("Failed to read plan file {}: {}", plan_path, e);
        StatusCode::NOT_FOUND
    })?;

    // Decode base64 (Phase 1 uses base64 encoding for binary data)
    use base64::prelude::*;
    let plan_bytes = BASE64_STANDARD.decode(plan_contents.trim()).map_err(|e| {
        tracing::error!("Failed to decode base64 Substrait plan: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // 2. Deserialize Substrait → LogicalPlan
    let substrait_plan = Plan::decode(&plan_bytes[..]).map_err(|e| {
        tracing::error!("Failed to decode Substrait plan: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let ctx = SessionContext::new();
    let state = ctx.state();

    let logical_plan = consumer::from_substrait_plan(&state, &substrait_plan)
        .await
        .map_err(|e| {
            tracing::error!("Failed to convert from Substrait: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 3. TODO: Bind parameters (Phase 2)
    // For Phase 1, we have no WHERE clause, so no parameters to bind

    // 4. Execute query
    let df = ctx.execute_logical_plan(logical_plan).await.map_err(|e| {
        tracing::error!("Failed to execute logical plan: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let batches = df.collect().await.map_err(|e| {
        tracing::error!("Failed to collect results: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if batches.is_empty() {
        tracing::warn!("Query returned no batches");
        return Ok(vec![]);
    }

    // 5. Serialize to Arrow IPC
    let mut buffer = Vec::new();
    let mut writer = FileWriter::try_new(&mut buffer, &batches[0].schema()).map_err(|e| {
        tracing::error!("Failed to create Arrow writer: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    for batch in batches {
        writer.write(&batch).map_err(|e| {
            tracing::error!("Failed to write batch: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    writer.finish().map_err(|e| {
        tracing::error!("Failed to finish Arrow writer: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        "Query executed successfully, returning {} bytes",
        buffer.len()
    );
    Ok(buffer)
}

async fn health_check() -> &'static str {
    "OK"
}
