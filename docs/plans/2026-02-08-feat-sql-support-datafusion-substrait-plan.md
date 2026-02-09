---
title: Add SQL Support via DataFusion and Substrait
type: feat
date: 2026-02-08
---

# Add SQL Support via DataFusion and Substrait

## Overview

Enable Isograph to compile ISO literals into SQL queries using DataFusion's query engine. At compile-time, build LogicalPlans with parameter placeholders, serialize them as Substrait binary format, and persist to disk. At runtime, Next.js app calls a Rust endpoint that reads the Substrait plan, binds parameters, uses datafusion-federation to execute against a database, and returns Arrow IPC format.

## Problem Statement

Current work on the `ch1ffa/sql` branch has basic SQL query generation but:
- ❌ No FROM clause, JOINs, or WHERE variables
- ❌ **Critical**: No query dispatch/execution layer (0% implemented)

We need a smarter approach than SQL string generation:
- Use DataFusion to build optimized LogicalPlans at compile-time
- Serialize as Substrait (cross-language query format)
- Execute at runtime via datafusion-federation

## Proposed Solution

### Architecture Flow

```
COMPILE TIME (Rust)
─────────────────────────────────────────────────
ISO Literal → Parse AST → Build DataFusion LogicalPlan → Serialize to Substrait → Write .bin file

Generated: __isograph/<component>/query_plan.bin


RUNTIME (Next.js → Rust → Database)
─────────────────────────────────────────────────
Next.js fetch → POST /query { plan_id, params }
    ↓
Rust endpoint (Axum)
    ├─ Load Substrait from disk
    ├─ Deserialize to LogicalPlan
    ├─ Bind parameters ($1 ← params.id)
    ├─ datafusion-federation → dispatch to database
    ├─ Execute query → RecordBatch
    └─ Serialize to Arrow IPC → HTTP response
    ↓
Next.js receives Arrow IPC bytes
    ├─ apache-arrow JS library
    ├─ Deserialize to Table
    └─ Convert to JS objects for Isograph store
```

## Technical Integration Points

### 1. Compile-Time: SQL Network Protocol

**File**: `crates/sql_network_protocol/src/query_generation/logical_plan_builder.rs`

Build DataFusion LogicalPlan from Isograph's `MergedSelectionMap`:

```rust
use datafusion_expr::{col, placeholder, lit, LogicalPlanBuilder};

pub fn build_logical_plan(
    merged_selection_map: &MergedSelectionMap,
    schema: &DataModelSchema,
) -> Result<LogicalPlan> {
    // Start with table scan
    let table_name = determine_root_table(merged_selection_map)?;
    let table_source = create_table_source(table_name, schema)?;

    let mut builder = LogicalPlanBuilder::scan(table_name, table_source, None)?;

    // Add WHERE clauses with placeholders
    for (param_name, param_type) in extract_parameters(merged_selection_map) {
        let filter_expr = col("id").eq(placeholder(&format!("${}", param_name)));
        builder = builder.filter(filter_expr)?;
    }

    // Add JOINs for linked fields
    for linked_field in extract_linked_fields(merged_selection_map) {
        let (join_table, join_on) = resolve_foreign_key(linked_field, schema)?;
        builder = builder.join(
            join_table,
            JoinType::Left,
            vec![join_on.left_col],
            vec![join_on.right_col],
            None,
        )?;
    }

    // Project selected columns
    let projections = extract_projections(merged_selection_map)?;
    builder = builder.project(projections)?;

    builder.build()
}
```

**Key Functions to Implement**:
- `determine_root_table()` - Extract table name from entity type
- `extract_parameters()` - Find all `$var` in selections
- `extract_linked_fields()` - Identify fields that need JOINs
- `resolve_foreign_key()` - Map field to FK relationship
- `extract_projections()` - Build column list with aliases

### 2. Compile-Time: Substrait Serialization

**File**: `crates/sql_network_protocol/src/substrait/serialize.rs`

