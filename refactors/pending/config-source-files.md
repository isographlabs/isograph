# Config `source_files`

Requires config-discovery.md (landed). Add one field to `IsographProjectConfig`.

```rust
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
```

Origin: `crates/isograph_config/src/compilation_options.rs` `IsographProjectConfig`. Delta: that field. No `#[serde(default)]`. A missing key is a deserialize error. `"source_files": []` is an empty list. Drop `Default` from the derive.

`build_json_schema` rewrites `libs/isograph-compiler/isograph-config-schema.json`. `source_files` is required.

## What the user does

Puts `"source_files": ["src/**/*.ts", "src/**/*.tsx", "!src/**/*.test.ts"]` in `isograph.config.json`.

## Tests

In `compilation_options.rs`:

- `{}` does not deserialize.
- `{"source_files": []}` deserializes. `source_files` is empty.
- `{"source_files": ["src/**/*.ts", "!src/**/*.test.ts"]}` deserializes to those two strings in order.
