# Config discovery

Requires isograph-cli.md. The daemon is keyed to one config: `--config` when given, otherwise the nearest `isograph.config.json` at or above the current directory. Two paths to one file are one daemon. The daemon still parks. It logs the config it is.

This is how the babel plugin finds a config (`searchPlaces: ['isograph.config.json']`, walk up). The CLI does the same walk, or takes a path.

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
error: no isograph.config.json at or above /tmp; create one, or name one with --config
```

```
$ isograph --config ./isograph.config.json
$ isograph --config /other/project/isograph.config.json status
```

## Change 1: find the config, key the instance to it

New module `crates/isograph_cli/src/discover.rs`. `App::Id` becomes the config flag. `App::DaemonArgs` can be `NoArgs` now that `Id` is not `NoArgs` (the clap group-name collision goes away).

`crates/isograph_cli/Cargo.toml` gains `thiserror = "2"`.

```rust
// from crates/isograph_cli/src/discover.rs
use std::path::{Path, PathBuf};

use freddie_cli::Instance;
use prelude::Postfix;

pub const CONFIG_FILE_NAME: &str = "isograph.config.json";

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
    #[error("no {CONFIG_FILE_NAME} at or above {}; create one, or name one with --config", .0.start.display())]
    NotFound(ConfigNotFound),
    #[error("could not read the current directory: {}", .0.source)]
    NoCurrentDir(NoCurrentDir),
    #[error(transparent)]
    NoUserDir(#[from] freddie_cli::NoUserDir),
}

/// The canonical path of the config `flag` names, or of the nearest `isograph.config.json`
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
    start
        .ancestors()
        .map(|dir| dir.join(CONFIG_FILE_NAME))
        .find(|candidate| candidate.is_file())
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

/// A filename derived from the canonical path, stable across invocations.
fn slug(config: &Path) -> String {
    format!("isograph-{:016x}", fnv1a(config.as_os_str().as_encoded_bytes()))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
```

`fnv1a` is written out because `std`'s hasher is not stable across releases and the slug is a filename the next invocation has to find. `thiserror` on `DiscoverError` for the `Display` / `Error` impls. `NoUserDir` converts with `From`.

`config_and_instance` computes the pair together so no caller can key an instance to the wrong config.

```rust
// from crates/isograph_cli/src/main.rs
mod discover;

#[derive(clap::Args, Debug)]
pub struct ConfigFlag {
    /// Path to the isograph config. When absent, the nearest isograph.config.json at or above
    /// the current directory.
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

The contents of the config file are not read. That is the next milestone (scan the project root).

## Tests

Unit tests on `nearest_config` and `slug`, in `discover.rs`, using a temp directory.

- A config in `project/` is found from `project/src/components`.
- A directory with no config above it is `None`.
- Two canonical paths get two slugs; one path gets the same slug twice.

Binary tests, same private-`HOME` harness as isograph-cli.md:

- `start` in a fixture project, then `status` names that config and reports running.
- `start` in a subdirectory of the fixture finds the same daemon.
- `start` with no config above cwd fails.
- `--config` pointing at a missing file fails.
- Two fixture projects are two daemons: `status` in each reports a different pid; `stop` in one leaves the other running.
- `logs` contains the canonical config path.
