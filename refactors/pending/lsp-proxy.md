# `isograph lsp` stdio proxy

Requires lsp-port.md (landed). Independent of lsp-tokens.md, lsp-sessions.md, lsp-diagnostics.md. The daemon port is already LSP JSON-RPC. This verb is a byte copy, not a second handshake.

VS Code and Zed spawn a process on stdio. They do not dial `{slug}.port`. `isograph lsp` is that process: the same walk-up / `--config` as every verb, start the daemon if needed, dial the LSP port, copy stdin/stdout. Dropping the editor drops the proxy. The daemon stays up.

Origin: `docs-website/docs/design-docs/event-model.md` (`isograph lsp` as stdio proxy). Origin of spawn args: `vscode-extension/src/languageClient.ts` `['lsp']` plus optional `--config`. Delta: the binary actually has the verb; the daemon is already the LSP server.

One shippable change.

## What the user does

```
$ isograph lsp
```

With a config at or above cwd, the daemon is running (or this process starts it the way `isograph start` does), and stdio is LSP. VS Code with `isograph.pathToIsograph` pointed at the cargo binary starts highlighting once lsp-tokens.md has landed. Zed is the same after `semantic_tokens` is `combined` or `full`.

`--config` is `ConfigFlag`, same as send.

## Types

```rust
// from crates/isograph_cli/src/lib.rs
    /// Speak LSP on stdio with the daemon for this config.
    Lsp(ConfigFlag),
```

Not hidden. In `--help`.

```rust
// from crates/isograph_cli/src/lsp_stdio.rs
pub fn run(id: &crate::ConfigFlag) -> std::process::ExitCode {
    // discover, start daemon if not running (same as `isograph start` if Held::Free),
    // poll port file with the same deadline style as tests (10s),
    // TcpStream::connect 127.0.0.1:port,
    // copy stdin -> socket and socket -> stdout until either side EOF,
    // then exit 0.
}
```

Do not parse LSP in the proxy. Byte copy. `initialize` is the editor’s. Send remains a separate client.

Starting the daemon: `isograph start` from the proxy is a nested process, or call the same `freddie_cli` start path. Nested `Command::new(current_exe()).args(["start"])` with the same `--config` / `HOME`. If already running, skip. Then connect.

`processId` in the editor’s `initialize` is the editor, not the proxy. We still do not watch `processId` on the daemon (lsp-sessions.md). When the editor dies, stdio EOF, proxy exits, TCP closes, session `Drop`.

## Tests

`cli.rs`: `lsp_is_in_help`. `lsp_with_the_daemon_stopped_starts_it`: HOME isolation, `isograph lsp` in a thread or subprocess with a pipe; write initialize on stdin; read a response with `capabilities`; kill the child; `isograph status` still running until `stop`. Deadline 10s.

Do not bring up VS Code.

## Call sites

- VS Code / Zed -> `isograph lsp` -> `{slug}.port` -> `accept_loop`
- `isograph send` does not use this verb
