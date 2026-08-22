use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use freddie_cli::Instance;
use isograph_config::IsographProjectConfig;
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
    #[error(transparent)]
    Load(LoadError),
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
        DiscoverError::ConfigNotReadable(ConfigNotReadable {
            path: named,
            source,
        })
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

#[derive(Debug)]
pub struct Unparseable {
    pub path: PathBuf,
    pub source: serde_json::Error,
}

pub fn load_config(path: &Path) -> Result<IsographProjectConfig, LoadError> {
    let json = config_json(path)?;
    serde_json::from_str(json.reference()).map_err(|source| {
        LoadError::Unparseable(Unparseable {
            path: path.to_owned(),
            source,
        })
    })
}

pub fn port_file(lock: &Path) -> PathBuf {
    lock.with_extension("port")
}

pub fn instance_for_config_path(flag: Option<&Path>) -> Result<(PathBuf, Instance), DiscoverError> {
    let config_path = config_path(flag)?;
    let instance = Instance::named(
        "isograph",
        slug(config_path.reference()),
        config_path.display().to_string(),
    )?;
    (config_path, instance).wrap_ok()
}

pub fn config_and_instance(
    flag: Option<&Path>,
) -> Result<(PathBuf, Instance, IsographProjectConfig), DiscoverError> {
    let (config_path, instance) = instance_for_config_path(flag)?;
    let config = load_config(config_path.reference()).map_err(DiscoverError::Load)?;
    (config_path, instance, config).wrap_ok()
}

/// A filename: `Instance::named` keys the lock to this and the log to `{slug}.log`.
fn slug(config: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);
    format!("isograph-{:016x}", hasher.finish())
}

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
    #[error("could not parse {}: {}", .0.path.display(), .0.source)]
    Unparseable(Unparseable),
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
    if on_path("node")
        && let Some(tsx) = tsx_cli(config)
    {
        return Executor {
            program: "node".to_owned(),
            prefix: tsx.display().to_string().wrap_vec(),
            eval: EvalKind::DashE,
        }
        .wrap_some();
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

const EXPORT_TO_JSON: &str = include_str!("export_to_json.js");
const DENO_EXPORT_TO_JSON: &str = include_str!("deno_export_to_json.js");

/// The config file as JSON text.
pub fn config_json(path: &Path) -> Result<String, LoadError> {
    match syntax(path) {
        Some(ConfigSyntax::Json) => std::fs::read_to_string(path).map_err(|source| {
            LoadError::Unreadable(ConfigNotReadable {
                path: path.to_owned(),
                source,
            })
        }),
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
            command.arg("-e").arg(EXPORT_TO_JSON).arg("--").arg(path);
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use prelude::Postfix;

    use super::{
        DiscoverError, LoadError, config_json, config_path, load_config, nearest_config, slug,
        tsx_cli,
    };

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
        std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
        assert_eq!(nearest_config(nested.reference()), config.wrap_some());
    }

    #[test]
    fn nearest_config_finds_js_when_there_is_no_json() {
        let dir = temp();
        let project = dir.path().join("project");
        let nested = project.join("src");
        let config = project.join("isograph.config.js");
        write_file(config.reference(), "export default {};\n");
        std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
        assert_eq!(nearest_config(nested.reference()), config.wrap_some());
    }

    #[test]
    fn nearest_config_finds_ts_when_there_is_no_json_or_js() {
        let dir = temp();
        let project = dir.path().join("project");
        let nested = project.join("src");
        let config = project.join("isograph.config.ts");
        write_file(config.reference(), "export default {};\n");
        std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
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
        std::fs::create_dir_all(start.reference()).expect("a test can create an empty directory");
        assert_eq!(nearest_config(start.reference()), None);
    }

    #[test]
    fn nearest_config_walks_up_to_a_parent() {
        let dir = temp();
        let project = dir.path().join("project");
        let child = project.join("src");
        let config = project.join("isograph.config.json");
        write_file(config.reference(), "{}\n");
        std::fs::create_dir_all(child.reference()).expect("a test can create a child directory");
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
        write_file(
            other.join("isograph.config.js").reference(),
            "export default {};\n",
        );
        std::fs::create_dir_all(src.reference()).expect("a test can create src");
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
        let got = config_path(config.as_path().wrap_some()).expect("the fixture file exists");
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
        let a = config_path(config.as_path().wrap_some()).expect("the fixture file exists");
        let b = config_path(dotted.as_path().wrap_some()).expect("the dotted path exists");
        assert_eq!(a, b);
    }

    #[test]
    fn config_path_fails_when_the_flag_is_missing() {
        let dir = temp();
        let missing = dir.path().join("nope.json");
        let err = config_path(missing.as_path().wrap_some()).expect_err("the file is missing");
        let DiscoverError::ConfigNotReadable(inner) = err else {
            panic!("expected ConfigNotReadable, got {err}");
        };
        assert_eq!(inner.path, missing);
    }

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
        write_file(
            dir.path().join("package.json").reference(),
            "{\"type\":\"module\"}\n",
        );
        write_file(path.reference(), "export default {};\n");
        match config_json(path.reference()) {
            Ok(json) => assert_eq!(json.trim(), "{}"),
            Err(LoadError::NoJsRuntime(_)) | Err(LoadError::JsFailed(_)) => {}
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
            Err(LoadError::NoJsRuntime(_)) | Err(LoadError::JsFailed(_)) => {}
            Err(e) => panic!("expected json or no runtime, got {e}"),
        }
    }

    #[test]
    fn config_json_stringifies_a_ts_default_export() {
        let dir = temp();
        let path = dir.path().join("isograph.config.ts");
        write_file(
            dir.path().join("package.json").reference(),
            "{\"type\":\"module\"}\n",
        );
        write_file(path.reference(), "export default {};\n");
        match config_json(path.reference()) {
            Ok(json) => assert_eq!(json.trim(), "{}"),
            Err(LoadError::NoJsRuntime(_)) | Err(LoadError::JsFailed(_)) => {}
            Err(e) => panic!("expected json, no runtime, or executor failure, got {e}"),
        }
    }

    #[test]
    fn config_json_js_that_throws_is_js_failed() {
        let dir = temp();
        let path = dir.path().join("isograph.config.js");
        write_file(
            dir.path().join("package.json").reference(),
            "{\"type\":\"module\"}\n",
        );
        write_file(path.reference(), "throw new Error('nope');\n");
        match config_json(path.reference()) {
            Err(LoadError::JsFailed(failed)) if failed.stderr.contains("nope") => {
                assert_eq!(failed.path, path);
            }
            Err(LoadError::JsFailed(_)) | Err(LoadError::NoJsRuntime(_)) => {}
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
        std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
        assert_eq!(tsx_cli(nested.reference()), mjs.wrap_some());
    }

    #[test]
    fn tsx_cli_returns_none_when_the_tree_has_no_tsx() {
        let dir = temp();
        let nested = dir.path().join("src");
        std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
        assert_eq!(tsx_cli(nested.reference()), None);
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
    fn port_file_is_the_lock_with_a_port_extension() {
        assert_eq!(
            super::port_file(Path::new("/tmp/isograph-abcd.lock")),
            Path::new("/tmp/isograph-abcd.port")
        );
    }

    #[test]
    fn instance_for_config_path_does_not_parse_json() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "{");
        let (got, _) =
            super::instance_for_config_path(path.as_path().wrap_some()).expect("the file exists");
        let canonical = path.canonicalize().expect("the fixture file exists");
        assert_eq!(got, canonical);
        super::load_config(path.reference()).expect_err("truncated json is unparseable");
    }
}
