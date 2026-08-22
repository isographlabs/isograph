# Config `includes`

Requires config-discovery.md (landed). `project_root` is a single directory. The watcher and extraction need glob arrays, negation, and (later) gitignore.

Biome's `files.includes` is the model: an array of globs, `!` prefixes a negation, last match wins, processed in order. Patterns are relative to the directory that contains the config file. `*` is one path segment, `**` is recursive.

The field name is `includes`. `project_root` stays; it is the default when `includes` is absent.

One shippable change after a load-without-panic prefactor.

## What the user does

```json
{
  "project_root": "src",
  "schema": "./schema.graphql",
  "includes": ["src/**/*.ts", "src/**/*.tsx", "!src/**/*.test.ts"]
}
```

`src/a.ts` is in scope. `src/a.test.ts` is not. `src/a.tsx` is in. Absent `includes` means `{project_root}/**/*.{js,jsx,ts,tsx}`.

## Change 1: load `IsographProjectConfig` without panic, without mkdir

`discover::IsographConfig` is an empty struct and ignores unknown fields. `isograph_config::IsographProjectConfig` is the real file and `deny_unknown_fields`. This change parses with `IsographProjectConfig`, in `isograph_config`, and `discover::load_config` returns that type.

`create_config` today panics on missing files and creates `artifact_directory` and `project_root` at parse time. Parse is not an effect. This change replaces `create_config` with a `Result`. It does not `create_dir_all`. Creating the artifact directory is an effect of generation, not of load. A missing `project_root` directory is a later watcher/compile diagnostic, not a panic at daemon start.

Origin: `crates/isograph_config/src/compilation_options.rs` `create_config`. Delta: `Result` instead of panic; no mkdir.

```rust
// from crates/isograph_config/src/compilation_options.rs
#[derive(Debug)]
pub struct Unreadable {
    pub path: PathBuf,
    pub source: std::io::Error,
}

#[derive(Debug)]
pub struct Unparseable {
    pub path: PathBuf,
    pub source: serde_json::Error,
}

#[derive(Debug)]
pub struct NotCanonical {
    pub path: PathBuf,
    pub source: std::io::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    Unreadable(Unreadable),
    #[error("could not parse {}: {}", .0.path.display(), .0.source)]
    Unparseable(Unparseable),
    #[error("could not canonicalize {}: {}", .0.path.display(), .0.source)]
    NotCanonical(NotCanonical),
}

pub fn load_project_config(
    config_location: &Path,
) -> Result<IsographProjectConfig, ConfigError> {
    let config_contents = std::fs::read_to_string(config_location).map_err(|source| {
        ConfigError::Unreadable(Unreadable {
            path: config_location.to_owned(),
            source,
        })
    })?;
    serde_json::from_str(config_contents.reference()).map_err(|source| {
        ConfigError::Unparseable(Unparseable {
            path: config_location.to_owned(),
            source,
        })
    })
}
```

`discover::load_config` becomes a call to `isograph_config::load_project_config` after `config_json` (so `.js` / `.ts` still stringify first). `DiscoverError::Load` wraps `ConfigError` for JSON parse, and keeps the existing JS-runtime variants.

`create_config` remains for callers that still want `CompilerConfig` (artifact paths, canonical schema). It uses `load_project_config` and returns `Result<CompilerConfig, ConfigError>`.

`thiserror` in `isograph_config`: workspace `thiserror` is 1.0; `isograph_cli` uses 2. Use workspace `thiserror` in `isograph_config`.

`isograph_cli` depends on `isograph_config`.

E2e today writes `{}\n`. After this change that file is `Unparseable` / missing fields. Every e2e fixture becomes:

```json
{
  "project_root": ".",
  "schema": "./schema.graphql"
}
```

plus a `schema.graphql` file of `type Query { __typename: String }\n`. `Daemon::start` writes both.

Tests of `load_project_config`:

