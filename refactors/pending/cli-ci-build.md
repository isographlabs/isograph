# CI builds and ships the isograph binary

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
