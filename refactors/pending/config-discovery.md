# Config discovery

Requires isograph-cli.md. The daemon is keyed to one config: `--config` when given, otherwise the nearest `isograph.config.json`, `isograph.config.js`, or `isograph.config.ts` at or above the current directory. At one directory, that order: json, then js, then ts. Two paths to one file are one daemon. The daemon still parks. It logs the config it is.

Walk-up is the babel plugin's walk, with two extra names. The babel plugin still only opens `isograph.config.json`.

## What the user does

```
$ cd app/src/components && isograph
$ isograph status
isograph (/Users/x/app/isograph.config.json): running, pid 12345
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json"}}
$ isograph stop
```

Every verb resolves the config the same way, so `isograph stop` in a subdirectory stops the daemon that `isograph` in that subdirectory started.

```
$ cd /tmp && isograph status
error: no isograph.config.json, isograph.config.js, or isograph.config.ts at or above /tmp; create one, or pass one using the --config flag
```

```
$ isograph --config ./isograph.config.json
$ isograph --config ./isograph.config.ts
$ isograph --config /other/project/isograph.config.js status
```

## Change 1: find the config, key the instance to it

New module `crates/isograph_cli/src/discover.rs`. `App::Id` becomes the config flag. `App::DaemonArgs` can be `NoArgs` now that `Id` is not `NoArgs` (the clap group-name collision goes away).

`crates/isograph_cli/Cargo.toml` gains `thiserror = "2"`.

```rust
// from crates/isograph_cli/src/discover.rs
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use freddie_cli::Instance;
use prelude::Postfix;

pub const CONFIG_FILE_NAMES: &[&str] = &[
    "isograph.config.json",
    "isograph.config.js",
    "isograph.config.ts",
];

pub struct ConfigNotReadable {
    pub path: PathBuf,
    pub source: std::io::Error,
}

pub struct ConfigNotFound {
    pub start: PathBuf,
}

pub struct NoCurrentDir {
    pub source: std::io::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("could not resolve the config at {}: {}", .0.path.display(), .0.source)]
    ConfigNotReadable(ConfigNotReadable),
    #[error("no isograph.config.json, isograph.config.js, or isograph.config.ts at or above {}; create one, or pass one using the --config flag", .0.start.display())]
    NotFound(ConfigNotFound),
    #[error("could not read the current directory: {}", .0.source)]
    NoCurrentDir(NoCurrentDir),
    #[error(transparent)]
    NoUserDir(#[from] freddie_cli::NoUserDir),
}

/// The canonical path of the config `flag` names, or of the nearest isograph config
/// at or above the current directory. Canonical, so two paths to one file name one daemon.
pub fn config_path(flag: Option<&Path>) -> Result<PathBuf, DiscoverError> {
    let named = match flag {
        Some(path) => path.to_owned(),
        None => {
            let start = std::env::current_dir()
                .map_err(|source| DiscoverError::NoCurrentDir(NoCurrentDir { source }))?;
            match nearest_config(start.reference()) {
                Some(found) => found,
                None => return DiscoverError::NotFound(ConfigNotFound { start }).wrap_err(),
            }
        }
    };
    named.canonicalize().map_err(|source| {
        DiscoverError::ConfigNotReadable(ConfigNotReadable { path: named, source })
    })
}

fn nearest_config(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|dir| {
        CONFIG_FILE_NAMES
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

pub fn config_and_instance(flag: Option<&Path>) -> Result<(PathBuf, Instance), DiscoverError> {
    let config = config_path(flag)?;
    let instance = Instance::named(
        "isograph",
        slug(config.reference()),
        config.display().to_string(),
    )?;
    (config, instance).wrap_ok()
}

/// A filename: `Instance::named` puts this in a path, so it cannot contain `/`.
fn slug(config: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);
    format!("isograph-{:016x}", hasher.finish())
}
```

`thiserror` on `DiscoverError` for the `Display` / `Error` impls. `NoUserDir` converts with `From`.

`config_and_instance` computes the pair together so no caller can key an instance to the wrong config.

