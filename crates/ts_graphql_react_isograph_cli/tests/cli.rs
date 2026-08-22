//! Drive the built `isograph` binary. Every daemon's lock and log live under a private HOME.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use prelude::Postfix;

const DEADLINE: Duration = Duration::from_secs(10);

fn isograph_bin() -> PathBuf {
    match std::env::var_os("ISOGRAPH_BIN") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env!("CARGO_BIN_EXE_isograph")),
    }
}

struct Daemon {
    dir: tempfile::TempDir,
}

impl Daemon {
    fn start() -> Self {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let config = dir.path().join("isograph.config.json");
        std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
        let daemon = Self { dir };
        let output = daemon.isograph(["start"].reference());
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(output.stderr.reference())
        );
        let text = stdout(output.reference());
        let path = config.canonicalize().expect("the fixture exists");
        assert!(text.contains("started"), "{text}");
        assert!(text.contains(&path.display().to_string()), "{text}");
        daemon
    }

    fn isograph(&self, args: &[&str]) -> Output {
        let home = self.dir.path().join("home");
        std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
        Command::new(isograph_bin())
            .args(args)
            .current_dir(self.dir.path())
            .env("HOME", home.reference())
            .env("XDG_STATE_HOME", home.join("state"))
            .env("LOCALAPPDATA", home.join("appdata"))
            .output()
            .expect("the isograph binary runs")
    }

    fn log_text(&self) -> String {
        let home = self.dir.path().join("home");
        let mut out = String::new();
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
                    let Ok(text) = std::fs::read_to_string(path.reference()) else {
                        continue;
                    };
                    out.push_str(text.reference());
                }
            }
        }
        out
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.isograph(["stop", "--force"].reference());
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

/// The path as it appears in the daemon's JSON log, where `\` is escaped.
fn path_in_json_log(path: &str) -> String {
    path.replace('\\', "\\\\")
}

// freddie_cli's stop without --force is SIGTERM, which it does not send on Windows.
#[cfg(windows)]
const STOP: &[&str] = &["stop", "--force"];
#[cfg(not(windows))]
const STOP: &[&str] = &["stop"];

#[test]
fn start_then_status_reports_running() {
    let daemon = Daemon::start();
    let status = daemon.isograph(["status"].reference());
    assert!(status.status.success());
    let text = stdout(status.reference());
    let path = daemon
        .dir
        .path()
        .join("isograph.config.json")
        .canonicalize()
        .expect("the fixture exists");
    assert!(text.contains("is running"), "{text}");
    assert!(text.contains(&path.display().to_string()), "{text}");
}

#[test]
fn the_log_contains_the_config_path() {
    let daemon = Daemon::start();
    let path = daemon
        .dir
        .path()
        .join("isograph.config.json")
        .canonicalize()
        .expect("the fixture exists")
        .display()
        .to_string();
    let path_in_log = path_in_json_log(path.reference());
    poll(|| {
        let log = daemon.log_text();
        (log.contains("isograph daemon up")
            && log.contains(path_in_log.reference())
            && log.contains("\"port\":"))
        .then_some(())
    });
}

#[test]
fn stop_then_status_reports_not_running() {
    let daemon = Daemon::start();
    assert!(daemon.isograph(["status"].reference()).status.success());
    let stopped = daemon.isograph(STOP);
    assert!(
        stopped.status.success(),
        "stdout: {} stderr: {}",
        stdout(stopped.reference()),
        stderr(stopped.reference())
    );
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    #[cfg(not(windows))]
    poll(|| {
        let log = daemon.log_text();
        (log.contains("SIGTERM: quitting") && log.contains("kill: exiting")).then_some(())
    });
}

#[test]
fn a_second_start_adopts_the_running_daemon() {
    let daemon = Daemon::start();
    let again = daemon.isograph(["start"].reference());
    assert!(again.status.success());
    assert!(
        stdout(again.reference()).contains("already running"),
        "{}",
        stdout(again.reference())
    );
    assert!(daemon.isograph(["status"].reference()).status.success());
}
