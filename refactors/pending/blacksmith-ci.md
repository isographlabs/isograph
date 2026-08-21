# CI jobs run on Blacksmith

GitHub Actions for `isographlabs/isograph` keep the same workflow files, job names, steps, artifacts, secrets, and `if:` gates. Jobs that currently name a GitHub-hosted runner instead name a Blacksmith runner of the same OS, arch, and vCPU class as today's GitHub standard (4 vCPU Linux/Windows, 6 vCPU macOS ARM).

`build-cli.yml` stays `runs-on: ${{ inputs.os }}`. Callers pass the tag. Steps, `actions/cache@v4`, `actions/setup-node` `cache: 'pnpm'`, and `actions-rust-lang/setup-rust-toolchain` stay. On a Blacksmith runner those cache actions use Blacksmith's colocated cache.

Tags:

- Linux x64: `blacksmith-4vcpu-ubuntu-2404`
- Linux ARM: `blacksmith-4vcpu-ubuntu-2404-arm`
- macOS ARM: `blacksmith-6vcpu-macos-latest`
- Windows: `blacksmith-4vcpu-windows-2025`
- macOS x64: `macos-15-intel`

`cli-macos-x64` keeps `os: macos-15-intel`. It still compiles and runs e2e for `x86_64-apple-darwin` on GitHub-hosted Intel.

A contributor still opens the GitHub Actions tab. Job logs show the Blacksmith tag as the runner. `all-checks-passed` still gates merge. An org member signed into `app.blacksmith.sh` sees the same run under Run History.

Blacksmith bills after 3000 x64 2vCPU-equivalent minutes per org per month. A wall minute on `blacksmith-4vcpu-ubuntu-2404` counts as 2 of those. A wall minute on `blacksmith-4vcpu-ubuntu-2404-arm` counts as 1.25. A wall minute on `blacksmith-4vcpu-windows-2025` counts as 4. A wall minute on `blacksmith-6vcpu-macos-latest` counts as 20. Paid rates: Ubuntu x64 4vCPU `$0.008/min`, Ubuntu ARM 4vCPU `$0.005/min`, Windows 4vCPU `$0.016/min`, macOS 6vCPU `$0.08/min`. Artifact upload and GitHub Pages stay on GitHub's bill.

## Change 1: Blacksmith GitHub App on `isographlabs`

An `isographlabs` org owner:

1. Opens `https://app.blacksmith.sh` and signs in with GitHub.
2. Installs the Blacksmith GitHub App on the `isographlabs` organization, repository `isograph` only.
3. In GitHub: Organization Settings → Actions → Runner groups → the group Blacksmith created. Enable public repositories. Restrict the group to `isographlabs/isograph`.
4. In GitHub: `isographlabs/isograph` Settings → Actions → General → Fork pull request workflows. "Require approval for first-time contributors" stays on.
5. In the Blacksmith dashboard settings, branch-protected caches stay on (the default).

Until step 3, a job with a `blacksmith-*` tag queues with "Waiting for a runner to pick up this job". After step 3, a push to `isographlabs/isograph` that already has a Blacksmith tag is picked up. A first-time fork PR still waits on GitHub approval; after approval it runs on Blacksmith with GitHub's read-only `GITHUB_TOKEN` and no secrets.

The App mints a one-hour JIT runner token per job. Linux and Windows jobs boot in Firecracker microVMs from GitHub's runner images. macOS jobs boot from GitHub's corresponding macOS image on M4.

## Change 2: Linux x64 jobs

Every `runs-on: ubuntu-latest` in the three workflow files becomes `runs-on: blacksmith-4vcpu-ubuntu-2404`. `cli-linux-x64` passes that tag as `os`.

Jobs in `.github/workflows/ci.yml` whose `runs-on` changes: `typecheck-demos`, `build-js-packages`, `build-website`, `prettier`, `lint`, `cargo-fmt`, `cargo-clippy`, `cargo-clippy-cli`, `cargo-test`, `build-swc`, `all-checks-passed`, `deploy-website`, `main-release`, `versioned-release`.