```rust
// from crates/isograph_cli/src/main.rs
mod discover;

#[derive(clap::Args, Debug)]
pub struct ConfigFlag {
    /// Path to the isograph config. When absent, the nearest isograph.config.json, .js, or .ts
    /// at or above the current directory.
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,
}

pub struct Isograph;

impl App for Isograph {
    type Id = ConfigFlag;
    type DaemonArgs = NoArgs;

    const NAME: &'static str = "isograph";

    fn instance(id: &ConfigFlag) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        let (_, instance) = discover::config_and_instance(id.config.as_deref())?;
        instance.wrap_ok()
    }

    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((config, _)) => {
                tracing::info!(config = %config.display(), "isograph daemon up");
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
}
```

`IsographArgs` is deleted. The daemon still parks. `status` / `logs` / `stop` in a subdirectory find the right daemon because they go through `instance`.

The contents of the config file are not read.

## Tests

Unit tests on `nearest_config` and `slug`, in `discover.rs`, using a temp directory.

- A `isograph.config.json` in `project/` is found from `project/src/components`.
- A `isograph.config.js` in `project/` is found from a subdirectory when no json is there.
- A `isograph.config.ts` in `project/` is found from a subdirectory when no json or js is there.
- In one directory that has both json and js, json is the one found.
- A directory with no config above it is `None`.
- Two canonical paths get two slugs; one path gets the same slug twice.

Binary tests, same private-`HOME` harness as isograph-cli.md:

- `start` in a fixture project, then `status` names that config and reports running.
- `start` in a subdirectory of the fixture finds the same daemon.
- `start` with no config above cwd fails.
- `--config` pointing at a missing file fails.
- Two fixture projects are two daemons: `status` in each reports a different pid; `stop` in one leaves the other running.
- `logs` contains the canonical config path.

## Change 2: JS and TS configs become JSON

A `.json` file is already JSON: read it. A `.js` or `.ts` file is a module. Run it with an executor, take the export (`export default` or `module.exports`), `JSON.stringify` that value, and that string is the JSON.

The executor is one command, the same shape as barnum (`"bun"` or `"node <tsx/cli>"`). First that is present wins:

1. `bun`
2. `deno`
3. `node` plus `tsx/cli` (`node_modules/tsx/dist/cli.mjs`) walking up from the config file, which is barnum's `require.resolve("tsx/cli")`
4. `pnpm exec tsx`
5. `npx tsx`
6. `yarn exec tsx`
7. `node`