```rust
use datafusion_substrait::logical_plan::producer;
use substrait::proto::Plan;
use prost::Message;

pub fn serialize_to_substrait(
    logical_plan: &LogicalPlan,
    session_state: &SessionState,
) -> Result<Vec<u8>> {
    // Convert to Substrait plan
    let substrait_plan = producer::to_substrait_plan(logical_plan, session_state)?;

    // Serialize to bytes
    Ok(substrait_plan.encode_to_vec())
}

pub fn write_substrait_artifact(
    plan_bytes: Vec<u8>,
    output_path: &Path,
) -> Result<()> {
    std::fs::write(output_path, plan_bytes)?;
    Ok(())
}
```

**Integration with Artifact Generation**:

**File**: `crates/artifact_content/src/generate_artifacts.rs`

```rust
// When generating artifacts for SQL profile
if TypeId::of::<TCompilationProfile>() == TypeId::of::<SQLAndJavascriptProfile>() {
    // Build LogicalPlan
    let logical_plan = build_logical_plan(&merged_selection_map, schema)?;

    // Serialize to Substrait
    let plan_bytes = serialize_to_substrait(&logical_plan, &session_state)?;

    // Write to __isograph/<component>/query_plan.bin
    let artifact_path = output_dir.join("query_plan.bin");
    write_substrait_artifact(plan_bytes, &artifact_path)?;
}
```

### 3. Runtime: Rust Endpoint (isograph-server)

**File**: `crates/isograph_server/src/main.rs`

```rust
use axum::{Router, routing::post, Json, http::StatusCode};
use datafusion::prelude::*;
use datafusion_substrait::logical_plan::consumer;
use datafusion_federation::default_session_state;
use substrait::proto::Plan;
use prost::Message;
use arrow::ipc::writer::FileWriter;

#[derive(serde::Deserialize)]
struct QueryRequest {
    plan_id: String,              // e.g. "User__getById"
    params: HashMap<String, serde_json::Value>,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/query", post(execute_query));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn execute_query(
    Json(req): Json<QueryRequest>
) -> Result<Vec<u8>, StatusCode> {
    // 1. Load Substrait plan from disk
    let plan_path = format!("__isograph/{}/query_plan.bin", req.plan_id.replace("__", "/"));
    let plan_bytes = std::fs::read(&plan_path)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 2. Deserialize Substrait → LogicalPlan
    let substrait_plan = Plan::decode(&plan_bytes[..])
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let state = default_session_state(); // datafusion-federation enabled state
    let logical_plan = consumer::from_substrait_plan(&state, &substrait_plan)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Bind parameters
    let param_values: Vec<(&str, ScalarValue)> = req.params.iter()
        .map(|(k, v)| (k.as_str(), json_to_scalar_value(v)))
        .collect();

    let bound_plan = logical_plan.with_param_values(param_values)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 4. Execute via datafusion-federation
    let ctx = SessionContext::new_with_state(state);
    let df = ctx.execute_logical_plan(bound_plan)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let batches = df.collect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 5. Serialize to Arrow IPC
    let mut writer = FileWriter::try_new(Vec::new(), &batches[0].schema())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for batch in batches {
        writer.write(&batch)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    writer.finish()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(writer.into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

fn json_to_scalar_value(value: &serde_json::Value) -> ScalarValue {
    match value {
        serde_json::Value::String(s) => ScalarValue::Utf8(Some(s.clone())),
        serde_json::Value::Number(n) if n.is_i64() => ScalarValue::Int64(Some(n.as_i64().unwrap())),
        serde_json::Value::Number(n) if n.is_f64() => ScalarValue::Float64(Some(n.as_f64().unwrap())),
        serde_json::Value::Bool(b) => ScalarValue::Boolean(Some(*b)),
        serde_json::Value::Null => ScalarValue::Null,
        _ => ScalarValue::Utf8(Some(value.to_string())),
    }
}
```

**Cargo.toml**:
```toml
[package]
name = "isograph_server"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
datafusion = "52.1"
datafusion-substrait = "52.1"
datafusion-federation = "0.4.14"
substrait = "0.30"
prost = "0.13"
arrow = "53.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 4. Runtime: Next.js Client Integration

**File**: `libs/isograph-react/src/network.ts`

```typescript
import { tableFromIPC } from 'apache-arrow';

