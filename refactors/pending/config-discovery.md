# Config discovery

Requires isograph-cli.md and cli-ci-build.md. The daemon is keyed to one config: `--config` when given, otherwise the nearest `isograph.config.json`, `isograph.config.js`, or `isograph.config.ts` at or above the current directory. At one directory, that order: json, then js, then ts. Two paths to one file are one daemon. The lock and the log file are `{slug}` and `{slug}.log`. The daemon still parks. It logs the config it is.

Walk-up is the babel plugin's walk, with two extra names. The babel plugin still only opens `isograph.config.json`.

## What the user does

```
$ cd app/src/components && isograph
/Users/x/app/isograph.config.json started (pid 12345)
$ isograph status
/Users/x/app/isograph.config.json is running (pid 12345)
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"logging","path":"/Users/x/Library/Logs/isograph/isograph-0123456789abcdef.log"}}
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json"}}
$ isograph stop
```

`isograph logs` follows that daemon's file. Two configs are two files in the same directory (`~/Library/Logs/isograph/` on macOS, `$XDG_STATE_HOME/isograph/` on Linux, `%LOCALAPPDATA%/isograph/logs/` on Windows).

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

#[derive(Debug)]
pub struct ConfigNotReadable {
    pub path: PathBuf,
    pub source: std::io::Error,
}

#[derive(Debug)]
pub struct ConfigNotFound {
    pub start: PathBuf,
}

#[derive(Debug)]
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

/// A filename: `Instance::named` keys the lock to this and the log to `{slug}.log`.
fn slug(config: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);
    format!("isograph-{:016x}", hasher.finish())
}
```

Walk-up is `Path::ancestors`. At each directory, json then js then ts. No crate searches that way and returns the path without also parsing.

`thiserror` on `DiscoverError` for the `Display` / `Error` impls. `NoUserDir` converts with `From`.

`config_and_instance` computes the pair together so no caller can key an instance to the wrong config. `Instance::named` puts the log at `{log_dir}/{slug}.log`, not `isograph.log`.

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

`start` prints `{config} started (pid …)` on stdout. Client tracing writes that same record to the log file. `run_daemon` writes `config` on `isograph daemon up` to the log file.

The contents of the config file are not read.

## Tests

Unit tests in `discover.rs`. Binary tests replace `Daemon` with `World`: one private HOME, any number of project directories, optional `--config`. Config files are `{}\n` so Change 3 does not break these tests. `World::isograph` uses `isograph_bin()` from cli-ci-build.md. The test job on each platform downloads the release artifact and runs `cargo test --tests` with `ISOGRAPH_BIN` set to it.

```rust
// from crates/isograph_cli/src/discover.rs
#[cfg(test)]
mod tests {
    use std::path::Path;

    use prelude::Postfix;

    use super::{config_path, nearest_config, slug, DiscoverError};

