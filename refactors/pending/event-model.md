# Event model: landing sequence

Architecture is `docs-website/docs/design-docs/event-model.md`. This file is the order implementation docs land.

Requires config-discovery.md (landed), the event-model design-doc, and `docs-website/docs/design-docs/pico.md`.

1. event-loop.md (landed). tokio current-thread runtime. `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`. SIGTERM sends `Quit`. `run_event_loop`, `run_effect_loop`.
2. send-events.md (landed). Event socket on `127.0.0.1:0`, port file next to the lock, `IsographEvent` serde JSON, `isograph send`. Wire replaced by lsp-port.md.
3. config-path.md (landed). `isograph config-path` prints the canonical config path.
4. filesystem-events.md (landed). `IsographState` is the pico database. `DiskChanged` with `Presence` interns or removes `DiskFile`. Files arrive through `isograph send`.
5. config-source-files.md (landed). Config field `source_files`: a `Vec` of glob strings.
6. filesystem-watcher.md. Later. After 4 and 5. Two changes in that file: `DiskChanged` file vs folder and `remove_disk_files_from_path`, then the OS watcher. `isograph start --filesystem watch|injected`. Default `Watch`.
7. extract-iso-literals-from-file.md (landed). After 4. Extract does not parse. pico memo `HostLanguage::extract_iso_literals` (file, the whole vec), `#[memo]` on the TypeScript impl like isograph `CompilationProfile`. `IsoLiteralExtraction` with text, context, and byte start. `IsographState<THostLanguage>` lives in `isograph_compiler`.
8. memoized-parse-iso-literal.md (landed). After 7. pico memo `parsed_iso_literal` keyed on the literal text. Host embedding errors after parse. `file_literals` on `db` + `path`.
9. literal-id.md (landed). After 7 and 8. pico memo `literal_id_at_location` (file + `LineChar`) stores `LiteralId` (path plus 0-based extract index). pico memo `iso_literal_extraction` is keyed on `LiteralId`. No parse tree or literal string stored at `(path, LineChar)`.
10. file-semantic-tokens.md (landed). Path to encoded tokens. Interned-file tests. Not the daemon.
11. lsp-semantic-token-encoding.md (landed). Encoder only.
12. lsp-port.md (landed). The `{slug}.port` TCP listener is LSP. Each TCP connection is one client and one `session`. `isograph/event` params are the `--file` JSON; the session deserializes them and posts that `IsographEvent`. `handle` is unchanged. `isograph send` does the handshake then that notification. Requests other than initialize are `MethodNotFound` on that connection. `Kill` unlinks the port file then `process::exit(0)`.
13. lsp-request-response.md (landed). Session posts `IsographEvent::LspRequest` (request, `connection.sender` clone). `handle` returns `SendLspResponse`. Effect loop writes the `Response`. No client id. No outstanding set.
14. lsp-dispatch.md. Later. isograph `LSPRequestDispatch` / `LSPNotificationDispatch` / `LSPRuntimeError` copied into `isograph_lsp`. Session posts `LspRequest` / `LspNotification` and does not interpret methods after initialize. Request handler is `method_not_found` returning `Vec<IsographEffect>` (one immediate `SendLspResponse`). Notification chain is `isograph/event`. Continue is no effects.
15. lsp-tokens.md. Later. `.on_request_sync::<SemanticTokensFullRequest>`. Advertise the legend on `initialize`. Independent of sessions.
16. lsp-outstanding.md. Later. `LspClientId`, `LspClientGone`, outstanding `(LspClientId, RequestId)`. For disconnect/cancel/async, not for highlighting.
17. lsp-sessions.md. Later. `ClientCapabilities` from `initialize` and a writer map for `publishDiagnostics`.
18. lsp-diagnostics.md. Later. Debounce then `publishDiagnostics`. Requires 17.
19. lsp-proxy.md. Later. `isograph lsp` stdio copy onto the port.
20. zed-and-vscode-extensions.md.

`AsyncWorkFinished` and `StartAsyncWork` land with compilation.
