//! Drive the built `isograph` binary. Every daemon's lock and log live under a private HOME.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use lsp_server::{Message, Request, RequestId};
use lsp_types::notification::{Initialized, Notification};
use lsp_types::request::{Initialize, Request as LspRequest, SemanticTokensFullRequest};
use prelude::Postfix;

const DEADLINE: Duration = Duration::from_secs(10);
const SETTLE: Duration = Duration::from_millis(250);

fn settle() {
    std::thread::sleep(SETTLE);
}

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
        std::fs::write(config.reference(), "{\"source_files\":[]}\n")
            .expect("a test can write a config file");
        let daemon = Self { dir };
        let output = daemon.isograph(["start", "--filesystem", "injected"].reference());
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(output.stderr.reference())
        );
        let text = stdout(output.reference());
        let path = config.canonicalize().expect("the fixture exists");
        assert!(text.contains("started"), "{text}");
        assert!(text.contains(&path.display().to_string()), "{text}");
        settle();
        assert!(
            daemon.log_text().contains("isograph daemon up"),
            "{}",
            daemon.log_text()
        );
        daemon
    }

    fn start_watch(source_files: &str) -> Self {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let config = dir.path().join("isograph.config.json");
        std::fs::write(config.reference(), source_files).expect("a test can write a config file");
        let daemon = Self { dir };
        let output = daemon.isograph(["start"].reference());
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(output.stderr.reference())
        );
        settle();
        assert!(
            daemon.log_text().contains("isograph daemon up"),
            "{}",
            daemon.log_text()
        );
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

fn write_frame(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("frame.json");
    std::fs::write(path.reference(), contents).expect("a test can write a frame");
    path
}