- `{}\n` is `Unparseable`.
- A missing file is `Unreadable`.
- A valid demo-shaped JSON is `Ok` with `includes == None`.

## Change 2: `includes` and `SourceScope`

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
    /// Glob patterns relative to the config file's directory. `!` at the start of a pattern
    /// excludes. Last match wins. When absent, the default is `{project_root}/**/*.{js,jsx,ts,tsx}`.
    pub includes: Option<Vec<String>>,
    #[serde(default)]
    pub options: ConfigFileOptions,
}
```

`includes: Option<Vec<String>>`. `None` is the default glob. `Some(vec![])` is nothing in scope. Those are not a bool.

`project_root` stays required. Existing demo configs keep working. `includes` absent means one include: `{project_root}/**/*.{js,jsx,ts,tsx}`.

Always out of scope for source walking, hardcoded, not a config flag: any path whose components include `node_modules` or `__isograph`. Schema and schema_extensions are still in scope even if they sit in `node_modules`.

`.gitignore` is always honored, via the `ignore` crate, in the watcher walk (filesystem-watcher.md). This doc only matches paths.

```rust
// from crates/isograph_cli/src/scope.rs
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};

#[derive(Clone)]
pub enum GlobRule {
    Include(GlobMatcher),
    Exclude(GlobMatcher),
}

#[derive(Clone)]
pub struct SourceScope {
    pub config_dir: PathBuf,
    pub rules: Vec<GlobRule>,
    pub always: AlwaysWatched,
    pub watch_roots: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct AlwaysWatched {
    pub config: PathBuf,
    pub schema: PathBuf,
    pub schema_extensions: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct GlobError {
    pub pattern: String,
    pub source: globset::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("invalid glob {}: {}", .0.pattern, .0.source)]
    Glob(GlobError),
    #[error("config path {} has no parent directory", .0.display())]
    NoParent(PathBuf),
}

impl SourceScope {
    pub fn from_config(
        config_path: &Path,
        config: &isograph_config::IsographProjectConfig,
    ) -> Result<Self, ScopeError> {
        let config_dir = match config_path.parent() {
            Some(dir) => dir.to_owned(),
            None => return ScopeError::NoParent(config_path.to_owned()).wrap_err(),
        };
        let patterns = match &config.includes {
            Some(patterns) => patterns.clone(),
            None => {
                let root = config.project_root.to_string_lossy().replace('\\', "/");
                format!("{root}/**/*.{{js,jsx,ts,tsx}}").wrap_vec()
            }
        };
        let mut rules = Vec::new();
        let mut watch_roots = Vec::new();
        for pattern in &patterns {
            rules.push(compile_rule(pattern)?);
            match pattern.strip_prefix('!') {
                Some(_) => {}
                None => watch_roots.push(watch_root_for(config_dir.reference(), pattern)),
            }
        }
        let schema = config_dir.join(config.schema.reference());
        let schema_extensions = config
            .schema_extensions
            .iter()
            .map(|p| config_dir.join(p.reference()))
            .collect::<Vec<_>>();
        let always = AlwaysWatched {
            config: config_path.to_owned(),
            schema,
            schema_extensions,
        };
        for always_path in always.iter() {
            if let Some(parent) = always_path.parent() {
                watch_roots.push(parent.to_owned());
            }
        }
        watch_roots.sort();
        watch_roots.dedup();
        Self {
            config_dir,
            rules,
            always,
            watch_roots,
        }
        .wrap_ok()
    }