```rust
// from crates/isograph_cli/src/discover.rs
use std::process::Command;

enum ConfigSyntax {
    Json,
    JavaScript,
    TypeScript,
}

enum EvalKind {
    DashE,
    DenoEval,
}

struct Executor {
    program: String,
    prefix: Vec<String>,
    eval: EvalKind,
}

impl Executor {
    fn display(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.prefix.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub struct UnknownSyntax {
    pub path: PathBuf,
}

pub struct NoJsRuntime {
    pub path: PathBuf,
}

pub struct JsFailed {
    pub path: PathBuf,
    pub program: String,
    pub stderr: String,
}

pub struct JsIo {
    pub path: PathBuf,
    pub program: String,
    pub source: std::io::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    Unreadable(ConfigNotReadable),
    #[error("{} is not .json, .js, or .ts", .0.path.display())]
    UnknownSyntax(UnknownSyntax),
    #[error("no bun, deno, tsx, npx, pnpm, yarn, or node; needed to load {}", .0.path.display())]
    NoJsRuntime(NoJsRuntime),
    #[error("{} failed on {}: {}", .0.program, .0.path.display(), .0.stderr)]
    JsFailed(JsFailed),
    #[error("could not run {} on {}: {}", .0.program, .0.path.display(), .0.source)]
    JsIo(JsIo),
}

fn syntax(path: &Path) -> Option<ConfigSyntax> {
    match path.extension().and_then(std::ffi::OsStr::to_str) {
        Some("json") => ConfigSyntax::Json.wrap_some(),
        Some("js") => ConfigSyntax::JavaScript.wrap_some(),
        Some("ts") => ConfigSyntax::TypeScript.wrap_some(),
        _ => None,
    }
}

fn on_path(program: &str) -> bool {
    Command::new(program).arg("--version").output().is_ok()
}

fn tsx_cli(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|dir| {
        let mjs = dir.join("node_modules/tsx/dist/cli.mjs");
        mjs.is_file().then_some(mjs)
    })
}

fn first_executor(config: &Path) -> Option<Executor> {
    if on_path("bun") {
        return Executor {
            program: "bun".to_owned(),
            prefix: Vec::new(),
            eval: EvalKind::DashE,
        }
        .wrap_some();
    }
    if on_path("deno") {
        return Executor {
            program: "deno".to_owned(),
            prefix: Vec::new(),
            eval: EvalKind::DenoEval,
        }
        .wrap_some();
    }
    if on_path("node") {
        if let Some(tsx) = tsx_cli(config) {
            return Executor {
                program: "node".to_owned(),
                prefix: tsx.display().to_string().wrap_vec(),
                eval: EvalKind::DashE,
            }
            .wrap_some();
        }
    }
    if on_path("pnpm") {
        return Executor {
            program: "pnpm".to_owned(),
            prefix: vec!["exec".to_owned(), "tsx".to_owned()],
            eval: EvalKind::DashE,
        }
        .wrap_some();
    }
    if on_path("npx") {
        return Executor {
            program: "npx".to_owned(),
            prefix: "tsx".to_owned().wrap_vec(),
            eval: EvalKind::DashE,
        }
        .wrap_some();
    }
    if on_path("yarn") {
        return Executor {
            program: "yarn".to_owned(),
            prefix: vec!["exec".to_owned(), "tsx".to_owned()],
            eval: EvalKind::DashE,
        }
        .wrap_some();
    }
    if on_path("node") {
        return Executor {
            program: "node".to_owned(),
            prefix: Vec::new(),
            eval: EvalKind::DashE,
        }
        .wrap_some();
    }
    None
}

const EXPORT_TO_JSON: &str = r"
const path = process.argv[process.argv.length - 1];
import('node:url').then(({ pathToFileURL }) => import(pathToFileURL(path).href)).then((m) => {
  const config = m.default ?? m;
  process.stdout.write(JSON.stringify(config));
}).catch((err) => {
  console.error(err);
  process.exit(1);
});
";

const DENO_EXPORT_TO_JSON: &str = r"
const path = Deno.args[0];
const href = new URL(path, 'file:///').href;
const m = await import(href);
const config = m.default ?? m;
Deno.stdout.writeSync(new TextEncoder().encode(JSON.stringify(config)));
";

/// The config file as JSON text.
pub fn config_json(path: &Path) -> Result<String, LoadError> {
    match syntax(path) {
        Some(ConfigSyntax::Json) => {
            std::fs::read_to_string(path).map_err(|source| {
                LoadError::Unreadable(ConfigNotReadable {
                    path: path.to_owned(),
                    source,
                })
            })
        }
        Some(ConfigSyntax::JavaScript | ConfigSyntax::TypeScript) => {
            let executor = match first_executor(path) {
                Some(executor) => executor,
                None => {
                    return LoadError::NoJsRuntime(NoJsRuntime {
                        path: path.to_owned(),
                    })
                    .wrap_err();
                }
            };
            run_js(executor.reference(), path)
        }
        None => LoadError::UnknownSyntax(UnknownSyntax {
            path: path.to_owned(),
        })
        .wrap_err(),
    }
}

fn run_js(executor: &Executor, path: &Path) -> Result<String, LoadError> {
    let mut command = Command::new(executor.program.reference());
    command.args(executor.prefix.reference());
    match executor.eval {
        EvalKind::DashE => {
            command
                .arg("-e")
                .arg(EXPORT_TO_JSON)
                .arg("--")
                .arg(path);
        }
        EvalKind::DenoEval => {
            command
                .arg("eval")
                .arg("--allow-read")
                .arg(DENO_EXPORT_TO_JSON)
                .arg("--")
                .arg(path);
        }
    }
    let output = command.output().map_err(|source| {
        LoadError::JsIo(JsIo {
            path: path.to_owned(),
            program: executor.display(),
            source,
        })
    })?;
    if !output.status.success() {
        return LoadError::JsFailed(JsFailed {
            path: path.to_owned(),
            program: executor.display(),
            stderr: String::from_utf8_lossy(output.stderr.reference()).into_owned(),
        })
        .wrap_err();
    }
    String::from_utf8_lossy(output.stdout.reference())
        .into_owned()
        .wrap_ok()
}
```

