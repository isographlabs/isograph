# Event model: landing sequence

Architecture is `docs-website/docs/design-docs/event-model.md`. This file is the order implementation docs land.

Requires config-discovery.md (landed), the event-model design-doc, and `docs-website/docs/design-docs/pico.md`.

1. event-loop.md (landed). tokio current-thread runtime. `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`. SIGTERM sends `Quit`. `run_event_loop`, `run_effect_loop`.
2. send-events.md (landed). Event socket on `127.0.0.1:0`, port file next to the lock, `IsographEvent` serde JSON, `isograph send`.
3. config-path.md (landed). `isograph config-path` prints the canonical config path.
4. filesystem-events.md (landed). `IsographState` is the pico database. `DiskChanged` with `Presence` interns or removes `DiskFile`. Files arrive through `isograph send`.
5. config-source-files.md (landed). Config field `source_files`: a `Vec` of glob strings.
6. filesystem-watcher.md. Later. After 4 and 5. OS watcher posts `DiskChanged`. `isograph start --filesystem watch|injected`. Default `Watch`.
7. extract-iso-literals-from-file.md (landed). After 4. Extract does not parse. pico memo `HostLanguage::extract_iso_literals` (file, the whole vec), `#[memo]` on the TypeScript impl like isograph `CompilationProfile`. `IsoLiteralExtraction` with text, context, and byte start. `IsographState<THostLanguage>` lives in `isograph_compiler`.
8. memoized-parse-iso-literal.md (landed). After 7. pico memo `parsed_iso_literal` keyed on the literal text. Host embedding errors after parse. `file_literals` on `db` + `path`.
9. literal-id.md (landed). After 7 and 8. pico memo `literal_id_at_location` (file + `LineChar`) stores `LiteralId` (path plus 0-based extract index). pico memo `iso_literal_extraction` is keyed on `LiteralId`. No parse tree or literal string stored at `(path, LineChar)`.
10. file-semantic-tokens.md. After 8, 9, and lsp-semantic-token-encoding.md. Offset parse tokens to file coordinates, concatenate, encode.
11. lsp-semantic-token-encoding.md. Encoder only. Landed.
12. e2e-semantic-tokens.md. After 10. Hidden `isograph semantic-tokens`. Start, send `Present`, query encoded tokens. Append does not change the JSON. Prepend shifts `delta_line`.
13. LSP adapter. Not written. `EditorChanged`, `isograph lsp` proxy, `OpenFile`.
14. lsp-parse-diagnostics.md, against the adapter and `file_literals` from 8.
15. zed-and-vscode-extensions.md.

`AsyncWorkFinished` and `StartAsyncWork` land with compilation.
