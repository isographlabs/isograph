//! Drive the built `isograph` binary. Every daemon's lock and log live under a private HOME.

use std::process::{Command, Output};
use std::time::{Duration, Instant};

use prelude::Postfix;

const DEADLINE: Duration = Duration::from_secs(10);

struct Daemon {
    dir: tempfile::TempDir,
}

impl Daemon {
    fn start() -> Self {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let daemon = Self { dir };
        let output = daemon.isograph(["start"].reference());
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(output.stderr.reference())
        );
        daemon
    }

    fn isograph(&self, args: &[&str]) -> Output {
        let home = self.dir.path().join("home");
        std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
        Command::new(env!("CARGO_BIN_EXE_isograph"))
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
                    if let Ok(text) = std::fs::read_to_string(path.reference()) {
                        out.push_str(text.reference());
                    }
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

#[test]
fn start_then_status_reports_running() {
    let daemon = Daemon::start();
    let status = daemon.isograph(["status"].reference());
    assert!(status.status.success());
    assert!(
        stdout(status.reference()).contains("is running"),
        "{}",
        stdout(status.reference())
    );
}

#[test]
fn the_log_contains_hello_from_isograph() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("hello from isograph")
            .then_some(())
    });
}

#[test]
fn stop_then_status_reports_not_running() {
    let daemon = Daemon::start();
    assert!(daemon.isograph(["status"].reference()).status.success());
    let stopped = daemon.isograph(["stop"].reference());
    assert!(stopped.status.success());
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
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
