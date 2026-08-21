# Event model: landing sequence

Types, dispatch, and process layout are `docs-website/docs/design-docs/event-model.md`. This file is the order the implementation slices land. It does not define types.

Requires config-discovery.md (landed) and the design-doc.

The first section is filesystem events and a CLI that writes them. That is filesystem-events.md, in this order:

1. Event loop, `handle`, socket, one event: this path has these contents. freddie `event-socket-local-addr.md` lands first so `listen(0)` reports the port.
2. `isograph send`.
3. Config `includes`.
4. Created, deleted, moved: `Presence` (`Present` / `Absent`). A move is two events.
5. Watcher. `--filesystem watch|injected`.

Later, not this section:

- pico intern of `DiskFile`, replacing the map. Same `DiskChanged`. No dedicated pending doc yet.
- lsp-semantic-token-encoding.md. Encoder only. Can overlap the first section.
- lsp-semantic-tokens.md changes 1–2: `file_literals`, legend. Not change 3's standalone stdio loop.
- LSP adapter (not written). `EditorChanged`, `isograph lsp` proxy, `OpenFile`.
- lsp-parse-diagnostics.md, retargeted at the adapter.
- zed-and-vscode-extensions.md.

`AsyncWorkFinished`, `StartAsyncWork`, `Quit` / `Kill` land with compilation, not with the first section.
