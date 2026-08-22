# Event model: landing sequence

Architecture is `docs-website/docs/design-docs/event-model.md`. This file is the order implementation docs land.

Requires config-discovery.md (landed) and the design-doc.

1. event-loop.md (landed). tokio current-thread runtime. `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`. SIGTERM sends `Quit`. `run_event_loop`, `run_effect_loop`.
2. send-events.md (landed). Event socket on `127.0.0.1:0`, port file next to the lock, `IsographEvent` serde JSON, `isograph send`.
3. config-path.md. `isograph config-path` prints the canonical config path.
4. filesystem-events.md, after send-events.md. `IsographState` is the pico database. `DiskChanged` with `Presence` interns or removes `DiskFile`. Files arrive through `isograph send`.
5. lsp-semantic-token-encoding.md. Encoder only. Landed.
6. LSP adapter. Not written. `EditorChanged`, `isograph lsp` proxy, `OpenFile`.
7. lsp-parse-diagnostics.md, against the adapter.
8. zed-and-vscode-extensions.md.

`AsyncWorkFinished` and `StartAsyncWork` land with compilation.
