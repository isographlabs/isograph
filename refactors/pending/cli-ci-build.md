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

Each platform job runs `cargo test --release --target` for this crate, then uploads that release binary. Every e2e test runs on every platform. The runner matches the target so the tests execute the binary they just built: no `cross`.

## Change 1: PR CI clippy's the crate

`.github/workflows/ci.yml` gains `cargo-clippy-cli`. `all-checks-passed` waits on it and on the five platform jobs from Change 2.

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

## Change 2: five-platform build and e2e

`.github/workflows/build-cli.yml` builds this crate, not the workspace. `pnpm build-compiler` is `cargo build` at the root and cannot see `isograph_cli`. One job per platform: `cargo test --release --target`, copy that release binary to `artifact/`, upload.

Native runners, so the tests run the binary:

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

`cargo test --release --target` compiles the bin and runs every test in this crate, including e2e under `tests/`. The integration tests drive `CARGO_BIN_EXE_isograph`, which is that release binary. HOME isolation is the tests' harness, not the workflow.

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
  build-cli:
    name: Build and test ${{ inputs.artifact-name }}
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
      - name: 'Test isograph (${{inputs.target}})'
        run: cargo test --manifest-path crates/isograph_cli/Cargo.toml --release --target ${{ inputs.target }}
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
