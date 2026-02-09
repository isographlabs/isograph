# E2E Tests for SQLite Demo

End-to-end tests for Phase 1 SQL support using Playwright.

## Setup

1. **Install dependencies** (from repo root):
   ```bash
   pnpm install
   ```

2. **Build the Isograph compiler**:
   ```bash
   cargo build
   ```

3. **Initialize test database**:
   ```bash
   ./test-fixtures/databases/init-db.sh
   ```

4. **Generate artifacts** (compile ISO literals → Substrait plans):
   ```bash
   cd demos/sqlite-demo
   npm run iso
   ```

   This should generate:
   - `src/components/__isograph/planets/HomePage/query_plan.bin` (base64-encoded Substrait)
   - TypeScript artifacts (entrypoint.ts, normalization_ast.ts, etc.)

5. **Start isograph-server** (in a separate terminal):
   ```bash
   cd crates/isograph_server
   cargo run
   ```

   Server should start on http://localhost:8080

## Running Tests

### Run all E2E tests
```bash
npm run test:e2e
```

### Run with UI mode (interactive)
```bash
npm run test:e2e:ui
```

### Run in headed mode (see browser)
```bash
npm run test:e2e:headed
```

## Test Structure

### `phase1-sql.spec.ts`
Tests for Phase 1 SQL support:

1. **Load and display planet data**
   - Verifies app loads successfully
   - Checks that SQL query results are displayed

2. **Verify isograph-server requests**
   - Intercepts network requests
   - Validates request format (plan_id, params)

3. **Error handling**
   - Ensures graceful degradation when server unavailable

4. **Artifact generation**
   - Verifies compiler generated necessary files

## Phase 1 Architecture

```
┌─────────────────────────────────────────────────────────┐
│ 1. ISO Literal → Compiler                               │
│    field planets.HomePage @component { ... }            │
└─────────────────────┬───────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────────┐
│ 2. Compiler generates:                                   │
│    - LogicalPlan (DataFusion)                           │
│    - Substrait binary (base64-encoded)                  │
│    - TypeScript artifacts                               │
│    Output: query_plan.bin, entrypoint.ts, etc.         │
└─────────────────────┬───────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────────┐
│ 3. React App makes request:                             │
│    POST http://localhost:8080/query                     │
│    { plan_id: "planets__HomePage", params: {} }        │
└─────────────────────┬───────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────────┐
│ 4. isograph-server:                                      │
│    - Reads query_plan.bin                               │
│    - Decodes base64 → Substrait binary                  │
│    - Deserializes → LogicalPlan                         │
│    - Executes via DataFusion                            │
│    - Returns Arrow IPC                                  │
└─────────────────────┬───────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────────┐
│ 5. React App:                                            │
│    - Deserializes Arrow IPC → JS objects               │
│    - Normalizes data                                    │
│    - Renders UI                                         │
└─────────────────────────────────────────────────────────┘
```

## Troubleshooting

### Compiler errors
- Ensure `cargo build` completed successfully
- Check that `isograph.config.json` has `"kind": "sql"`

### Server not responding
- Verify server is running: `curl http://localhost:8080/health`
- Check server logs for errors

### Artifacts not generated
- Run `npm run iso` from sqlite-demo directory
- Check for compiler errors in output

### Tests failing
- Ensure dev server is running (Playwright will auto-start it)
- Check browser console for errors: `npm run test:e2e:headed`

## Next Steps (Phase 2 & 3)

**Phase 2**: Add WHERE clauses with parameters
```typescript
// Example: Filter by planet ID
const { fragmentReference } = useLazyReference(
  iso(`entrypoint planets.PlanetDetail`),
  { id: 1 }  // Parameter binding
);
```

**Phase 3**: Add JOINs for linked fields
```
field planets.PlanetWithPeople @component {
  name
  people {  # Requires JOIN
    name
    height
  }
}
```
