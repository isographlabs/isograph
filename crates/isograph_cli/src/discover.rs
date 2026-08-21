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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use prelude::Postfix;

    use super::{DiscoverError, config_path, nearest_config, slug};

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
}