fn daemon_port(daemon: &Daemon) -> u16 {
    let home = daemon.dir.path().join("home");
    let mut stack = home.wrap_vec();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(dir.reference()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "port") {
                let text = std::fs::read_to_string(path.reference()).expect("the port file");
                return text.trim().parse().expect("the port file is a port");
            }
        }
    }
    panic!("the daemon wrote a port file");
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
fn start_with_missing_source_files_exits_1() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let home = dir.path().join("home");
    std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
    let output = Command::new(isograph_bin())
        .args(["start"].reference())
        .current_dir(dir.path())
        .env("HOME", home.reference())
        .env("XDG_STATE_HOME", home.join("state"))
        .env("LOCALAPPDATA", home.join("appdata"))
        .output()
        .expect("the isograph binary runs");
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(err.contains("source_files"), "{err}");
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
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| daemon.log_text().contains("hello world").then_some(()));
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_of_disk_changed_present_then_absent_exits_0() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    let present = write_frame(
        daemon.dir.path(),
        r#"{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}}"#,
    );
    let sent = daemon.isograph(["send", "--file", present.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    assert!(!present.exists(), "send deletes --file");
    let absent = write_frame(
        daemon.dir.path(),
        r#"{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/a.ts","presence":"Absent"}}}"#,
    );
    let sent = daemon.isograph(["send", "--file", absent.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    assert!(!absent.exists(), "send deletes --file");
}

#[test]
fn send_of_folder_removed_exits_0() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    let present = write_frame(
        daemon.dir.path(),
        r#"{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}}"#,
    );
    let sent = daemon.isograph(["send", "--file", present.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    let removed = write_frame(
        daemon.dir.path(),
        r#"{"kind":"DiskChanged","value":{"FolderRemoved":{"path":"/tmp/proj/src"}}}"#,
    );
    let sent = daemon.isograph(["send", "--file", removed.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    assert!(!removed.exists(), "send deletes --file");
}

#[test]
fn send_with_the_daemon_stopped_fails() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let frame = write_frame(dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let home = dir.path().join("home");
    std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
    let output = Command::new(isograph_bin())
        .args(["send", "--file", frame.to_str().expect("utf-8")].reference())
        .current_dir(dir.path())
        .env("HOME", home.reference())
        .env("XDG_STATE_HOME", home.join("state"))
        .env("LOCALAPPDATA", home.join("appdata"))
        .output()
        .expect("the isograph binary runs");
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(err.contains("not running"), "{err}");
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_of_not_json_fails() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    let frame = write_frame(daemon.dir.path(), "not json\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(!sent.status.success());
    let err = stderr(sent.reference());
    assert!(err.contains("Internal"), "{err}");
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_of_unknown_kind_fails() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"Nope\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(!sent.status.success());
    let err = stderr(sent.reference());
    assert!(err.contains("Internal"), "{err}");
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_is_not_in_help() {
    let output = Command::new(isograph_bin())
        .arg("--help")
        .output()
        .expect("the isograph binary runs");
    assert!(output.status.success());
    let text = stdout(output.reference());
    assert!(text.contains("start"), "{text}");
    assert!(!text.contains("send"), "{text}");
}

#[test]
fn stop_after_send_exits_without_force() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| daemon.log_text().contains("hello world").then_some(()));
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
fn two_sends_then_stop_exits_without_force() {
    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    for _ in 0..2 {
        let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
        let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
        assert!(
            sent.status.success(),
            "stdout: {} stderr: {}",
            stdout(sent.reference()),
            stderr(sent.reference())
        );
    }
    poll(|| daemon.log_text().contains("hello world").then_some(()));
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
fn truncated_body_does_not_kill_the_daemon() {
    use std::io::Write;

    let daemon = Daemon::start();
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
    let port = daemon_port(daemon.reference());
    {
        let mut stream = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
            .expect("connecting to the daemon");
        stream
            .write_all(b"Content-Length: 100\r\n\r\n{")
            .expect("writing a truncated body");
    }
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| daemon.log_text().contains("hello world").then_some(()));
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
fn start_help_contains_filesystem() {
    let output = Command::new(isograph_bin())
        .args(["start", "--help"].reference())
        .output()
        .expect("the isograph binary runs");
    assert!(output.status.success());
    let text = stdout(output.reference());
    assert!(text.contains("filesystem"), "{text}");
}

#[test]
fn watch_of_empty_source_files_logs_scan_finished() {
    let daemon = Daemon::start_watch("{\"source_files\":[]}\n");
    poll(|| {
        let log = daemon.log_text();
        (log.contains("scan finished") && log.contains("isograph daemon up")).then_some(())
    });
}

#[test]
fn watch_interns_matching_files_and_skips_the_rest() {
    let daemon = {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let config = dir.path().join("isograph.config.json");
        std::fs::write(
            config.reference(),
            r#"{"source_files":["src/**/*.ts","!src/**/*.test.ts"]}"#,
        )
        .expect("a test can write a config file");
        let src = dir.path().join("src");
        std::fs::create_dir_all(src.reference()).expect("a test can create src");
        std::fs::write(
            src.join("Home.ts"),
            "export const Home = iso(`entrypoint Query.HomeRoute`)\n",
        )
        .expect("a test can write Home.ts");
        let daemon = Daemon { dir };
        let output = daemon.isograph(["start"].reference());
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(output.stderr.reference())
        );
        daemon
    };
    poll(|| {
        let log = daemon.log_text();
        (log.contains("disk present") && log.contains("Home.ts")).then_some(())
    });
    let src = daemon.dir.path().join("src");
    std::fs::write(
        src.join("Other.ts"),
        "export const Other = iso(`entrypoint Query.Other`)\n",
    )
    .expect("a test can write Other.ts");
    poll(|| daemon.log_text().contains("Other.ts").then_some(()));
    std::fs::write(src.join("skip.rs"), "fn skip() {}\n").expect("a test can write skip.rs");
    std::fs::write(
        src.join("Home.test.ts"),
        "export const HomeTest = iso(`entrypoint Query.HomeTest`)\n",
    )
    .expect("a test can write Home.test.ts");
    std::thread::sleep(Duration::from_millis(500));
    let log = daemon.log_text();
    assert!(!log.contains("skip.rs"), "skip.rs should not intern: {log}");
    assert!(
        !log.contains("Home.test.ts"),
        "Home.test.ts should not intern: {log}"
    );
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

#[test]
fn config_path_prints_the_canonical_path() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let nested = dir.path().join("src");
    std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
    let output = Command::new(isograph_bin())
        .args(["config-path"].reference())
        .current_dir(nested.reference())
        .output()
        .expect("the isograph binary runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        stderr(output.reference())
    );
    let expected = config.canonicalize().expect("the fixture exists");
    assert_eq!(
        stdout(output.reference()).trim(),
        expected.to_str().expect("utf-8")
    );
}

#[test]
fn config_path_with_flag_prints_that_file() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let output = Command::new(isograph_bin())
        .args(["config-path", "--config", config.to_str().expect("utf-8")].reference())
        .current_dir(dir.path())
        .output()
        .expect("the isograph binary runs");
    assert!(output.status.success());
    let expected = config.canonicalize().expect("the fixture exists");
    assert_eq!(
        stdout(output.reference()).trim(),
        expected.to_str().expect("utf-8")
    );
}

#[test]
fn config_path_with_no_config_exits_1() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let output = Command::new(isograph_bin())
        .args(["config-path"].reference())
        .current_dir(dir.path())
        .output()
        .expect("the isograph binary runs");
    assert!(!output.status.success());
    assert!(stdout(output.reference()).is_empty());
    let err = stderr(output.reference());
    assert!(err.contains("no isograph.config.json"), "{err}");
}

struct LspClient {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    next_id: i32,
}

impl LspClient {
    fn connect(port: u16) -> Self {
        let stream =
            TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connecting to the daemon");
        let writer = stream.try_clone().expect("cloning the stream");
        let mut client = Self {
            writer,
            reader: BufReader::new(stream),
            next_id: 2,
        };
        client.initialize();
        client
    }

    fn initialize(&mut self) {
        let id = RequestId::from(1);
        write_message(
            &mut self.writer,
            Message::Request(Request {
                id: id.clone(),
                method: Initialize::METHOD.to_owned(),
                params: serde_json::json!({ "capabilities": {} }),
            }),
        );
        let response = read_response(&mut self.reader, id.reference());
        assert!(response.error.is_none(), "{response:?}");
        write_message(
            &mut self.writer,
            Message::Notification(lsp_server::Notification {
                method: Initialized::METHOD.to_owned(),
                params: serde_json::json!({}),
            }),
        );
    }

    fn semantic_tokens_full(&mut self, uri: &str) -> Option<Vec<lsp_types::SemanticToken>> {
        let id = RequestId::from(self.next_id);
        self.next_id += 1;
        write_message(
            &mut self.writer,
            Message::Request(Request {
                id: id.clone(),
                method: SemanticTokensFullRequest::METHOD.to_owned(),
                params: serde_json::json!({ "textDocument": { "uri": uri } }),
            }),
        );
        tokens_from_response(read_response(&mut self.reader, id.reference()))
    }
}

fn write_message(writer: &mut impl Write, message: Message) {
    message.write(writer).expect("writing an lsp message");
}

fn read_message(reader: &mut impl BufRead) -> Message {
    Message::read(reader)
        .expect("reading an lsp message")
        .expect("the connection stayed open")
}

fn read_response(reader: &mut impl BufRead, expected: &RequestId) -> lsp_server::Response {
    loop {
        let Message::Response(response) = read_message(reader) else {
            continue;
        };
        assert_eq!(&response.id, expected);
        return response;
    }
}

fn tokens_from_response(response: lsp_server::Response) -> Option<Vec<lsp_types::SemanticToken>> {
    assert!(response.error.is_none(), "{response:?}");
    let value = response.result?;
    if value.is_null() {
        return None;
    }
    match serde_json::from_value(value).expect("the result is SemanticTokensResult") {
        lsp_types::SemanticTokensResult::Tokens(tokens) => tokens.data.wrap_some(),
        lsp_types::SemanticTokensResult::Partial(_) => {
            panic!("the daemon does not send partial semantic tokens")
        }
    }
}

fn source_path(daemon: &Daemon, relative: &str) -> std::path::PathBuf {
    daemon
        .dir
        .path()
        .canonicalize()
        .expect("the fixture exists")
        .join(relative)
}

fn file_uri(path: &std::path::Path) -> String {
    url::Url::from_file_path(path)
        .expect("the path is absolute")
        .to_string()
}

fn send_json(daemon: &Daemon, value: serde_json::Value) {
    let frame = write_frame(daemon.dir.path(), value.to_string().as_str());
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
}

fn present(path: &std::path::Path, contents: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "DiskChanged",
        "value": {
            "File": {
                "path": path,
                "presence": { "Present": contents }
            }
        }
    })
}

fn absent(path: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "kind": "DiskChanged",
        "value": {
            "File": {
                "path": path,
                "presence": "Absent"
            }
        }
    })
}

const CONTENTS: &str = "export const Home = iso(`entrypoint Query.HomeRoute`)";
const KEYWORD: u32 = 15;

#[test]
fn send_of_a_present_iso_literal_returns_entrypoint_as_keyword() {
    let daemon = Daemon::start();
    let path = source_path(daemon.reference(), "src/Home.ts");
    assert!(!path.exists(), "injected send does not write the path");
    send_json(daemon.reference(), present(path.reference(), CONTENTS));
    settle();
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let tokens = client
        .semantic_tokens_full(uri.as_str())
        .expect("the test interned this path");
    assert!(!path.exists(), "injected send does not write the path");
    assert_eq!(tokens[0].delta_line, 0);
    assert_eq!(tokens[0].token_type, KEYWORD);
    assert_eq!(tokens[0].length, 10);
    assert_eq!(
        tokens[0].delta_start,
        "export const Home = iso(`".encode_utf16().count() as u32
    );
}

#[test]
fn send_of_a_path_never_interned_returns_null_tokens() {
    let daemon = Daemon::start();
    let path = source_path(daemon.reference(), "src/missing.ts");
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    assert!(client.semantic_tokens_full(uri.as_str()).is_none());
}

#[test]
fn send_of_a_present_file_with_no_iso_returns_empty_data() {
    let daemon = Daemon::start();
    let path = source_path(daemon.reference(), "src/Home.ts");
    send_json(
        daemon.reference(),
        present(path.reference(), "export const x = 1;\n"),
    );
    settle();
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let tokens = client
        .semantic_tokens_full(uri.as_str())
        .expect("the test interned this path");
    assert!(tokens.is_empty());
}

#[test]
fn send_of_absent_after_present_returns_null_tokens() {
    let daemon = Daemon::start();
    let path = source_path(daemon.reference(), "src/Home.ts");
    send_json(daemon.reference(), present(path.reference(), CONTENTS));
    settle();
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let _present = client
        .semantic_tokens_full(uri.as_str())
        .expect("the test interned this path");
    send_json(daemon.reference(), absent(path.reference()));
    settle();
    assert!(client.semantic_tokens_full(uri.as_str()).is_none());
}

#[test]
fn send_prefix_increments_delta_line_and_keeps_keyword() {
    let daemon = Daemon::start();
    let path = source_path(daemon.reference(), "src/Home.ts");
    send_json(daemon.reference(), present(path.reference(), CONTENTS));
    settle();
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let before = client
        .semantic_tokens_full(uri.as_str())
        .expect("the test interned this path");
    send_json(
        daemon.reference(),
        present(path.reference(), &("const x = 1;\n".to_owned() + CONTENTS)),
    );
    settle();
    let after = client
        .semantic_tokens_full(uri.as_str())
        .expect("the test interned this path");
    assert_eq!(after[0].delta_line, 1);
    assert_eq!(after[0].delta_start, before[0].delta_start);
    assert_eq!(after[0].token_type, KEYWORD);
    assert_eq!(after[0].length, 10);
}