`on_path` is a PATH probe. Inner structs derive `Debug`.

This change does not parse the JSON and does not call `config_json` from `instance` / `run_daemon`.

### Tests

- A `.json` file whose contents are `{}\n` comes back as that text.
- A `.js` file `export default {};` comes back as `{}` (skipped if no executor).
- A `.ts` file `export default {};` comes back as `{}` (skipped if no executor that will execute TypeScript).
- A `.js` file that throws fails with `JsFailed`.
- A `.txt` path fails with `UnknownSyntax`.
- `tsx_cli` finds `node_modules/tsx/dist/cli.mjs` from a subdirectory of a project that has it.

## Change 3: deserialize the JSON

`config_json` produces text. This change parses it into an empty struct. Unknown fields are ignored, so today's `project_root` / `schema` configs load. The struct grows later.

`crates/isograph_cli/Cargo.toml` gains `serde` with `derive`, and `serde_json`.

```rust
// from crates/isograph_cli/src/discover.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IsographConfig {}

pub struct Unparseable {
    pub path: PathBuf,
    pub source: serde_json::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    Unreadable(ConfigNotReadable),
    #[error("{} is not .json, .js, or .ts", .0.path.display())]
    UnknownSyntax(UnknownSyntax),
    #[error("no bun, deno, tsx, npx, pnpm, yarn, or node; needed to load {}", .0.path.display())]
    NoJsRuntime(NoJsRuntime),
    #[error("{} failed on {}: {}", .0.program, .0.path.display(), .0.stderr)]
    JsFailed(JsFailed),
    #[error("could not run {} on {}: {}", .0.program, .0.path.display(), .0.source)]
    JsIo(JsIo),
    #[error("could not parse {}: {}", .0.path.display(), .0.source)]
    Unparseable(Unparseable),
}

pub fn load_config(path: &Path) -> Result<IsographConfig, LoadError> {
    let json = config_json(path)?;
    serde_json::from_str(json.reference()).map_err(|source| {
        LoadError::Unparseable(Unparseable {
            path: path.to_owned(),
            source,
        })
    })
}

pub fn config_and_instance(
    flag: Option<&Path>,
) -> Result<(PathBuf, Instance, IsographConfig), DiscoverError> {
    let config_path = config_path(flag)?;
    let config = load_config(config_path.reference()).map_err(DiscoverError::Load)?;
    let instance = Instance::named(
        "isograph",
        slug(config_path.reference()),
        config_path.display().to_string(),
    )?;
    (config_path, instance, config).wrap_ok()
}
```

`DiscoverError` gains `Load(LoadError)` with `#[error(transparent)]`. `instance` still discards the config. `run_daemon` binds it:

```rust
    fn instance(id: &ConfigFlag) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        let (_, instance, _) = discover::config_and_instance(id.config.as_deref())?;
        instance.wrap_ok()
    }

    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((path, _, _config)) => {
                tracing::info!(config = %path.display(), "isograph daemon up");
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
```

A config that is not JSON (or whose JS/TS export is not JSON-serializable) fails every verb, including `start`, before the daemon is spawned.

### Tests

- `load_config` on `{}\n` succeeds.
- `load_config` on `{"project_root":"./src"}` succeeds.
- `load_config` on `{` fails with `Unparseable`.
- `start` with a fixture whose config is `{` fails.
- `start` with a fixture whose config is `{}` succeeds.
