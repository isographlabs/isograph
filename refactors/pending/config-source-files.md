# Config `source_files`

Requires config-discovery.md (landed). Add one field to `IsographProjectConfig`.

```rust
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub source_files: Vec<String>,
```

Origin: `crates/isograph_config/src/compilation_options.rs` `IsographProjectConfig`. Delta: that field. `#[serde(default)]` is empty `Vec`. Existing configs keep parsing.

## What the user does

Puts `"source_files": ["src/**/*.ts", "src/**/*.tsx", "!src/**/*.test.ts"]` in `isograph.config.json`.

## Tests

In `compilation_options.rs`:

- JSON without `source_files` deserializes. `source_files` is empty.
- JSON with `"source_files": ["src/**/*.ts", "!src/**/*.test.ts"]` deserializes to those two strings in order.