Before:

```yaml
# from .github/workflows/ci.yml
  typecheck-demos:
    name: Typecheck and Lint Demos
    runs-on: ubuntu-latest

  build-js-packages:
    name: Build js packages
    runs-on: ubuntu-latest

  build-website:
    name: Build website
    runs-on: ubuntu-latest

  prettier:
    name: Run prettier
    runs-on: ubuntu-latest

  lint:
    name: Run lint
    runs-on: ubuntu-latest

  cargo-fmt:
    name: cargo fmt
    runs-on: ubuntu-latest

  cargo-clippy:
    name: cargo clippy
    runs-on: ubuntu-latest

  cargo-clippy-cli:
    name: cargo clippy isograph_cli
    runs-on: ubuntu-latest

  cargo-test:
    name: Run cargo test (excluding relay tests)
    runs-on: ubuntu-latest

  build-swc:
    name: Build swc
    runs-on: ubuntu-latest

  cli-linux-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-unknown-linux-gnu
      os: ubuntu-latest
      build-name: isograph
      artifact-name: isograph_cli-linux-x64
      artifact-file: isograph_cli

  all-checks-passed:
    name: All checks passed
    runs-on: ubuntu-latest

  deploy-website:
    name: Deploy website
    runs-on: ubuntu-latest

  main-release:
    name: Main NPM release
    runs-on: ubuntu-latest

  versioned-release:
    name: Versioned NPM release
    runs-on: ubuntu-latest
```

```yaml
# from .github/workflows/run-cargo-bin-and-ensure-no-changes.yml
jobs:
  build-and-run-binary:
    name: Run ${{ inputs.binary }}
    runs-on: ubuntu-latest
```

```yaml
# from .github/workflows/publish-isograph-extension.yml
jobs:
  publish:
    runs-on: ubuntu-latest
```

After:

```yaml
# from .github/workflows/ci.yml
  typecheck-demos:
    name: Typecheck and Lint Demos
    runs-on: blacksmith-4vcpu-ubuntu-2404

  build-js-packages:
    name: Build js packages
    runs-on: blacksmith-4vcpu-ubuntu-2404

  build-website:
    name: Build website
    runs-on: blacksmith-4vcpu-ubuntu-2404

  prettier:
    name: Run prettier
    runs-on: blacksmith-4vcpu-ubuntu-2404

  lint:
    name: Run lint
    runs-on: blacksmith-4vcpu-ubuntu-2404

  cargo-fmt:
    name: cargo fmt
    runs-on: blacksmith-4vcpu-ubuntu-2404

  cargo-clippy:
    name: cargo clippy
    runs-on: blacksmith-4vcpu-ubuntu-2404

  cargo-clippy-cli:
    name: cargo clippy isograph_cli
    runs-on: blacksmith-4vcpu-ubuntu-2404

  cargo-test:
    name: Run cargo test (excluding relay tests)
    runs-on: blacksmith-4vcpu-ubuntu-2404

  build-swc:
    name: Build swc
    runs-on: blacksmith-4vcpu-ubuntu-2404

  cli-linux-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-unknown-linux-gnu
      os: blacksmith-4vcpu-ubuntu-2404
      build-name: isograph
      artifact-name: isograph_cli-linux-x64
      artifact-file: isograph_cli

  all-checks-passed:
    name: All checks passed
    runs-on: blacksmith-4vcpu-ubuntu-2404

  deploy-website:
    name: Deploy website
    runs-on: blacksmith-4vcpu-ubuntu-2404

  main-release:
    name: Main NPM release
    runs-on: blacksmith-4vcpu-ubuntu-2404

  versioned-release:
    name: Versioned NPM release
    runs-on: blacksmith-4vcpu-ubuntu-2404
```

