# Event model: landing sequence

Types, dispatch, and process layout are `docs-website/docs/design-docs/event-model.md`. This file is the order the implementation slices land. It does not define types.

Requires config-discovery.md (landed) and the design-doc.

## Order

1. freddie `refactors/pending/event-socket-local-addr.md`. `EventSocket::local_addr() -> SocketAddr` so `listen(0)` is usable. Pin-rev in i2 after it lands.
2. filesystem-events.md. `DiskChanged`, `IncomingEvent.DiskChanged`, event loop, `isograph send`, config `includes`, watcher. State is a disk map. `handle` returns `()`. `IncomingEvent` in this slice has only `DiskChanged`.
3. pico intern of `DiskFile`, replacing the map. Same `DiskChanged`. No dedicated pending doc yet.
4. lsp-semantic-token-encoding.md. Encoder only. No server. Can overlap (2).
5. lsp-semantic-tokens.md changes 1–2: `file_literals`, legend. Not change 3's standalone stdio `isograph lsp` loop. That loop is not the process in the design-doc. The adapter in the daemon calls `file_literals` and the encoder.
6. LSP adapter (not written). `EditorChanged`, `IncomingEvent.EditorChanged`, `{slug}.lsp`, `isograph lsp` as the stdio proxy, `OpenFile`. Adapter is request/response; `handle` is not. `ReportDiagnostics` / `WriteArtifacts` land when they have a performer.
7. lsp-parse-diagnostics.md, retargeted at the adapter rather than the standalone server.
8. zed-and-vscode-extensions.md. Both editors spawn `isograph lsp`.

`AsyncWorkFinished`, `StartAsyncWork`, `Quit` / `Kill` land with compilation, not with (2).
