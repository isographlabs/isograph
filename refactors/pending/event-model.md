# Event model: landing sequence

Architecture is `docs-website/docs/design-docs/event-model.md`. This file is the order implementation docs land.

Requires config-discovery.md (landed) and the design-doc.

1. event-loop.md (landed). tokio current-thread runtime. `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`. SIGTERM sends `Quit`. `run_event_loop`, `run_effect_loop`.
2. send-events.md (landed). Event socket on `127.0.0.1:0`, port file next to the lock, `IsographEvent` serde JSON, `isograph send`.
3. config-path.md (landed). `isograph config-path` prints the canonical config path.
4. filesystem-events.md (landed). `IsographState` is the pico database. `DiskChanged` with `Presence` interns or removes `DiskFile`. Files arrive through `isograph send`.
4a. disk-changed-absent-and-path-keys.md. After 4. `Absent` of a missing path does not bump the tracked map. Relative path keys are stored as given.
5. config-source-files.md. Config field `source_files`: a `Vec` of glob strings.
6. filesystem-watcher.md. After 4 and 5. OS watcher posts `DiskChanged`. `isograph start --filesystem watch|injected`. Default `Watch`.
7. extract-iso-literals-from-file.md. After 4a. Extract does not parse. pico memo `extract_iso_literals_from_file_content`. `IsoLiteralExtraction` with text, context, byte start, and index. Move `IsographState` to `isograph_compiler`.
8. memoized-parse-iso-literal.md. After 7. pico memo `parsed_iso_literal` keyed on literal text. `parsed_iso_literal_in_file` takes the extraction index. Host embedding errors after parse.
9. file-semantic-tokens.md. After 8 and lsp-semantic-token-encoding.md. Offset parse tokens to file coordinates, concatenate, encode.
10. lsp-semantic-token-encoding.md. Encoder only. Landed.
11. LSP adapter. Not written. `EditorChanged`, `isograph lsp` proxy, `OpenFile`.
12. lsp-parse-diagnostics.md, against the adapter and `file_literals` from 8.
13. zed-and-vscode-extensions.md.

`AsyncWorkFinished` and `StartAsyncWork` land with compilation.
