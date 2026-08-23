# Zed and VS Code extensions

Requires `docs-website/docs/design-docs/event-model.md` and vscode-extension.md. The editors talk to the LSP adapter via `isograph lsp` (stdio proxy). VS Code is vscode-extension.md. This file is the Zed extension, how highlighting differs in each editor, and Zed CI. It does not ship before the adapter and the proxy exist (`refactors/pending/event-model.md` LSP adapter item).

## What the user does

VS Code: vscode-extension.md.

Zed: install the Isograph extension from the extensions page (or Install Dev Extension while iterating). Open the same file. Same coloring, after the user has set `semantic_tokens` to `combined` or `full` for TypeScript and TSX. Without that setting, Zed does not request semantic tokens and the iso literal stays a string.

## VS Code extension

vscode-extension.md, then vscode-config-discovery.md. Spawn `isograph lsp`, document selector the four JS/TS languages, settings `pathToIsograph` / `pathToConfig`. No format-on-save. No `GraphQL.vscode-graphql-syntax`. Publish remains `publish-isograph-extension.yml`.

## What a Zed extension is

A git repo with `extension.toml`. Procedural bits are Rust compiled to `wasm32-wasip2`. Zed loads that wasm. A language server the extension names is a native binary Zed spawns, not wasm. Tree-sitter grammars are wasm compiled from C, separate from the extension wasm.

Directory:

```
zed-extension/
  extension.toml
  Cargo.toml
  src/lib.rs
```

```toml
# from zed-extension/extension.toml
id = "isograph"
name = "Isograph"
version = "0.0.1"
schema_version = 1
authors = ["Isograph"]
description = "Isograph language server for JavaScript and TypeScript"
repository = "https://github.com/isographlabs/isograph"

[language_servers.isograph]
name = "Isograph Language Server"
languages = ["JavaScript", "JSX", "TypeScript", "TSX"]

[language_servers.isograph.language_ids]
"JavaScript" = "javascript"
"JSX" = "javascriptreact"
"TypeScript" = "typescript"
"TSX" = "typescriptreact"
```

The `languages` values are Zed's names for the built-in languages, matching each language's `config.toml` `name`. Iso literals live inside JS/TS files. This slice does not register an `isograph` language or a `.iso` file type.

GraphQL schema files (`.graphql`) are a separate language: tree-sitter, not `isograph lsp`. That is `refactors/pending/zed-graphql-schema.md`. Do not add `languages/isograph/`.

```toml
# from zed-extension/Cargo.toml
[package]
name = "isograph"
version = "0.0.1"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
prelude = { path = "../crates/prelude" }
zed_extension_api = "0.7.0"
```

`edition = "2021"` is what Zed's template uses. The extension crate is not a workspace member of i2: it targets `wasm32-wasip2` and `zed_extension_api`, and workspace lints (`postfix_constructors`, `print_stderr`) are not that crate's rules. Keep it out of `[workspace] members`.

```rust
// from zed-extension/src/lib.rs
use prelude::Postfix;
use zed_extension_api as zed;
use zed_extension_api::{LanguageServerId, Result, Worktree};

struct IsographExtension;

impl zed::Extension for IsographExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let command = language_server_binary(worktree)?;
        zed::Command {
            command,
            args: "lsp".to_owned().wrap_vec(),
            env: worktree.shell_env(),
        }
        .wrap_ok()
    }
}

fn language_server_binary(worktree: &Worktree) -> Result<String> {
    if let Some(path) = worktree.which("isograph") {
        return path.wrap_ok();
    }
    if let Some(path) = package_binary(worktree) {
        return path.wrap_ok();
    }
    "no isograph binary on PATH or in node_modules/@isograph/compiler"
        .to_owned()
        .wrap_err()
}

fn package_binary(worktree: &Worktree) -> Option<String> {
    let (os, arch) = zed::current_platform();
    let rel = match (os, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "artifacts/macos-arm64/isograph_cli",
        (zed::Os::Mac, zed::Architecture::X8664) => "artifacts/macos-x64/isograph_cli",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "artifacts/linux-arm64/isograph_cli",
        (zed::Os::Linux, zed::Architecture::X8664) => "artifacts/linux-x64/isograph_cli",
        (zed::Os::Windows, zed::Architecture::X8664) => "artifacts/win-x64/isograph_cli.exe",
        _ => return None,
    };
    worktree
        .read_text_file("node_modules/@isograph/compiler/package.json")
        .ok()?;
    format!("node_modules/@isograph/compiler/{rel}").wrap_some()
}

zed::register_extension!(IsographExtension);
```