```yaml
# from .github/workflows/run-cargo-bin-and-ensure-no-changes.yml
jobs:
  build-and-run-binary:
    name: Run ${{ inputs.binary }}
    runs-on: blacksmith-4vcpu-ubuntu-2404
```

```yaml
# from .github/workflows/publish-isograph-extension.yml
jobs:
  publish:
    runs-on: blacksmith-4vcpu-ubuntu-2404
```

`build-swc` and `run-cargo-bin-and-ensure-no-changes.yml` still install `musl-tools` / target `x86_64-unknown-linux-musl`. The Blacksmith Ubuntu 24.04 image is GitHub's Ubuntu 24.04 image.

`deploy-website` still uses `actions/configure-pages@v4` and `actions/deploy-pages@v4` with workflow `permissions.pages: write` and `permissions.id-token: write`.

A push to a branch on `isographlabs/isograph` after this change: Linux x64 jobs in the Actions log run on `blacksmith-4vcpu-ubuntu-2404`. `cli-linux-arm64`, `cli-macos-x64`, `cli-macos-arm64`, and `cli-win-x64` still use the GitHub-hosted tags they have now. `all-checks-passed` still waits on all of them.

## Change 3: Linux ARM CLI

Before:

```yaml
# from .github/workflows/ci.yml
  cli-linux-arm64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: aarch64-unknown-linux-gnu
      os: ubuntu-24.04-arm
      build-name: isograph
      artifact-name: isograph_cli-bin-linux-arm64
      artifact-file: isograph_cli
```

After:

```yaml
# from .github/workflows/ci.yml
  cli-linux-arm64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: aarch64-unknown-linux-gnu
      os: blacksmith-4vcpu-ubuntu-2404-arm
      build-name: isograph
      artifact-name: isograph_cli-bin-linux-arm64
      artifact-file: isograph_cli
```

`build-cli.yml` build and test jobs for this caller run on that ARM tag. Artifact name `isograph_cli-bin-linux-arm64` is unchanged. `main-release` / `versioned-release` still download it into `libs/isograph-compiler/artifacts/linux-arm64`.

## Change 4: macOS ARM CLI

Before:

```yaml
# from .github/workflows/ci.yml
  cli-macos-arm64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: aarch64-apple-darwin
      os: macos-latest
      build-name: isograph
      artifact-name: isograph_cli-macos-arm64
      artifact-file: isograph_cli
```

After:

```yaml
# from .github/workflows/ci.yml
  cli-macos-arm64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: aarch64-apple-darwin
      os: blacksmith-6vcpu-macos-latest
      build-name: isograph
      artifact-name: isograph_cli-macos-arm64
      artifact-file: isograph_cli
```

`cli-macos-x64` is unchanged:

```yaml
# from .github/workflows/ci.yml
  cli-macos-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-apple-darwin
      os: macos-15-intel
      build-name: isograph
      artifact-name: isograph_cli-macos-x64
      artifact-file: isograph_cli
```

## Change 5: Windows CLI

Blacksmith Windows is GitHub's Windows Server 2025 image with VS Build Tools 2022. `cli-win-x64` targets `x86_64-pc-windows-msvc`; `link.exe` comes from those Build Tools. `longpaths: true` still runs `git config --system core.longpaths true`.

Before:

```yaml
# from .github/workflows/ci.yml
  cli-win-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-pc-windows-msvc
      os: windows-latest
      build-name: isograph.exe
      artifact-name: isograph_cli-bin-win-x64
      artifact-file: isograph_cli.exe
      longpaths: true
```

After:

```yaml
# from .github/workflows/ci.yml
  cli-win-x64:
    uses: ./.github/workflows/build-cli.yml
    with:
      target: x86_64-pc-windows-msvc
      os: blacksmith-4vcpu-windows-2025
      build-name: isograph.exe
      artifact-name: isograph_cli-bin-win-x64
      artifact-file: isograph_cli.exe
      longpaths: true
```