    fn temp() -> tempfile::TempDir {
        tempfile::tempdir().expect("a test can create a temp directory")
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a test can create parent directories");
        }
        std::fs::write(path, contents).expect("a test can write a file");
    }

    #[test]
    fn nearest_config_finds_json_from_a_nested_directory() {
        let dir = temp();
        let project = dir.path().join("project");
        let nested = project.join("src/components");
        let config = project.join("isograph.config.json");
        write_file(config.reference(), "{}\n");
        std::fs::create_dir_all(nested.reference())
            .expect("a test can create a nested directory");
        assert_eq!(nearest_config(nested.reference()), config.wrap_some());
    }

    #[test]
    fn nearest_config_finds_js_when_there_is_no_json() {
        let dir = temp();
        let project = dir.path().join("project");
        let nested = project.join("src");
        let config = project.join("isograph.config.js");
        write_file(config.reference(), "export default {};\n");
        std::fs::create_dir_all(nested.reference())
            .expect("a test can create a nested directory");
        assert_eq!(nearest_config(nested.reference()), config.wrap_some());
    }

    #[test]
    fn nearest_config_finds_ts_when_there_is_no_json_or_js() {
        let dir = temp();
        let project = dir.path().join("project");
        let nested = project.join("src");
        let config = project.join("isograph.config.ts");
        write_file(config.reference(), "export default {};\n");
        std::fs::create_dir_all(nested.reference())
            .expect("a test can create a nested directory");
        assert_eq!(nearest_config(nested.reference()), config.wrap_some());
    }

    #[test]
    fn nearest_config_prefers_json_to_js_in_the_same_directory() {
        let dir = temp();
        let project = dir.path();
        let json = project.join("isograph.config.json");
        let js = project.join("isograph.config.js");
        write_file(json.reference(), "{}\n");
        write_file(js.reference(), "export default {};\n");
        assert_eq!(nearest_config(project), json.wrap_some());
    }

    #[test]
    fn nearest_config_prefers_json_to_ts_in_the_same_directory() {
        let dir = temp();
        let project = dir.path();
        let json = project.join("isograph.config.json");
        let ts = project.join("isograph.config.ts");
        write_file(json.reference(), "{}\n");
        write_file(ts.reference(), "export default {};\n");
        assert_eq!(nearest_config(project), json.wrap_some());
    }

    #[test]
    fn nearest_config_prefers_js_to_ts_in_the_same_directory() {
        let dir = temp();
        let project = dir.path();
        let js = project.join("isograph.config.js");
        let ts = project.join("isograph.config.ts");
        write_file(js.reference(), "export default {};\n");
        write_file(ts.reference(), "export default {};\n");
        assert_eq!(nearest_config(project), js.wrap_some());
    }

    #[test]
    fn nearest_config_returns_none_when_the_tree_has_no_config() {
        let dir = temp();
        let start = dir.path().join("empty");
        std::fs::create_dir_all(start.reference())
            .expect("a test can create an empty directory");
        assert_eq!(nearest_config(start.reference()), None);
    }

    #[test]
    fn nearest_config_walks_up_to_a_parent() {
        let dir = temp();
        let project = dir.path().join("project");
        let child = project.join("src");
        let config = project.join("isograph.config.json");
        write_file(config.reference(), "{}\n");
        std::fs::create_dir_all(child.reference())
            .expect("a test can create a child directory");
        assert_eq!(nearest_config(child.reference()), config.wrap_some());
    }

    #[test]
    fn nearest_config_prefers_a_child_js_to_a_parent_json() {
        let dir = temp();
        let parent = dir.path().join("project");
        let child = parent.join("pkg");
        write_file(parent.join("isograph.config.json").reference(), "{}\n");
        let js = child.join("isograph.config.js");
        write_file(js.reference(), "export default {};\n");
        assert_eq!(nearest_config(child.reference()), js.wrap_some());
    }

    #[test]
    fn nearest_config_skips_a_directory_named_like_the_json_file() {
        let dir = temp();
        let project = dir.path();
        std::fs::create_dir_all(project.join("isograph.config.json").reference())
            .expect("a test can create a directory named like the json file");
        let js = project.join("isograph.config.js");
        write_file(js.reference(), "export default {};\n");
        assert_eq!(nearest_config(project), js.wrap_some());
    }

    #[test]
    fn nearest_config_ignores_a_sibling_directory() {
        let dir = temp();
        let project = dir.path().join("project");
        let src = project.join("src");
        let other = project.join("other");
        write_file(project.join("isograph.config.json").reference(), "{}\n");
        write_file(other.join("isograph.config.js").reference(), "export default {};\n");
        std::fs::create_dir_all(src.reference())
            .expect("a test can create src");
        assert_eq!(
            nearest_config(src.reference()),
            project.join("isograph.config.json").wrap_some()
        );
    }

    #[test]
    fn slug_is_stable_for_one_path() {
        let path = Path::new("/a/b/isograph.config.json");
        assert_eq!(slug(path), slug(path));
    }

    #[test]
    fn slug_differs_for_two_paths() {
        assert_ne!(
            slug(Path::new("/a/isograph.config.json")),
            slug(Path::new("/b/isograph.config.json"))
        );
    }

    #[test]
    fn slug_is_a_filename() {
        let slug = slug(Path::new("/a/b/isograph.config.json"));
        assert!(!slug.contains('/'));
        let hex = slug
            .strip_prefix("isograph-")
            .expect("the slug is isograph-<hash>");
        assert_eq!(hex.len(), 16);
        assert!(hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    }

    #[test]
    fn config_path_canonicalizes_an_existing_flag() {
        let dir = temp();
        let config = dir.path().join("isograph.config.json");
        write_file(config.reference(), "{}\n");
        let got = config_path(config.as_path().wrap_some())
            .expect("the fixture file exists");
        let canonical = config.canonicalize().expect("the fixture file exists");
        assert_eq!(got, canonical);
        assert!(got.is_absolute());
    }

    #[test]
    fn config_path_of_a_dotted_path_matches_the_real_file() {
        let dir = temp();
        let config = dir.path().join("isograph.config.json");
        write_file(config.reference(), "{}\n");
        let dotted = dir.path().join(".").join("isograph.config.json");
        let a = config_path(config.as_path().wrap_some())
            .expect("the fixture file exists");
        let b = config_path(dotted.as_path().wrap_some())
            .expect("the dotted path exists");
        assert_eq!(a, b);
    }

    #[test]
    fn config_path_fails_when_the_flag_is_missing() {
        let dir = temp();
        let missing = dir.path().join("nope.json");
        let err = config_path(missing.as_path().wrap_some())
            .expect_err("the file is missing");
        let DiscoverError::ConfigNotReadable(inner) = err else {
            panic!("expected ConfigNotReadable, got {err}");
        };
        assert_eq!(inner.path, missing);
    }
}
```

```rust
// from crates/isograph_cli/tests/cli.rs
//! Drive the built `isograph` binary. Every daemon's lock and log live under a private HOME.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use prelude::Postfix;