    pub fn contains(&self, path: &Path) -> bool {
        if self.always.contains(path) {
            return true;
        }
        if has_component(path, "node_modules") || has_component(path, "__isograph") {
            return false;
        }
        let relative = match path.strip_prefix(self.config_dir.reference()) {
            Ok(relative) => relative,
            Err(_) => return false,
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let mut included = false;
        for rule in &self.rules {
            match rule {
                GlobRule::Include(matcher) if matcher.is_match(relative.as_str()) => {
                    included = true;
                }
                GlobRule::Exclude(matcher) if matcher.is_match(relative.as_str()) => {
                    included = false;
                }
                _ => {}
            }
        }
        included
    }
}

impl AlwaysWatched {
    fn contains(&self, path: &Path) -> bool {
        path == self.config.as_path()
            || path == self.schema.as_path()
            || self.schema_extensions.iter().any(|p| p.as_path() == path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.config)
            .chain(std::iter::once(&self.schema))
            .chain(self.schema_extensions.iter())
    }
}

fn has_component(path: &Path, name: &str) -> bool {
    path.components().any(|c| c.as_os_str() == name)
}

fn compile_rule(pattern: &str) -> Result<GlobRule, ScopeError> {
    let body = match pattern.strip_prefix('!') {
        Some(rest) => rest,
        None => pattern,
    };
    let matcher = Glob::new(body)
        .map_err(|source| {
            ScopeError::Glob(GlobError {
                pattern: pattern.to_owned(),
                source,
            })
        })?
        .compile_matcher();
    match pattern.strip_prefix('!') {
        Some(_) => GlobRule::Exclude(matcher).wrap_ok(),
        None => GlobRule::Include(matcher).wrap_ok(),
    }
}

fn watch_root_for(config_dir: &Path, pattern: &str) -> PathBuf {
    let prefix = match pattern.split(['*', '{']).next() {
        Some(prefix) => prefix.trim_end_matches('/'),
        None => "",
    };
    if prefix.is_empty() {
        config_dir.to_owned()
    } else {
        config_dir.join(prefix)
    }
}
```

`config_path.parent()` of a canonical absolute file is `Some`. `None` is `ScopeError::NoParent`. Brace expansion in the default `{js,jsx,ts,tsx}` is globset's `{a,b}` support.

Exclude patterns do not add watch roots. `always` paths add their parent. Dedup the `Vec`. `split` always yields at least one piece; the `None` arm is the empty prefix, same as watching `config_dir`. Do not `expect` on `next`.

`contains` returning `bool` is std `is_match` folded over ordered rules. The two outcomes are in-scope and out-of-scope; `GlobRule` already names those. Raise: `contains` is a yes/no against a path; the cases are the `GlobRule` variants applied in order, not a domain enum on the result.

### Cargo

```toml
# from Cargo.toml workspace.dependencies
globset = "0.4"
ignore = "0.4"
```

`isograph_cli` depends on `globset` and `isograph_config`. `ignore` is used in filesystem-watcher.md. Add `ignore` there, not here.

### Tests

Scope unit tests, paths relative to a temp `config_dir`:

- Default includes: `src/a.ts` under `project_root: "src"` is in. `src/a.rs` is out. `README.md` is out.
- `includes: ["src/**/*.ts"]`: `src/a.ts` in, `src/a.tsx` out, `lib/a.ts` out.
- `includes: ["**", "!**/*.test.ts"]`: `src/a.ts` in, `src/a.test.ts` out.
- Last match wins: `["**", "!**/*.test.ts", "**/special.test.ts"]`: `special.test.ts` in.
- `includes: []`: nothing in, except `always` (config, schema).
- `node_modules/foo.ts` out even if `includes: ["**"]`.
- `__isograph/foo.ts` out even if `includes: ["**"]`.
- Schema path `vendor/schema.graphql` in even if under a negated glob.
- Path outside `config_dir` out, unless it is an `always` path.
- Empty file name matching: `includes: ["src/**"]` and path `src/` (a directory). Test files only.
- Windows separators in the path become `/` before match.

`from_config` of a bad glob (`[`) is `ScopeError::Glob`.

## Call sites

- `discover::load_config` -> `isograph_config::load_project_config`.
- filesystem-watcher.md -> `SourceScope::from_config` and `SourceScope::contains`.
