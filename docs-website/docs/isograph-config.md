# The Isograph configuration object

## Config file location and name

The file should be named `isograph.config.json` and located at the root of your project.

## Config file contents

An example (complete) Isograph config is as follows:

```json
{
  "project_root": "./src/components",
  "artifact_directory": "./src",
  "schema": "./backend/schema.graphql",
  "schema_extensions": ["./backend/schema-extension.graphql"],
  "options": {
    "on_invalid_id_type": "error"
  }
}
```

- All paths are relative.
- `schema` and `schema_extensions` take relative paths to files, not to folders.
- Only `project_root` and `schema` are required.
- Valid values for `on_invalid_id_type` are `ignore`, `warning` and `error`.
- `artifact_directory` defaults to `project_root`.