const DEADLINE: Duration = Duration::from_secs(10);

struct World {
    dir: tempfile::TempDir,
}

struct Running<'a> {
    world: &'a World,
    cwd: PathBuf,
    config: Option<PathBuf>,
    start_stdout: String,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        Self { dir }
    }

    fn project(&self, name: &str) -> PathBuf {
        let project = self.dir.path().join(name);
        std::fs::create_dir_all(project.reference())
            .expect("a test can create a project directory");
        project
    }

    fn write_config(project: &Path, file_name: &str, contents: &str) -> PathBuf {
        let path = project.join(file_name);
        std::fs::write(path.reference(), contents).expect("a test can write a config file");
        path
    }

    fn project_with_json(&self, name: &str) -> (PathBuf, PathBuf) {
        let project = self.project(name);
        let config = Self::write_config(project.reference(), "isograph.config.json", "{}\n");
        (project, config)
    }

    fn isograph(
        &self,
        cwd: &Path,
        config: Option<&Path>,
        args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
    ) -> Output {
        let home = self.dir.path().join("home");
        std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
        let mut command = Command::new(isograph_bin());
        command.args(args);
        if let Some(config) = config {
            command.arg("--config").arg(config);
        }
        command
            .current_dir(cwd)
            .env("HOME", home.reference())
            .env("XDG_STATE_HOME", home.join("state"))
            .env("LOCALAPPDATA", home.join("appdata"))
            .output()
            .expect("the isograph binary runs")
    }

    fn start(&self, cwd: &Path, config: Option<&Path>) -> Running<'_> {
        let output = self.isograph(cwd, config, ["start"]);
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(output.stderr.reference())
        );
        Running {
            world: self,
            cwd: cwd.to_owned(),
            config: config.map(Path::to_owned),
            start_stdout: stdout(output.reference()),
        }
    }

    fn log_paths(&self) -> Vec<PathBuf> {
        let home = self.dir.path().join("home");
        let mut paths = Vec::new();
        let mut stack = home.wrap_vec();
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(dir.reference()) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "log") {
                    paths.push(path);
                }
            }
        }
        paths
    }
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        let _ = self.world.isograph(
            self.cwd.reference(),
            self.config.as_deref(),
            ["stop", "--force"],
        );
    }
}

fn poll<T>(mut f: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(value) = f() {
            return value;
        }
        assert!(start.elapsed() < DEADLINE, "deadline passed");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(output.stdout.reference()).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(output.stderr.reference()).into_owned()
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().expect("the fixture exists")
}

fn hashed_log_name(path: &Path) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("log file names are utf-8");
    let hex = name
        .strip_prefix("isograph-")
        .and_then(|s| s.strip_suffix(".log"))
        .expect("the log file is isograph-<hash>.log");
    assert_eq!(hex.len(), 16, "{name}");
    assert!(
        hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "{name}"
    );
}