Zed settings for an LSP go under `lsp.isograph.settings`. First slice: PATH / node_modules only, no `--config`. Walk-up from the worktree cwd (Zed sets cwd to the worktree root when spawning) finds the config the way the daemon already does.

`fn new() -> Self` is required by `zed::Extension`. No `unwrap`. `read_text_file` failure is `None`. Unsupported `current_platform` is `None`, then `language_server_command` returns `Err`. `zed_extension_api` 0.7.0 is the latest crates.io release.

## Comparison

Spawn:

- VS Code: Node `LanguageClient` with `command` + `args` + `cwd`. stdio JSON-RPC.
- Zed: wasm `language_server_command` returns `zed::Command { command, args, env }`. Zed spawns that native binary on stdio. Same `isograph lsp` binary.

Finding the binary:

- VS Code: `isograph.pathToIsograph`, or walk `node_modules/@isograph/compiler` from the config directory (vscode-config-discovery.md).
- Zed: `worktree.which("isograph")`, or `node_modules/@isograph/compiler/artifacts/<platform>/isograph_cli`. Wasm cannot use `std::fs` the way the VS Code code does; `Worktree` and `current_platform` are the API.

Which files:

- VS Code: `documentSelector` four language ids, then per-config globs (vscode-config-discovery.md).
- Zed: `languages = ["JavaScript", "JSX", "TypeScript", "TSX"]` on the language server. Zed already has those languages (tree-sitter + vtsls). Isograph is an additional language server on them, the way Deno or ESLint is. Users who list `language_servers` without `"..."` will not get it. Default is `"..."` which includes newly registered servers.

Highlighting:

- VS Code: `editor.semanticHighlighting.enabled` defaults on. The client requests `textDocument/semanticTokens/full`. Tokens from `isograph lsp` color the iso literal. Tree-sitter is not involved.
- Zed: highlighting of JS/TS is tree-sitter. `semantic_tokens` defaults to `off`. Combined or full must be set or Zed never asks. An extension cannot add `injections.scm` to Zed's built-in TypeScript grammar, so a tree-sitter isograph grammar would color `.iso` files we do not have, not `iso(\`...\`)` inside `.ts`. Iso-literal coloring in Zed is LSP semantic tokens plus a user setting.

Settings:

- VS Code: `contributes.configuration` in `package.json`, `workspace.getConfiguration('isograph')`.
- Zed: `lsp.isograph.settings` in user `settings.json`, plus `languages.TypeScript.semantic_tokens`. The extension can ship `semantic_token_rules.json` only next to a language it defines, not next to built-in TypeScript.

Activation:

- VS Code: `activationEvents` on language id, then `activate()` runs, then the client starts.
- Zed: opening a registered language starts the language servers listed for it. No JS `activate`.

Distribution:

- VS Code: vsce / Open VSX, `publish-isograph-extension.yml`.
- Zed: `zed-industries/extensions` submodule PR, or Install Dev Extension from a local folder.

Process model once the LSP adapter exists:

- Both spawn `isograph lsp`.
- That process is a stdio proxy onto the per-config daemon (`docs-website/docs/design-docs/event-model.md`). Several editor windows share one compiler. Until that proxy exists, both spawns fail the same way.

## Highlighting mechanism

