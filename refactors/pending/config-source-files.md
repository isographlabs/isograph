# Config `source_files`

Requires config-discovery.md (landed). One field on `IsographProjectConfig`: a list of glob strings naming the files that contain iso literals.

The name is `source_files`. Not `includes` (includes of what). Not `project_root` (that is already a directory).

One shippable change.

## What the user does

```json
{
  "project_root": "src",
  "schema": "./schema.graphql",
  "source_files": ["src/**/*.ts", "src/**/*.tsx", "!src/**/*.test.ts"]
}
```

Absent `source_files` is an empty list. Existing configs keep parsing.

## Types

Origin: `crates/isograph_config/src/compilation_options.rs` `IsographProjectConfig`. Delta: `source_files`.

```rust
// from crates/isograph_config/src/compilation_options.rs
#[derive(Deserialize, JsonSchema, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    pub project_root: PathBuf,
    pub artifact_directory: Option<PathBuf>,
    pub schema: PathBuf,
    #[serde(default)]
    pub schema_extensions: Vec<PathBuf>,
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub source_files: Vec<String>,
    #[serde(default)]
    pub options: ConfigFileOptions,
}
```

`#[serde(default)]` is empty `Vec`. `create_config` does not copy this field onto `CompilerConfig`. This change does not interpret the globs.

## Tests

In `compilation_options.rs`:

- Demo-shaped JSON without `source_files` deserializes. `source_files` is empty.
- The same JSON with `"source_files": ["src/**/*.ts", "!src/**/*.test.ts"]` deserializes to those two strings in order.

## Call sites

None in this change. filesystem-watcher.md reads `source_files`.
