# CI builds and ships the isograph binary

Requires isograph-cli.md.

`crates/isograph_cli` is excluded from the workspace, so `cargo test`, `cargo clippy`, and `pnpm build-compiler` never compile it. CI must build it on its own lockfile. Release still drops the binary into `libs/isograph-compiler/artifacts/{platform}/isograph_cli`, and `cli.js` / `index.js` still pick that file.

Supported platforms, same as `index.js`:

- `macos-x64` (`x86_64-apple-darwin`)
- `macos-arm64` (`aarch64-apple-darwin`)
- `linux-x64` (`x86_64-unknown-linux-gnu`)
- `linux-arm64` (`aarch64-unknown-linux-gnu`)
- `win-x64` (`x86_64-pc-windows-msvc`)

The cargo bin name is `isograph` (`isograph.exe` on Windows). The artifact file name stays `isograph_cli` (`isograph_cli.exe` on Windows) so `index.js` does not change.

Each platform is two jobs: build and upload the release binary, then download that artifact and run every e2e test against it. The runner matches the target so the tests can execute the download: no `cross`.

## Change 1: PR CI clippy's the crate

`.github/workflows/ci.yml` gains `cargo-clippy-cli`. `all-checks-passed` waits on it and on the five platform jobs from Change 3.

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
```

## Change 2: e2e tests take the binary from `ISOGRAPH_BIN`

`tests/cli.rs` runs the path in `ISOGRAPH_BIN` when that environment variable is set. When it is absent, the binary cargo built for the test (`CARGO_BIN_EXE_isograph`). Local `cargo test` is unchanged. CI sets `ISOGRAPH_BIN` to the downloaded artifact.

```rust
// from crates/isograph_cli/tests/cli.rs
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use prelude::Postfix;

fn isograph_bin() -> PathBuf {
    match std::env::var_os("ISOGRAPH_BIN") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env!("CARGO_BIN_EXE_isograph")),
    }
}

impl Daemon {
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
}
```

## Change 3: five-platform build and upload, then download and test

`.github/workflows/build-cli.yml` builds this crate, not the workspace. `pnpm build-compiler` is `cargo build` at the root and cannot see `isograph_cli`. Two jobs per platform: `build` compiles `--release --target` and uploads; `test` downloads that file and runs `cargo test` with `ISOGRAPH_BIN` pointing at it. Every e2e test runs on every platform.

Native runners, so the tests can execute the download:

- `linux-x64`: `ubuntu-latest`
- `linux-arm64`: `ubuntu-24.04-arm`
- `macos-x64`: `macos-15-intel`
- `macos-arm64`: `macos-latest`
- `win-x64`: `windows-latest`

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

After, the whole workflow. Drop `cross` and `musl`. Drop pnpm, Node, and the turbo cache: this crate is cargo-only. `artifact-file` is the uploaded name (`isograph_cli` or `isograph_cli.exe`). `build-name` is the cargo output (`isograph` or `isograph.exe`).

The test job compiles the test harness (cargo still builds a bin as a dependency of `tests/`). The process the tests spawn is the download, not that cargo bin. HOME isolation is the tests' harness, not the workflow.

```yaml
# from .github/workflows/build-cli.yml
on:
  workflow_call:
    inputs:
      target:
        required: true
        type: string
      os:
        required: true
        type: string
      build-name:
        required: true
        type: string
      artifact-name:
        required: true
        type: string
      artifact-file:
        required: true
        type: string
      longpaths:
        required: false
        type: boolean

jobs:
  build:
    name: Build ${{ inputs.artifact-name }}
    timeout-minutes: 15
    runs-on: ${{ inputs.os }}
    steps:
      - uses: actions/checkout@v2
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          override: true
          target: ${{ inputs.target }}
      - name: Enable longer pathnames for git
        if: inputs.longpaths
        run: git config --system core.longpaths true
      - name: 'Build isograph (${{inputs.target}})'
        run: cargo build --manifest-path crates/isograph_cli/Cargo.toml --release --target ${{ inputs.target }}
      - name: Name the artifact isograph_cli
        shell: bash
        run: |
          src="crates/isograph_cli/target/${{ inputs.target }}/release/${{ inputs.build-name }}"
          mkdir -p artifact
          cp "$src" "artifact/${{ inputs.artifact-file }}"
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ inputs.artifact-name }}
          path: artifact/${{ inputs.artifact-file }}
          if-no-files-found: error

  test:
    name: Test ${{ inputs.artifact-name }}
    needs: build
    timeout-minutes: 15
    runs-on: ${{ inputs.os }}
    steps:
      - uses: actions/checkout@v2
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          override: true
      - uses: actions/download-artifact@v4
        with:
          name: ${{ inputs.artifact-name }}
          path: artifact
      - name: Make the artifact executable
        if: runner.os != 'Windows'
        run: chmod +x "artifact/${{ inputs.artifact-file }}"
      - name: 'Test isograph (${{inputs.target}})'
        env:
          ISOGRAPH_BIN: ${{ github.workspace }}/artifact/${{ inputs.artifact-file }}
        run: cargo test --manifest-path crates/isograph_cli/Cargo.toml --tests
```

`ci.yml` calls the workflow once per platform. These jobs are in `all-checks-passed.needs` with `cargo-clippy-cli`. `main-release` and `versioned-release` already download the five artifact names into `libs/isograph-compiler/artifacts/...`. Those names stay.

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
      os: ubuntu-24.04-arm
      build-name: isograph
      artifact-name: isograph_cli-bin-linux-arm64
      artifact-file: isograph_cli

  build-cli-macos-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-apple-darwin
      os: macos-15-intel
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
      build-name: isograph.exe
      artifact-name: isograph_cli-bin-win-x64
      artifact-file: isograph_cli.exe
      longpaths: true
```

`all-checks-passed.needs` appends `cargo-clippy-cli`, `build-cli-linux-x64`, `build-cli-linux-arm64`, `build-cli-macos-x64`, `build-cli-macos-arm64`, `build-cli-win-x64`.

`index.js` and `cli.js` are unchanged. `iso` still spawns the platform file under `artifacts/`.