The tokens are `isograph_parser::SemanticToken`, encoded as LSP `SemanticToken` by lsp-semantic-token-encoding.md, served on `textDocument/semanticTokens/full`. VS Code applies them by default. Zed applies them when `semantic_tokens` is not `off`.

The extension does not reimplement coloring. It starts the server. A TextMate grammar for VS Code or a tree-sitter grammar for Zed that tries to parse iso literals inside template strings is a second, divergent highlighter. Not this work.

Zed users who want iso coloring add:

```json
{
  "languages": {
    "TypeScript": { "semantic_tokens": "combined" },
    "TSX": { "semantic_tokens": "combined" },
    "JavaScript": { "semantic_tokens": "combined" },
    "JSX": { "semantic_tokens": "combined" }
  }
}
```

Document that in the extension README. There is no extension API that turns semantic tokens on for built-in languages.

## CI

What we can assert without an editor:

- `handle` and `isograph send` (filesystem-events.md). Already on `cargo test` / `build-cli.yml`.
- LSP: spawn `isograph lsp`, speak JSON-RPC, assert `initialize` capabilities include semantic tokens, `textDocument/didOpen` of a fixture, `textDocument/semanticTokens/full` returns a non-empty `data` whose first token_type is KEYWORD for a `field` literal. No VS Code, no Zed. This lands with the LSP adapter (`refactors/pending/event-model.md` item 6). Encoding unit tests already live in lsp-semantic-token-encoding.md.
- VS Code extension: vscode-extension.md (`npm run typecheck` / `lint` / `prettier-check`).
- Zed extension: `cargo check --target wasm32-wasip2 --manifest-path zed-extension/Cargo.toml`. Add the target in that job. This proves the wasm crate compiles. It does not prove Zed loads it.

What we cannot assert in CI:

- That a pixel in VS Code or Zed is the keyword color.
- That a user who left Zed `semantic_tokens` at `off` sees coloring. They will not.

Do not run Zed or VS Code in GitHub Actions as the primary test of highlighting. The LSP fixture is the test. The wasm compile is the test of the Zed crate. The VS Code typecheck is the test of the Node crate.

## Ordered changes (after `isograph lsp` and vscode-extension.md)

Each is independently shippable. VS Code is vscode-extension.md.

### Change 1: the Zed extension crate

`zed-extension/` as above: `extension.toml`, `Cargo.toml`, `src/lib.rs` with `language_server_command` that finds `isograph` and passes `lsp`. Not a workspace member.

README: Install Dev Extension, point at `zed-extension/`. Set `semantic_tokens` combined for TypeScript and TSX. Open a demo.

### Change 2: Zed CI job

```yaml
# from .github/workflows/ci.yml
  zed-extension:
    name: zed-extension wasm
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          target: wasm32-wasip2
      - run: cargo check --target wasm32-wasip2 --manifest-path zed-extension/Cargo.toml
```

`all-checks-passed.needs` appends `zed-extension`.

### Change 3: LSP protocol tests

A test in `crates/ts_graphql_react_isograph_cli/tests/` that spawns `CARGO_BIN_EXE_isograph` with `lsp`, writes LSP headers+JSON to stdin, reads stdout. Assert initialize result has `semanticTokensProvider`. Open a document whose text is `iso(\`field Pet.fullName { id }\`)`. Request semantic tokens. Assert `data` is non-empty and the first `token_type` is 15 (KEYWORD). HOME isolation as in `tests/cli.rs`.

This is the highlighting test. It does not import `vscode` or Zed.

### Change 4: GraphQL schema files

`refactors/pending/zed-graphql-schema.md`. Tree-sitter language for SDL. After change 1. Blocked on the GraphQL extension's language name, grammar id, and suffixes.

## Local iteration

VS Code: vscode-extension.md.

Zed: Install Dev Extension on `zed-extension/`. `zed --foreground` for wasm logs. Restart the language server after a Rust binary rebuild (`editor: restart language server`). The wasm extension itself reloads when Zed rebuilds it on install; re-install the dev extension after `src/lib.rs` changes.