fn pid_from(output: &Output) -> u32 {
    let text = stdout(output.reference());
    let rest = text
        .split_once("pid ")
        .expect("status names a pid")
        .1;
    rest.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .expect("pid is digits")
}

#[test]
fn start_then_status_reports_running_and_names_the_config() {
    let world = World::new();
    let (project, config) = world.project_with_json("app");
    let running = world.start(project.reference(), None);
    let path = canonical(config.reference()).display().to_string();
    assert!(
        running.start_stdout.contains("started"),
        "{}",
        running.start_stdout
    );
    assert!(
        running.start_stdout.contains(path.reference()),
        "{}",
        running.start_stdout
    );
    let status = world.isograph(project.reference(), None, ["status"]);
    assert!(status.status.success());
    let text = stdout(status.reference());
    assert!(text.contains("is running"), "{text}");
    assert!(text.contains(path.reference()), "{text}");
}

#[test]
fn start_in_a_subdirectory_finds_the_same_daemon() {
    let world = World::new();
    let (project, config) = world.project_with_json("app");
    let nested = project.join("src/components");
    std::fs::create_dir_all(nested.reference())
        .expect("a test can create a nested directory");
    let _running = world.start(nested.reference(), None);
    let from_root = world.isograph(project.reference(), None, ["status"]);
    let from_nested = world.isograph(nested.reference(), None, ["status"]);
    assert!(from_root.status.success());
    assert!(from_nested.status.success());
    assert_eq!(pid_from(from_root.reference()), pid_from(from_nested.reference()));
    let path = canonical(config.reference()).display().to_string();
    assert!(stdout(from_root.reference()).contains(path.reference()));
    assert!(stdout(from_nested.reference()).contains(path.reference()));
}

#[test]
fn start_with_no_config_above_cwd_fails() {
    let world = World::new();
    let cwd = world.project("empty");
    let output = world.isograph(cwd.reference(), None, ["start"]);
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(err.contains("isograph.config.json"), "{err}");
    assert!(err.contains("isograph.config.js"), "{err}");
    assert!(err.contains("isograph.config.ts"), "{err}");
    assert!(err.contains(&cwd.display().to_string()), "{err}");
    assert!(err.contains("--config"), "{err}");
}

#[test]
fn start_with_config_flag_pointing_at_a_missing_file_fails() {
    let world = World::new();
    let cwd = world.project("app");
    let missing = cwd.join("nope.json");
    let output = world.isograph(cwd.reference(), missing.as_path().wrap_some(), ["start"]);
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(err.contains("could not resolve the config at"), "{err}");
    assert!(err.contains(&missing.display().to_string()), "{err}");
}

#[test]
fn start_with_config_flag_uses_that_file_not_walk_up() {
    let world = World::new();
    let project = world.project("app");
    World::write_config(project.reference(), "isograph.config.json", "{}\n");
    let js = World::write_config(
        project.reference(),
        "isograph.config.js",
        "export default {};\n",
    );
    std::fs::write(
        project.join("package.json").reference(),
        "{\"type\":\"module\"}\n",
    )
    .expect("a test can write package.json");
    let _running = world.start(project.reference(), js.as_path().wrap_some());
    let via_flag = world.isograph(project.reference(), js.as_path().wrap_some(), ["status"]);
    let via_walk = world.isograph(project.reference(), None, ["status"]);
    assert!(via_flag.status.success());
    assert!(!via_walk.status.success());
    assert!(
        stdout(via_flag.reference()).contains(&canonical(js.reference()).display().to_string()),
        "{}",
        stdout(via_flag.reference())
    );
}