export async function executeQuery(
  planId: string,
  params: Record<string, any>
): Promise<any[]> {
  // 1. Call Rust endpoint
  const response = await fetch('http://localhost:8080/query', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ plan_id: planId, params }),
  });

  if (!response.ok) {
    throw new Error(`Query failed: ${response.statusText}`);
  }

  // 2. Get Arrow IPC bytes
  const arrayBuffer = await response.arrayBuffer();

  // 3. Deserialize with apache-arrow
  const table = tableFromIPC(new Uint8Array(arrayBuffer));

  // 4. Convert to JS objects
  return table.toArray();
}
```

**Install Arrow JS**:
```bash
npm install apache-arrow
```

**Usage in Next.js**:
```typescript
import { executeQuery } from '@isograph/react';

export default async function UserPage({ params }: { params: { id: string } }) {
  const users = await executeQuery('User__getById', { id: params.id });

  return (
    <div>
      <h1>{users[0].name}</h1>
      <p>{users[0].email}</p>
    </div>
  );
}
```

### 5. Type Mapping: Arrow → TypeScript

**Key Mappings**:
| Arrow DataType | TypeScript Type |
|----------------|-----------------|
| `Utf8` | `string` |
| `Int32` | `number` |
| `Int64` | `number` (⚠️ precision loss) or `bigint` |
| `Float64` | `number` |
| `Boolean` | `boolean` |
| `Timestamp(Microsecond, UTC)` | `Date` |
| `List(Utf8)` | `string[]` |

**Null Handling**:
- Arrow nullable field → TypeScript `field: string | null`
- Arrow non-nullable → TypeScript `field: string`

**TypeScript Codegen Update** (`artifact_content/src/generate_artifacts.rs`):
```rust
// Generate TypeScript types from Arrow schema
fn arrow_type_to_typescript(arrow_type: &DataType) -> String {
    match arrow_type {
        DataType::Utf8 => "string".to_string(),
        DataType::Int32 | DataType::Int64 => "number".to_string(),
        DataType::Float64 => "number".to_string(),
        DataType::Boolean => "boolean".to_string(),
        DataType::Timestamp(_, _) => "Date".to_string(),
        DataType::List(inner) => format!("{}[]", arrow_type_to_typescript(inner.data_type())),
        _ => "unknown".to_string(),
    }
}
```

## Implementation Checklist

### Compile-Time Components

**sql_network_protocol crate**:
- [x] `src/query_generation/logical_plan_builder.rs` - Build LogicalPlan from MergedSelectionMap (Phase 1: simple SELECT)
- [ ] `src/query_generation/foreign_key_resolver.rs` - Map linked fields to JOINs (Phase 3)
- [ ] `src/query_generation/parameter_binding.rs` - Extract parameters and create placeholders (Phase 2)
- [x] `src/substrait/serialize.rs` - Serialize LogicalPlan to Substrait binary
- [ ] `src/schema/parse_sql_schema.rs` - Parse SQL schema definition (future)

**Integration with compiler**:
- [ ] Update `artifact_content/src/generate_artifacts.rs` to call Substrait serialization
- [ ] Write query_plan.bin to artifact directory

### Runtime Components

**isograph_server crate**:
- [x] `src/main.rs` - Axum HTTP server with /query endpoint
- [x] Load Substrait from disk
- [x] Deserialize Substrait → LogicalPlan
- [ ] Bind runtime parameters (Phase 2)
- [ ] Execute with datafusion-federation (using datafusion directly for Phase 1)
- [x] Serialize response to Arrow IPC

**Next.js integration**:
- [ ] Install `apache-arrow` npm package
- [ ] Update `@isograph/react` network function to call Rust endpoint
- [ ] Deserialize Arrow IPC to JS objects
- [ ] Handle errors (404 for missing plan, 500 for execution errors)

### Testing

- [ ] E2E test: Compile simple SELECT query → Execute → Verify JS object
- [ ] Test parameterized query (e.g., WHERE id = $1)
- [ ] Test query with JOIN (linked field)
- [ ] Test null handling (nullable vs non-null columns)
- [ ] Test error cases (missing plan, invalid params)

## Acceptance Criteria

- [ ] ISO literal `field User.getById($id: ID!) { id, name, email }` compiles to Substrait binary
- [ ] Substrait file written to `__isograph/User/getById/query_plan.bin`
- [ ] Next.js can call `executeQuery('User__getById', { id: '123' })`
- [ ] Rust endpoint loads Substrait, binds `$1 = 123`, executes against Postgres
- [ ] Response is Arrow IPC format
- [ ] Next.js deserializes to JS: `[{ id: '123', name: 'Alice', email: 'alice@example.com' }]`
- [ ] TypeScript types generated match Arrow schema
- [ ] Query with JOIN works: `field User.profile { user { id, name }, avatar { url } }`

## Key Technical Decisions

### 1. Foreign Key Declaration

**Option A: Schema annotations**
```graphql
type User @sqlTable(name: "users") {
  posts: [Post!]! @sqlForeignKey(table: "posts", column: "user_id", references: "id")
}
```

**Option B: Convention-based inference**
- Field name `posts` → table `posts`
- FK column `user_id` (snake_case of parent type + _id)

**Recommendation**: Start with Option A (explicit) for correctness, add Option B later for convenience.

### 2. Parameter Type Coercion

**Strict mode** (recommended):
- Client sends String, plan expects Int → Error 400
- Forces type correctness at compile-time

**Permissive mode**:
- Attempt coercion (e.g., "123" → 123)
- Risk of unexpected behavior

**Recommendation**: Strict mode, validate types in Rust endpoint.

### 3. Arrow Library Choice (JS)

**apache-arrow**: Official, 112K downloads/week, feature-complete
**flechette**: 1.3-7x faster, smaller bundle (43KB vs 163KB)

**Recommendation**: Start with `apache-arrow` (mature, well-documented), benchmark later and switch to `flechette` if performance matters.

## Dependencies

**Rust crates**:
- `datafusion` (v52.1.0) - LogicalPlan building
- `datafusion-substrait` (v52.1.0) - Substrait serialization
- `datafusion-federation` (v0.4.14) - Query dispatch
- `axum` (v0.7) - HTTP server
- `prost` (v0.13) - Protobuf for Substrait
- `arrow` (v53.0) - Arrow IPC serialization

**JavaScript packages**:
- `apache-arrow` (v21.1.0) - Arrow IPC deserialization

## Risks & Mitigations

**Risk 1: Substrait doesn't support all DataFusion features**
- **Mitigation**: Test with real queries in Phase 1, document limitations

**Risk 2: Schema changes break compiled plans**
- **Mitigation**: Version artifacts, validate schema on server startup (future enhancement)

**Risk 3: Parameter type mismatches at runtime**
- **Mitigation**: Strict type validation in Rust endpoint, return 400 Bad Request

**Risk 4: Variable-length array parameters (`WHERE id IN ($ids)`)**
- **Mitigation**: Research Substrait support, may need workarounds or unsupported (document)

## Files to Create/Modify

**New Files**:
- `crates/isograph_server/src/main.rs`
- `crates/isograph_server/Cargo.toml`
- `crates/sql_network_protocol/src/query_generation/logical_plan_builder.rs`
- `crates/sql_network_protocol/src/substrait/serialize.rs`
- `libs/isograph-react/src/arrow/deserialize.ts`

**Modified Files**:
- `crates/artifact_content/src/generate_artifacts.rs` (add Substrait generation)
- `libs/isograph-react/src/network.ts` (call Rust endpoint, deserialize Arrow)
- `crates/sql_network_protocol/src/sql_network_protocol.rs` (complete implementation)

## Example End-to-End Flow

**ISO Literal**:
```typescript
iso(`
  field User.getById($id: ID!) {
    id
    name
    email
  }
`)
```

**Compile-Time**:
1. Parse → `ClientFieldDeclaration`
2. Build LogicalPlan:
   ```rust
   LogicalPlanBuilder::scan("users", table_source, None)?
       .filter(col("id").eq(placeholder("$1")))?
       .project(vec![col("id"), col("name"), col("email")])?
       .build()?
   ```
3. Serialize to Substrait → `__isograph/User/getById/query_plan.bin`

**Runtime (Next.js)**:
```typescript
const users = await executeQuery('User__getById', { id: '123' });
// users = [{ id: '123', name: 'Alice', email: 'alice@example.com' }]
```

**Server Executes**:
```sql
SELECT id, name, email FROM users WHERE id = $1
-- $1 = '123'
```

**Response**: Arrow IPC bytes → Next.js deserializes → JS array
