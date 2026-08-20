# CI builds and ships the isograph binary

Requires isograph-cli.md.

`crates/isograph_cli` is excluded from the workspace, so `cargo test`, `cargo clippy`, and `pnpm build-compiler` never compile it. CI must build it on its own lockfile. Release still drops the binary into `libs/isograph-compiler/artifacts/{platform}/isograph_cli`, and `cli.js` / `index.js` still pick that file.

Supported platforms, same as `index.js`:

- `macos-x64` (`x86_64-apple-darwin`)
- `macos-arm64` (`aarch64-apple-darwin`)
- `linux-x64` (`x86_64-unknown-linux-gnu`)
- `linux-arm64` (`aarch64-unknown-linux-gnu`)
- `win-x64` (`x86_64-pc-windows-msvc`)

The cargo bin name is `isograph`. The artifact file name stays `isograph_cli` (`isograph_cli.exe` on Windows) so `index.js` does not change.

## Change 1: PR CI compiles, tests, and clippy's the crate

`.github/workflows/ci.yml` gains three jobs, and `all-checks-passed` waits on them.

`cargo-fmt` already runs `cargo fmt --manifest-path crates/isograph_cli/Cargo.toml`. Leave it.

```yaml
# from .github/workflows/ci.yml
  cargo-clippy-cli:
    name: cargo clippy isograph_cli
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: clippy
      - name: Run cargo clippy
        run: cargo clippy --manifest-path crates/isograph_cli/Cargo.toml --all-targets -- -D warnings

  cargo-test-cli:
    name: cargo test isograph_cli
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
      - name: Run cargo test
        run: cargo test --manifest-path crates/isograph_cli/Cargo.toml

  build-cli:
    name: cargo build isograph_cli
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
      - name: Build
        run: cargo build --manifest-path crates/isograph_cli/Cargo.toml --release
```

`all-checks-passed.needs` appends `cargo-clippy-cli`, `cargo-test-cli`, `build-cli`.

## Tests

`cargo-test-cli` runs `crates/isograph_cli/tests/cli.rs`. Private HOME. The binary starts, status reports running, the log contains `hello from isograph`, stop then status reports not running, a second start adopts.

```rust
// from crates/isograph_cli/tests/cli.rs
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
```

## Change 2: five-platform release artifacts

`.github/workflows/build-cli.yml` builds this crate, not the workspace. `pnpm build-compiler` is `cargo build` at the root and cannot see `isograph_cli`.

Before, the build step:

```yaml
# from .github/workflows/build-cli.yml
      - name: 'Build isograph_cli with cargo (${{inputs.target}})'
        run: pnpm exec turbo ${{ inputs.cross && 'cross' || 'build-compiler' }} -- --target ${{ inputs.target }} --release
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ inputs.artifact-name }}
          path: target/${{ inputs.target }}/release/${{ inputs.build-name }}
          if-no-files-found: error
```

After:

```yaml
# from .github/workflows/build-cli.yml
      - name: 'Build isograph (${{inputs.target}})'
        run: cargo build --manifest-path crates/isograph_cli/Cargo.toml --release --target ${{ inputs.target }}
      - name: Name the artifact isograph_cli
        run: |
          src="crates/isograph_cli/target/${{ inputs.target }}/release/${{ inputs.build-name }}"
          mkdir -p artifact
          cp "$src" "artifact/${{ inputs.artifact-file }}"
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ inputs.artifact-name }}
          path: artifact/${{ inputs.artifact-file }}
          if-no-files-found: error
```

`build-cli.yml` gains an `artifact-file` input (`isograph_cli` or `isograph_cli.exe`). `build-name` is `isograph` (the cargo bin). Linux arm still sets `cross: true` if that target still needs it; the cargo line is the same, with `cross` swapped in only when that input is set:

```yaml
        run: ${{ inputs.cross && 'cross' || 'cargo' }} build --manifest-path crates/isograph_cli/Cargo.toml --release --target ${{ inputs.target }}
```

`ci.yml` calls the workflow once per platform. These jobs are in `all-checks-passed.needs`. `main-release` and `versioned-release` already download the five artifact names into `libs/isograph-compiler/artifacts/...`. Those names stay.

```yaml
# from .github/workflows/ci.yml
  build-cli-linux-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-unknown-linux-gnu
      os: ubuntu-latest
      build-name: isograph
      artifact-name: isograph_cli-linux-x64
      artifact-file: isograph_cli

  build-cli-linux-arm64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: aarch64-unknown-linux-gnu
      os: ubuntu-latest
      build-name: isograph
      artifact-name: isograph_cli-bin-linux-arm64
      artifact-file: isograph_cli
      cross: true

  build-cli-macos-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-apple-darwin
      os: macos-latest
      build-name: isograph
      artifact-name: isograph_cli-macos-x64
      artifact-file: isograph_cli

  build-cli-macos-arm64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: aarch64-apple-darwin
      os: macos-latest
      build-name: isograph
      artifact-name: isograph_cli-macos-arm64
      artifact-file: isograph_cli

  build-cli-win-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-pc-windows-msvc
      os: windows-latest
      build-name: isograph
      artifact-name: isograph_cli-bin-win-x64
      artifact-file: isograph_cli.exe
      longpaths: true
```

`index.js` and `cli.js` are unchanged. `iso` still spawns the platform file under `artifacts/`.