#[test]
fn two_projects_are_two_daemons() {
    let world = World::new();
    let (a, a_config) = world.project_with_json("a");
    let (b, b_config) = world.project_with_json("b");
    let _da = world.start(a.reference(), None);
    let _db = world.start(b.reference(), None);
    let status_a = world.isograph(a.reference(), None, ["status"]);
    let status_b = world.isograph(b.reference(), None, ["status"]);
    assert!(status_a.status.success());
    assert!(status_b.status.success());
    assert_ne!(pid_from(status_a.reference()), pid_from(status_b.reference()));
    assert!(
        stdout(status_a.reference())
            .contains(&canonical(a_config.reference()).display().to_string())
    );
    assert!(
        stdout(status_b.reference())
            .contains(&canonical(b_config.reference()).display().to_string())
    );
    let a_path = canonical(a_config.reference()).display().to_string();
    let b_path = canonical(b_config.reference()).display().to_string();
    let logs = poll(|| {
        let paths = world.log_paths();
        (paths.len() == 2).then_some(paths)
    });
    for path in logs.iter() {
        hashed_log_name(path.reference());
    }
    let texts: Vec<String> = logs
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("the log is readable"))
        .collect();
    assert_eq!(
        texts.iter().filter(|text| text.contains(a_path.reference())).count(),
        1
    );
    assert_eq!(
        texts.iter().filter(|text| text.contains(b_path.reference())).count(),
        1
    );
    assert!(!texts
        .iter()
        .any(|text| text.contains(a_path.reference()) && text.contains(b_path.reference())));
}

#[test]
fn stop_in_one_project_leaves_the_other_running() {
    let world = World::new();
    let (a, _) = world.project_with_json("a");
    let (b, _) = world.project_with_json("b");
    let _da = world.start(a.reference(), None);
    let _db = world.start(b.reference(), None);
    let stopped = world.isograph(a.reference(), None, ["stop"]);
    assert!(stopped.status.success());
    poll(|| (!world.isograph(a.reference(), None, ["status"]).status.success()).then_some(()));
    assert!(world.isograph(b.reference(), None, ["status"]).status.success());
}

#[test]
fn the_log_is_named_for_the_config_hash_and_contains_the_canonical_path() {
    let world = World::new();
    let (project, config) = world.project_with_json("app");
    let _running = world.start(project.reference(), None);
    let path = canonical(config.reference()).display().to_string();
    let logs = poll(|| {
        let paths = world.log_paths();
        let has = paths.iter().any(|log| {
            std::fs::read_to_string(log)
                .ok()
                .is_some_and(|text| {
                    text.contains("isograph daemon up") && text.contains(path.reference())
                })
        });
        has.then_some(paths)
    });
    assert_eq!(logs.len(), 1);
    hashed_log_name(logs[0].reference());
}

#[test]
fn relative_and_absolute_config_flags_are_one_daemon() {
    let world = World::new();
    let (project, config) = world.project_with_json("app");
    let absolute = canonical(config.reference());
    let _running = world.start(project.reference(), Path::new("isograph.config.json").wrap_some());
    let status = world.isograph(project.reference(), absolute.as_path().wrap_some(), ["status"]);
    assert!(status.status.success());
    assert_eq!(
        pid_from(world.isograph(project.reference(), None, ["status"]).reference()),
        pid_from(status.reference())
    );
}

#[test]
fn stop_then_status_reports_not_running() {
    let world = World::new();
    let (project, _) = world.project_with_json("app");
    let _running = world.start(project.reference(), None);
    assert!(world.isograph(project.reference(), None, ["status"]).status.success());
    let stopped = world.isograph(project.reference(), None, ["stop"]);
    assert!(stopped.status.success());
    poll(|| {
        (!world.isograph(project.reference(), None, ["status"]).status.success()).then_some(())
    });
}

#[test]
fn a_second_start_adopts_the_running_daemon() {
    let world = World::new();
    let (project, _) = world.project_with_json("app");
    let _running = world.start(project.reference(), None);
    let again = world.isograph(project.reference(), None, ["start"]);
    assert!(again.status.success());
    assert!(
        stdout(again.reference()).contains("already running"),
        "{}",
        stdout(again.reference())
    );
    assert!(world.isograph(project.reference(), None, ["status"]).status.success());
}

#[test]
fn start_in_a_js_only_project_names_the_js_config() {
    let world = World::new();
    let project = world.project("app");
    let config = World::write_config(
        project.reference(),
        "isograph.config.js",
        "export default {};\n",
    );
    std::fs::write(
        project.join("package.json").reference(),
        "{\"type\":\"module\"}\n",
    )
    .expect("a test can write package.json");
    let _running = world.start(project.reference(), None);
    let status = world.isograph(project.reference(), None, ["status"]);
    assert!(status.status.success());
    assert!(
        stdout(status.reference())
            .contains(&canonical(config.reference()).display().to_string()),
        "{}",
        stdout(status.reference())
    );
}

