# Event model: landing sequence

Architecture is `docs-website/docs/design-docs/event-model.md`. This file is the order implementation docs land.

Requires config-discovery.md (landed) and the design-doc.

1. event-loop.md. tokio current-thread runtime. `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`. SIGTERM sends `Quit`. `run_event_loop`, `run_effect_loop`.
2. filesystem-events.md, after event-loop.md. CLI send, config `includes`, `Presence`, watcher.
3. pico intern of `DiskFile`. Same `DiskChanged`. No pending doc yet.
4. lsp-semantic-token-encoding.md. Encoder only. May overlap (2).
5. lsp-semantic-tokens.md changes 1–2: `file_literals`, legend.
6. LSP adapter. Not written. `EditorChanged`, `isograph lsp` proxy, `OpenFile`.
7. lsp-parse-diagnostics.md, against the adapter.
8. zed-and-vscode-extensions.md.

`AsyncWorkFinished` and `StartAsyncWork` land with compilation.