#[cfg(unix)]
#[test]
fn symlink_and_real_path_are_one_daemon() {
    let world = World::new();
    let project = world.project("app");
    let real = World::write_config(project.reference(), "isograph.config.json", "{}\n");
    let link = project.join("link.config.json");
    std::os::unix::fs::symlink(real.reference(), link.reference())
        .expect("a test can create a symlink");
    let _running = world.start(project.reference(), real.as_path().wrap_some());
    let status = world.isograph(project.reference(), link.as_path().wrap_some(), ["status"]);
    assert!(status.status.success());
    assert_eq!(
        pid_from(
            world
                .isograph(project.reference(), real.as_path().wrap_some(), ["status"])
                .reference()
        ),
        pid_from(status.reference())
    );
}
```

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

#[derive(Debug)]
pub struct UnknownSyntax {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct NoJsRuntime {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct JsFailed {
    pub path: PathBuf,
    pub program: String,
    pub stderr: String,
}

#[derive(Debug)]
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

Added to the `discover.rs` tests module. `config_json` on `.js` / `.ts` is `NoJsRuntime` when this machine has no executor; `JsFailed` from plain `node` on TypeScript is the same skip.

```rust
// from crates/isograph_cli/src/discover.rs
    use super::{
        config_json, first_executor, tsx_cli, Executor, LoadError,
    };

    #[test]
    fn config_json_returns_json_file_bytes() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "{}\n");
        assert_eq!(
            config_json(path.reference()).expect("the fixture is readable json"),
            "{}\n"
        );
    }

    #[test]
    fn config_json_stringifies_an_esm_default_export() {
        let dir = temp();
        let path = dir.path().join("isograph.config.js");
        write_file(dir.path().join("package.json").reference(), "{\"type\":\"module\"}\n");
        write_file(path.reference(), "export default {};\n");
        match config_json(path.reference()) {
            Ok(json) => assert_eq!(json.trim(), "{}"),
            Err(LoadError::NoJsRuntime(_)) => {}
            Err(e) => panic!("expected json or no runtime, got {e}"),
        }
    }

    #[test]
    fn config_json_stringifies_a_cjs_export() {
        let dir = temp();
        let path = dir.path().join("isograph.config.js");
        write_file(path.reference(), "module.exports = {};\n");
        match config_json(path.reference()) {
            Ok(json) => assert_eq!(json.trim(), "{}"),
            Err(LoadError::NoJsRuntime(_)) => {}
            Err(e) => panic!("expected json or no runtime, got {e}"),
        }
    }

    #[test]
    fn config_json_stringifies_a_ts_default_export() {
        let dir = temp();
        let path = dir.path().join("isograph.config.ts");
        write_file(dir.path().join("package.json").reference(), "{\"type\":\"module\"}\n");
        write_file(path.reference(), "export default {};\n");
        match (config_json(path.reference()), first_executor(path.reference())) {
            (Ok(json), _) => assert_eq!(json.trim(), "{}"),
            (Err(LoadError::NoJsRuntime(_)), _) => {}
            (Err(LoadError::JsFailed(_)), Some(Executor { program, prefix, .. }))
                if program == "node" && prefix.is_empty() => {}
            (Err(e), _) => panic!("expected json, no runtime, or plain node, got {e}"),
        }
    }

    #[test]
    fn config_json_js_that_throws_is_js_failed() {
        let dir = temp();
        let path = dir.path().join("isograph.config.js");
        write_file(dir.path().join("package.json").reference(), "{\"type\":\"module\"}\n");
        write_file(path.reference(), "throw new Error('nope');\n");
        match config_json(path.reference()) {
            Err(LoadError::JsFailed(failed)) => {
                assert_eq!(failed.path, path);
                assert!(failed.stderr.contains("nope"), "{}", failed.stderr);
            }
            Err(LoadError::NoJsRuntime(_)) => {}
            other => panic!("expected JsFailed or no runtime, got {other:?}"),
        }
    }

    #[test]
    fn config_json_unknown_extension_is_unknown_syntax() {
        let dir = temp();
        let path = dir.path().join("isograph.config.txt");
        write_file(path.reference(), "{}\n");
        let err = config_json(path.reference()).expect_err("txt is not a config syntax");
        let LoadError::UnknownSyntax(inner) = err else {
            panic!("expected UnknownSyntax, got {err}");
        };
        assert_eq!(inner.path, path);
    }

    #[test]
    fn config_json_no_extension_is_unknown_syntax() {
        let dir = temp();
        let path = dir.path().join("isograph.config");
        write_file(path.reference(), "{}\n");
        let err = config_json(path.reference()).expect_err("no extension is not a config syntax");
        let LoadError::UnknownSyntax(inner) = err else {
            panic!("expected UnknownSyntax, got {err}");
        };
        assert_eq!(inner.path, path);
    }

    #[test]
    fn tsx_cli_finds_the_mjs_from_a_subdirectory() {
        let dir = temp();
        let mjs = dir.path().join("node_modules/tsx/dist/cli.mjs");
        write_file(mjs.reference(), "");
        let nested = dir.path().join("src/components");
        std::fs::create_dir_all(nested.reference())
            .expect("a test can create a nested directory");
        assert_eq!(tsx_cli(nested.reference()), mjs.wrap_some());
    }

    #[test]
    fn tsx_cli_returns_none_when_the_tree_has_no_tsx() {
        let dir = temp();
        let nested = dir.path().join("src");
        std::fs::create_dir_all(nested.reference())
            .expect("a test can create a nested directory");
        assert_eq!(tsx_cli(nested.reference()), None);
    }
```

## Change 3: deserialize the JSON

`config_json` produces text. This change parses it into an empty struct. Unknown fields are ignored, so today's `project_root` / `schema` configs load. The struct grows later.

`crates/isograph_cli/Cargo.toml` gains `serde` with `derive`, and `serde_json`.

```rust
// from crates/isograph_cli/src/discover.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IsographConfig {}

#[derive(Debug)]
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

Added to the `discover.rs` tests module, plus two binary tests on `World`.

```rust
// from crates/isograph_cli/src/discover.rs
    use super::load_config;

    #[test]
    fn load_config_empty_object() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "{}\n");
        load_config(path.reference()).expect("empty object is a config");
    }

    #[test]
    fn load_config_ignores_unknown_fields() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "{\"project_root\":\"./src\",\"schema\":\"./schema.graphql\"}\n");
        load_config(path.reference()).expect("unknown fields are ignored");
    }

    #[test]
    fn load_config_unparseable_object() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "{");
        let err = load_config(path.reference()).expect_err("truncated json is unparseable");
        let LoadError::Unparseable(inner) = err else {
            panic!("expected Unparseable, got {err}");
        };
        assert_eq!(inner.path, path);
    }

    #[test]
    fn load_config_empty_file() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "");
        let err = load_config(path.reference()).expect_err("empty file is unparseable");
        let LoadError::Unparseable(inner) = err else {
            panic!("expected Unparseable, got {err}");
        };
        assert_eq!(inner.path, path);
    }

    #[test]
    fn load_config_null() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "null\n");
        let err = load_config(path.reference()).expect_err("null is not a config object");
        let LoadError::Unparseable(inner) = err else {
            panic!("expected Unparseable, got {err}");
        };
        assert_eq!(inner.path, path);
    }

    #[test]
    fn load_config_array() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "[]\n");
        let err = load_config(path.reference()).expect_err("array is not a config object");
        let LoadError::Unparseable(inner) = err else {
            panic!("expected Unparseable, got {err}");
        };
        assert_eq!(inner.path, path);
    }
```

```rust
// from crates/isograph_cli/tests/cli.rs
#[test]
fn start_with_unparseable_config_fails() {
    let world = World::new();
    let project = world.project("app");
    let config = World::write_config(project.reference(), "isograph.config.json", "{");
    let output = world.isograph(project.reference(), None, ["start"]);
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(err.contains("could not parse"), "{err}");
    assert!(
        err.contains(&canonical(config.reference()).display().to_string())
            || err.contains(&config.display().to_string()),
        "{err}"
    );
}

#[test]
fn start_with_empty_object_succeeds() {
    let world = World::new();
    let (project, config) = world.project_with_json("app");
    let _running = world.start(project.reference(), None);
    let status = world.isograph(project.reference(), None, ["status"]);
    assert!(status.status.success());
    assert!(
        stdout(status.reference())
            .contains(&canonical(config.reference()).display().to_string()),
        "{}",
        stdout(status.reference())
    );
}
```
